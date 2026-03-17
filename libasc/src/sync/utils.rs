use std::sync::Arc;

use eyre::{Result, bail};
use rand::random;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{content::Content, graph::Graph, hash::ObjectHash, key::{PublicKey, Signature}, repository::Repository, snapshot::Snapshot, sync::stream::Stream, unwrap, user::{User, Users}};

pub type Repo = Arc<Mutex<Repository>>;

pub type ServerSecret = [u8; 32];

pub fn get_server_secret() -> ServerSecret {
    let mut buf = [0; 32];

    let iter = std::iter::repeat_with(random::<u8>);

    for (i, n) in iter.enumerate().take(32) {
        buf[i] = n;
    }

    buf
}

pub async fn login_as(
    user_key: PublicKey,
    stream: &mut impl Stream,
    project_code: ObjectHash,
    repo_users: &mut Users
) -> Result<()> {
    let user = unwrap!(
        repo_users.get_user(&user_key),
        "user with public key {user_key:?} does not exist."
    );

    stream.send(&project_code).await?;

    let get_secret: Option<ServerSecret> = stream.receive().await?;

    let Some(secret) = get_secret else {
        bail!("project codes do not match.");
    };

    let mut key = user.private_key.clone().unwrap();

    let auth = key.sign(&secret);

    stream.send(&auth).await?;

    let result: Result<(), String> = stream.receive().await?;

    if let Err(message) = result {
        bail!("server error: {message}");
    }

    let users: Users = stream.receive().await?;

    for user in users.iter_owned() {
        // An `Err` is returned if the user already exists,
        // which we can just skip.
        let _ = repo_users.add_user(user);
    }

    Ok(())
}

pub async fn handle_login(
    repo: &Repository,
    stream: &mut impl Stream,
    validate_user: impl FnOnce(&User) -> Result<(), String>
) -> Result<()>
{
    let client_project_code: ObjectHash = stream.receive().await?;

    let secret = (repo.project_code == client_project_code).then(get_server_secret);

    stream.send(&secret).await?;

    if secret.is_none() {
        return Ok(());
    }

    let login: Signature = stream.receive().await?;

    let result: Result<(), String> = if login.verify(&secret.unwrap()) {
        match repo.users.get_user(&login.key()) {
            Some(user) => validate_user(user),
            None => Err("user does not exist".to_string())
        }
    }
    else {
        Err("failed to verify signature".to_string())
    };

    stream.send(&result).await?;

    if result.is_ok() {
        stream.send(&repo.users).await?;
    }

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
