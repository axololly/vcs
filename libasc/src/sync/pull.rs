use std::{collections::{HashMap, HashSet, VecDeque}, hash::{self, DefaultHasher, Hasher}};

use eyre::{Result, bail, eyre};
use rateless_tables::{Decoder, Encoder, Symbol};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt as Read, AsyncWriteExt as Write};

use crate::{action::Action, content::Content, graph::Graph, hash::{ObjectHash, RawObjectHash}, repository::{NamedHashes, Repository}, snapshot::Snapshot, sync::{stream::Stream, utils::{dfs_get, handle_login, login_as, Object, Repo, SendState, DONE, PENDING}}, unwrap, user::User};

pub async fn client_fetch_objects(
    stream: &mut impl Stream,
    repo: &Repository
) -> Result<HashMap<ObjectHash, Object>>
{
    let mut queue: VecDeque<ObjectHash> = VecDeque::new();

    let mut snapshots_to_resolve: HashSet<ObjectHash> = HashSet::new();

    for snapshot_hash in repo.history.iter_hashes() {
        let Ok(snapshot) = repo.fetch_snapshot(snapshot_hash) else {
            queue.push_back(snapshot_hash);
            
            snapshots_to_resolve.insert(snapshot_hash);

            continue;
        };

        for content_hash in snapshot.files.into_values() {
            if repo.fetch_content_object(content_hash).is_err() {
                queue.push_back(snapshot_hash);
            }
        }
    }

    let mut contents: HashMap<ObjectHash, Object> = HashMap::new();

    while let Some(next) = queue.pop_front() {
        stream.send(&PENDING).await?;

        stream.send(&next).await?;
        
        let raw_object: Result<Object, String> = stream.receive().await?;

        let content = raw_object
            .map_err(|message| eyre!("server error: {message}"))?;

        if let Object::Commit(snapshot) = &content
            && snapshots_to_resolve.contains(&snapshot.hash)
        {
            queue.extend(snapshot.files.values().cloned());
        }

        if let Object::Content(Content::Delta(delta)) = &content
            && repo.fetch_string_content(delta.original).is_err()
        {
            queue.push_back(delta.original);
        }

        contents.insert(next, content);
    }

    stream.send(&DONE).await?;

    Ok(contents)
}

pub async fn server_serve_objects(
    stream: &mut impl Stream,
    repo: &Repository
) -> Result<()>
{
    loop {
        let state: SendState<()> = stream.receive().await?;

        if state == DONE {
            break;
        }

        let hash: ObjectHash = stream.receive().await?;

        let result = if repo.history.contains(hash) {
            repo.fetch_snapshot(hash)
                .map(Box::new)
                .map(Object::Commit)
        }
        else {
            repo.fetch_content_object(hash)
                .map(Object::Content)
        };

        if let Err(e) = &result {
            let error: Result<(), String> = Err(e.to_string());

            stream.send(&error).await?;

            return result.map(|_| ());
        }

        let reply: Result<Object, ()> = result.map_err(|_| ());

        stream.send(&reply).await?;
    }

    Ok(())
}

pub enum BranchPullResult {
    NotOnRemote,
    UpToDate,
    FastForward(Graph, ObjectHash),
    Conflict(Graph, ObjectHash, ObjectHash)
}

pub enum TagPullResult {
    Conflict(ObjectHash, ObjectHash),
    New(ObjectHash)
}

pub enum PullResult {
    Branch(String, BranchPullResult),
    Tag(String, TagPullResult)
}

pub async fn client_pull_one_branch(
    stream: &mut impl Stream,
    repo: &Repository,
    branch: &str
) -> Result<BranchPullResult>
{
    let local_tip = repo.branches.get(branch).unwrap();

    stream.send(&(branch, local_tip)).await?;

    let remote_tip_if_any: Option<ObjectHash> = stream.receive().await?;

    let Some(remote_tip) = remote_tip_if_any else {
        return Ok(BranchPullResult::NotOnRemote);
    };

    if local_tip == remote_tip {
        return Ok(BranchPullResult::UpToDate);
    }

    let mut branch = Graph::new();

    dfs_get(&repo.history, local_tip, &mut branch);

    let mut enc = Encoder::default();

    enc.extend(branch.iter_hashes());

    let (changes, remote_tip) = loop {
        let symbol = enc.next().unwrap();
        
        stream.send(&symbol).await?;

        let reply: SendState<(Graph, ObjectHash)> = stream.receive().await?;

        if let SendState::Done(value) = reply {
            break value;
        }
    };

    branch.extend(&changes);

    if branch.is_descendant(remote_tip, local_tip)? {
        Ok(BranchPullResult::FastForward(branch, remote_tip))
    }
    else {
        Ok(BranchPullResult::Conflict(branch, local_tip, remote_tip))
    }
}

