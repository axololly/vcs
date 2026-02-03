use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{sync::{clone::handle_clone_as_server, pull::handle_pull_as_server, push::handle_push_as_server, stream::Stream, utils::Repo}};

#[derive(Deserialize, Serialize)]
pub enum Method {
    Push,
    Pull,
    Clone
}

pub async fn handle_server(stream: &mut impl Stream, repo: Repo) -> Result<()> {
    let method: Method = stream.receive().await?;

    match method {
        Method::Pull => handle_pull_as_server(stream, repo).await,
        Method::Push => handle_push_as_server(stream, repo).await,
        Method::Clone => handle_clone_as_server(stream, repo).await
    }
}
