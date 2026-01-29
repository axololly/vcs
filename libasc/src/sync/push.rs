use eyre::{eyre, Result};

use crate::{graph::Graph, hash::ObjectHash, repository::Repository, sync::{stream::Stream, utils::{handle_login, login_as, BranchResponse, Repo}}, unwrap, user::User};

pub enum BranchPushResult {
    CreatedOnRemote,
    UpToDate,
    FastForward(Graph, ObjectHash),
    SplitHistory(Graph, ObjectHash) // Is this even allowed?
}

pub async fn client_push_one_branch(
    stream: &mut impl Stream,
    repo: &Repository,
    branch: &str
) -> Result<BranchPushResult>
{
    let local_tip = repo.branches.get(branch).unwrap();

    stream.send(&(branch, local_tip)).await?;

    let remote_tip: Option<ObjectHash> = stream.receive().await?;

    
    
    todo!()
}

pub async fn handle_push_as_client(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()>
{
    let mut repo = repo.lock().await;

    let user = unwrap!(
        repo.current_user(),
        "no valid user set for this repository."
    );
    
    login_as(user, stream, repo.project_code).await?;

    let branch_names: Vec<_> = repo.branches.names().collect();

    
    
    Ok(())
}

pub async fn handle_push_as_server(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()>
{
    let mut repo = repo.lock().await;

    let check = |user: &User| {
        user.permissions
            .can_push()
            .then_some(())
            .ok_or(format!("user {:?} does not have permission to push", user.name))
    };

    handle_login(&repo, stream, check);

    Ok(())
}