pub async fn handle_pull_as_client(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<Vec<PullResult>>
{
    let mut repo = repo.lock().await;

    let user = unwrap!(
        repo.current_user(),
        "no valid user on this repository"
    );
    
    login_as(user, stream, repo.project_code).await?;

    let branch_names: Vec<_> = repo.branches
        .iter()
        .map(|(name, _)| name.clone())
        .collect();

    let mut pull_results: Vec<PullResult> = vec![];

    for name in branch_names {
        stream.send(&PENDING).await?;
    
        let result = client_pull_one_branch(stream, &repo, &name).await?;
        
        match &result {
            BranchPullResult::NotOnRemote => {},
            BranchPullResult::UpToDate => {},

            BranchPullResult::FastForward(graph, remote_tip) => {
                repo.history.extend(graph);

                let old = repo.branches.get(&name).unwrap();
                
                repo.branches.create(name.clone(), *remote_tip);

                repo.action_history.push(
                    Action::MoveBranch {
                        name: name.clone(),
                        old,
                        new: *remote_tip
                    }
                );
            }

            BranchPullResult::Conflict(graph, local_tip, remote_tip) => {
                repo.history.extend(graph);

                repo.branches.create(format!("{name}-local"), *local_tip);
                
                repo.branches.create(name.clone(), *remote_tip);

                repo.action_history.push(
                    Action::MoveBranch {
                        name: name.clone(),
                        old: *local_tip,
                        new: *remote_tip
                    }
                );

                repo.action_history.push(
                    Action::CreateBranch {
                        name: format!("{name}-local"),
                        hash: *local_tip
                    }
                );
            }
        }

        pull_results.push(PullResult::Branch(name, result));
    }

    stream.send(&DONE).await?;

    stream.send(&repo.tags).await?;

    let new_tags: NamedHashes = stream.receive().await?;

    for (name, server_hash) in new_tags.into_iter() {
        let tag_result = match repo.tags.get(&name) {
            Some(client_hash) if client_hash != server_hash => {
                repo.tags.rename(&name, format!("{name}-local"));
                
                repo.action_history.push(
                    Action::RenameTag {
                        old: name.to_string(),
                        new: format!("{name}-local"),
                        hash: client_hash
                    }
                );

                repo.tags.create(name.clone(), server_hash);

                repo.action_history.push(
                    Action::CreateTag {
                        name: name.to_string(),
                        hash: client_hash
                    }
                );

                TagPullResult::Conflict(client_hash, server_hash)
            },
            
            None => {
                repo.tags.create(name.to_string(), server_hash);

                repo.action_history.push(
                    Action::CreateTag {
                        name: name.to_string(),
                        hash: server_hash
                    }
                );

                TagPullResult::New(server_hash)
            },

            _ => continue
        };

        pull_results.push(PullResult::Tag(name, tag_result));
    }

    let mut new_objects = client_fetch_objects(stream, &repo).await?;

    for (hash, object) in new_objects {
        match object {
            Object::Commit(snapshot) => repo.save_snapshot(*snapshot)?,
            Object::Content(content) => repo.save_content_object(content, hash)?
        }
    }
    
    Ok(pull_results)
}

pub async fn handle_pull_as_server(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()> {
    let mut repo = repo.lock().await;

    let check = |user: &User| {
        user.permissions
            .can_pull()
            .then_some(())
            .ok_or(format!("user {:?} does not have permission to pull", user.name))
    };

    handle_login(&repo, stream, check).await?;

    loop {
        let do_branches: SendState<()> = stream.receive().await?;

        if do_branches == DONE {
            break;
        }

        let (branch_name, client_tip): (String, ObjectHash) = stream.receive().await?;
    
        let Some(server_tip) = repo.branches.get(&branch_name) else {
            stream.send(&None::<()>).await?;

            continue;
        };

        stream.send(&Some(server_tip)).await?;

        if client_tip == server_tip {
            continue;
        }

        let mut branch = Graph::new();

        dfs_get(&repo.history, server_tip, &mut branch);

        let mut dec = Decoder::default();

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

        let (_, changes) = dec.consume();

        let mut diff = Graph::new();

        for hash in changes {
            let parents = branch.get_parents(hash).unwrap();

            if parents.is_empty() {
                diff.insert_orphan(hash);
            }

            for &parent in parents {
                diff.insert(hash, parent);
            }
        }

        let done: SendState<_> = SendState::Done((diff, server_tip));

        stream.send(&done).await?;
    }

    let client_tags: NamedHashes = stream.receive().await?;

    let mut new_tags = NamedHashes::new();

    for (name, &server_hash) in repo.tags.iter() {
        if let Some(client_hash) = client_tags.get(name)
            && client_hash == server_hash
        {
            continue;
        }
        
        new_tags.create(name.to_string(), server_hash);
    }

    stream.send(&new_tags).await?;

    server_serve_objects(stream, &repo).await?;

    Ok(())
}
