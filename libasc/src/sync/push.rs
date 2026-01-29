use eyre::Result;

use crate::sync::{stream::Stream, utils::Repo};

pub async fn handle_push_as_client<R, W>(stream: &mut impl Stream, repo: Repo) -> Result<()> {
    // TODO
    
    Ok(())
}

pub async fn handle_push_as_server<R, W>(stream: &mut impl Stream, repo: Repo) -> Result<()> {
    // TODO

    Ok(())
}
