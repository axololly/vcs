use std::{collections::{HashMap, HashSet, VecDeque}, hash::{self, DefaultHasher, Hasher}};

use eyre::{Result, bail, eyre};
use rateless_tables::{Decoder, Encoder, Symbol};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt as Read, AsyncWriteExt as Write};

use crate::{action::Action, content::Content, graph::Graph, hash::{ObjectHash, RawObjectHash}, repository::{NamedHashes, Repository}, sync::{stream::Stream, utils::{Repo, handle_login, login_as}}, unwrap, user::User};

pub fn dfs_get(graph: &Graph, start: ObjectHash, chain: &mut Graph) {
    let parents = graph.get_parents(start).unwrap();

    if parents.is_empty() {
        chain.insert_orphan(start);
    }

    for &parent in parents {
        chain.insert(start, parent);

        dfs_get(graph, parent, chain);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum SendState<T> {
    // "That's enough streaming. Here is ..."
    Done(T),

    // "I'm not done, keep streaming"
    Pending
}

const PENDING: SendState<()> = SendState::Pending;
const DONE: SendState<()> = SendState::Done(());

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

#[derive(Debug, Deserialize, Serialize)]
enum BranchResponse {
    HasBranch(ObjectHash),
    DoesntHaveBranch
}

pub async fn client_pull_one_branch(
    stream: &mut impl Stream,
    repo: &Repository,
    branch: &str
) -> Result<BranchPullResult>
{
    let local_tip = repo.branches.get(branch).unwrap();

    stream.send(&(branch, local_tip)).await?;

    let branch_response: BranchResponse = stream.receive().await?;

    match branch_response {
        BranchResponse::HasBranch(remote_tip) => {
            println!("comparing tips of {branch}: {local_tip} vs {remote_tip}");
            
            if local_tip == remote_tip {
                return Ok(BranchPullResult::UpToDate);
            }
        }
        
        _ => return Ok(BranchPullResult::NotOnRemote)
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
        
        println!("pulling branch {name}");
    
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
                println!("conflict on branch {name} - dividing branches");

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

    let server_tags: NamedHashes = stream.receive().await?;

    for (name, server_hash) in server_tags.into_iter() {
        let tag_result = match repo.tags.get(&name) {
            Some(hash) if hash != server_hash => {
                TagPullResult::Conflict(hash, server_hash)
            },
            
            None => {
                TagPullResult::New(server_hash)
            },

            _ => continue
        };

        pull_results.push(PullResult::Tag(name, tag_result));
    }
    
    Ok(pull_results)
}

pub async fn handle_pull_as_server(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()> {
    let mut repo = repo.lock().await;

    let check = |user: &User| {
        if user.permissions.can_pull() {
            Ok(())
        }
        else {
            Err("user does not have permission".to_string())
        }
    };

    handle_login(&repo, stream, check).await?;

    let do_branches: SendState<()> = stream.receive().await?;

    loop {
        if do_branches == DONE {
            break;
        }

        let (branch_name, client_tip): (String, ObjectHash) = stream.receive().await?;
    
        let Some(server_tip) = repo.branches.get(&branch_name) else {
            stream.send(&BranchResponse::DoesntHaveBranch).await?;

            continue
        };

        stream.send(&BranchResponse::HasBranch(server_tip)).await?;

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

        let is_client_done: SendState<()> = stream.receive().await?;

        if is_client_done == DONE {
            break;
        }
    }

    stream.send(&repo.tags).await?;

    let stop: u8 = stream.receive().await?;
    Ok(())
}
