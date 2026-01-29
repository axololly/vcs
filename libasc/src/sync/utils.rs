use std::{collections::VecDeque, hash::{DefaultHasher, Hasher}, sync::Arc};

use eyre::{Result, bail, eyre};
use rand::random;
use rateless_tables::Symbol;
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt as Read, AsyncWriteExt as Write}, sync::Mutex};

use crate::{content::Content, graph::Graph, hash::ObjectHash, key::Signature, repository::Repository, snapshot::Snapshot, sync::stream::Stream, user::User};

pub type Repo = Arc<Mutex<Repository>>;

pub type ServerSecret = [u8; 32];

pub async fn login_as(
    user: &User,
    stream: &mut impl Stream,
    project_code: ObjectHash
) -> Result<()> {
    stream.send(&project_code).await?;

    let get_secret: Result<ServerSecret, ()> = stream.receive().await?;

    let Ok(secret) = get_secret else {
        bail!("project codes do not match");
    };

    let mut key = user.private_key.clone().unwrap();

    let auth = key.sign(&secret);

    stream.send(&auth).await?;

    let result: Result<(), String> = stream.receive().await?;

    result.map_err(|message| eyre!("server error: {message}"))
}

pub async fn handle_login(
    repo: &Repository,
    stream: &mut impl Stream,
    validate_user: impl FnOnce(&User) -> Result<(), String>
) -> Result<()>
{
    let client_project_code: ObjectHash = stream.receive().await?;

    if repo.project_code != client_project_code {
        let error: Result<ServerSecret, ()> = Err(());

        return stream.send(&error).await;
    }

    let secret: Result<ServerSecret, ()> = {
        let mut buf = [0; 32];

        let mut iter = std::iter::repeat_with(random::<u8>);

        for (i, n) in iter.enumerate().take(32) {
            buf[i] = n;
        }

        Ok(buf)
    };

    stream.send(&secret).await?;

    let login: Signature = stream.receive().await?;

    let result: Result<(), String> = if login.verify(&secret.unwrap()) {
        match repo.users.get_user_by_pub_key(login.key()) {
            Some(user) => validate_user(user),
            None => Err("user does not exist".to_string())
        }
    }
    else {
        Err("failed to verify signature".to_string())
    };

    stream.send(&result).await?;

    Ok(())
}

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
pub enum SendState<T> {
    // "That's enough streaming. Here is ..."
    Done(T),

    // "I'm not done, keep streaming"
    Pending
}

pub const PENDING: SendState<()> = SendState::Pending;
pub const DONE: SendState<()> = SendState::Done(());

#[derive(Deserialize, Serialize)]
pub enum Object {
    Commit(Box<Snapshot>),
    Content(Content)
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BranchResponse {
    HasBranch(ObjectHash),
    DoesntHaveBranch
}
