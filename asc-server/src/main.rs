use std::{fs, sync::Arc};

use chrono::Utc;
use eyre::Result;
use libasc::{repository::Repository, sync::{entry::handle_server, stream::StdinStdout}};
use tokio::sync::Mutex;

async fn run() -> Result<()> {
    stable_eyre::install()?;

    let repo = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-remote-repo")?
    ));

    let mut stream = StdinStdout::new();

    handle_server(&mut stream, repo).await
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let mut now = Utc::now().to_string();

        if let Some(i) = now.find('.') {
            let _ = now.split_off(i);
        }

        let _ = fs::write(
            format!("/var/tmp/asc-server-{}", now),
            format!("{e:?}")
        );
    }
}
