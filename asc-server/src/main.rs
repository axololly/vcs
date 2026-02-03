use std::{fs, sync::Arc};

use chrono::Utc;
use directories::BaseDirs;
use eyre::Result;
use libasc::{repository::Repository, sync::{server::handle_server, stream::StdinStdout}};
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

        let name = format!("asc-server-{}", now);

        let Some(dirs) = BaseDirs::new() else {
            eprintln!("Failed to identify user directories through `directories` crate.");

            return;
        };

        let log_path = dirs.config_dir().join(name);

        let _ = fs::write(
            log_path,
            format!("{e:?}")
        );
    }
}
