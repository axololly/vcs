use std::collections::HashMap;

use eyre::Result;
use rateless_tables::{Decoder, Encoder};
use serde::{Deserialize, Serialize};

use crate::{action::Action, graph::Graph, hash::ObjectHash, repository::{NamedItems, Repository}, sync::{stream::Stream, utils::{dfs_get, handle_login, login_as, Object, Repo, SendState, DONE, PENDING}}, unwrap, user::User};

pub enum BranchPushResult {
    CreatedOnRemote,
    UpToDate,
    FastForward(ObjectHash, ObjectHash),
    SplitHistory
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TagPushResult {
    Conflict,
    CreatedOnRemote
}

pub enum PushResult {
    Branch(String, BranchPushResult),
    Tag(String, TagPushResult)
}

pub async fn client_push_one_branch(
    stream: &mut impl Stream,
    repo: &Repository,
    branch: &str
) -> Result<BranchPushResult>
{
    let local_tip = *repo.branches.get(branch).unwrap();

    stream.send(&(branch, local_tip)).await?;

    let remote_tip_if_any: Option<ObjectHash> = stream.receive().await?;

    let mut branch = Graph::new();

    dfs_get(&repo.history, local_tip, &mut branch);

    if let Some(remote_tip) = remote_tip_if_any {
        if local_tip == remote_tip {
            stream.send(&DONE).await?;

            return Ok(BranchPushResult::UpToDate);
        }

        // Unrelated histories
        if !branch.is_descendant(local_tip, remote_tip)? {
            stream.send(&DONE).await?;

            return Ok(BranchPushResult::SplitHistory);
        }
    }

    stream.send(&PENDING).await?;

    let mut enc = Encoder::default();

    enc.extend(branch.iter_hashes());

    for symbol in enc {
        stream.send(&symbol).await?;

        let state: SendState<()> = stream.receive().await?;

        if state == DONE {
            break;
        }
    }

    let requested: Vec<ObjectHash> = stream.receive().await?;

    let mut objects: HashMap<ObjectHash, Object> = HashMap::new();

    for hash in requested {
        let object = if repo.history.contains(hash) {
            let snapshot = repo.fetch_snapshot(hash)?;

            for &content_hash in snapshot.files.values() {
                let content = repo.fetch_content_object(content_hash)?;

                objects.insert(content_hash, Object::Content(content));
            }

            Ok(Object::Commit(Box::new(snapshot)))
        }
        else {
            repo.fetch_content_object(hash)
                .map(Object::Content)
        }?;

        objects.insert(hash, object);
    }

    stream.send(&objects).await?;
    
    let result = if let Some(remote_tip) = remote_tip_if_any {
        BranchPushResult::FastForward(remote_tip, local_tip)
    }
    else {
        BranchPushResult::CreatedOnRemote
    };

    Ok(result)
    
}

pub async fn handle_push_as_client(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<Vec<PushResult>>
{
    let repo = repo.lock().await;

    let user = unwrap!(
        repo.current_user(),
        "no valid user set for this repository."
    );
    
    login_as(user, stream, repo.project_code).await?;

    let mut results: Vec<PushResult> = vec![];

    for branch in repo.branches.names() {
        stream.send(&PENDING).await?;

        let branch_result = client_push_one_branch(stream, &repo, branch).await?;

        results.push(PushResult::Branch(branch.to_string(), branch_result));
    }

    stream.send(&DONE).await?;

    stream.send(&repo.tags).await?;

    let tag_results: HashMap<String, TagPushResult> = stream.receive().await?;

    for (name, tag_result) in tag_results {
        results.push(PushResult::Tag(name, tag_result));
    }

    let missing_on_server: Vec<ObjectHash> = stream.receive().await?;

    let mut requested_objects = HashMap::new();

    for hash in missing_on_server {
        let snapshot = repo.fetch_snapshot(hash)?;

        for &content_hash in snapshot.files.values() {
            let content = repo.fetch_content_object(content_hash)?;

            requested_objects.insert(hash, Object::Content(content));
        }

        requested_objects.insert(hash, Object::Commit(Box::new(snapshot)));
    }

    stream.send(&requested_objects).await?;
    
    Ok(results)
}

pub async fn handle_push_as_server(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()>
{
    let mut repo = repo.lock().await;

    // TODO: implement hooks
    let check = |_: &User| Ok(());

    handle_login(&repo, stream, check).await?;

    loop {
        let state: SendState<()> = stream.receive().await?;

        if state == DONE {
            break;
        }

        let (branch_name, client_tip): (String, ObjectHash) = stream.receive().await?;

        let server_tip_if_any = repo.branches.get(&branch_name);

        stream.send(&server_tip_if_any).await?;

        let state: SendState<()> = stream.receive().await?;

        if state == DONE {
            continue;
        }

        let mut branch = Graph::new();

        let mut dec = Decoder::default();

        if let Some(&server_tip) = server_tip_if_any {
            dfs_get(&repo.history, server_tip, &mut branch);
        }

        dec.extend(branch.iter_hashes());

        loop {
            let symbol = stream.receive().await?;

            dec.add_coded_symbol(symbol);

            dec.decode();

            if dec.is_done() {
                break;
            }

            stream.send(&PENDING).await?;
        }

        stream.send(&DONE).await?;

        let (changes, _) = dec.consume();

        stream.send(&changes).await?;

        let requested: HashMap<ObjectHash, Object> = stream.receive().await?;

        for (hash, object) in requested {
            match object {
                Object::Commit(snapshot) => repo.save_snapshot(*snapshot)?,
                Object::Content(content) => repo.save_content_object(content, hash)?
            }
        }

        let previous = repo.branches.create(branch_name.clone(), client_tip);

        let action = if let Some(old) = previous {
            if old == repo.current_hash {
                repo.current_hash = client_tip;
            }
            
            Action::MoveBranch {
                name: branch_name,
                old,
                new: client_tip
            }
        }
        else {
            Action::CreateBranch {
                name: branch_name,
                hash: client_tip
            }
        };
        
        repo.action_history.push(action);
    }

    let client_tags: NamedItems<ObjectHash> = stream.receive().await?;

    let mut tag_results: HashMap<String, TagPushResult> = HashMap::new();

    let mut needed_snapshots = Vec::new();

    for (name, client_hash) in client_tags.into_iter() {
        let Some(&server_hash) = repo.tags.get(&name) else {
            needed_snapshots.push(client_hash);

            repo.tags.create(name.to_string(), client_hash);

            tag_results.insert(name, TagPushResult::CreatedOnRemote);

            continue;
        };

        if client_hash == server_hash {
            continue;
        }

        tag_results.insert(name, TagPushResult::Conflict);
    }

    stream.send(&tag_results).await?;

    stream.send(&needed_snapshots).await?;

    let tag_objects: HashMap<ObjectHash, Object> = stream.receive().await?;

    for (hash, object) in tag_objects {
        match object {
            Object::Commit(snapshot) => repo.save_snapshot(*snapshot)?,
            Object::Content(content) => repo.save_content_object(content, hash)?
        }
    }

    repo.save()?;

    Ok(())
}
