use std::{fs, sync::Arc};

use chrono::Utc;
use directories::BaseDirs;
use eyre::{Report, Result};
use libasc::{repository::Repository, sync::{server::handle_server, stream::StdinStdout}};
use tokio::sync::Mutex;

macro_rules! error {
    ($($t:tt)*) => {{
        eprintln!($($t)*);

        return Ok(());
    }};
}

async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    let Some(repo_path) = args.first() else {
        error!("Error: repository path was not specified.");
    };

    let repo = Arc::new(Mutex::new(
        Repository::load_from(repo_path)?
    ));

    let mut stream = StdinStdout::new();

    handle_server(&mut stream, repo).await
}

fn save_error(error: &Report) {
    let mut now = Utc::now().to_string();

    if let Some(i) = now.find('.') {
        let _ = now.split_off(i);
    }

    let now = now.replace(' ', "_");

    let name = format!("asc-server_{}", now);

    let Some(dirs) = BaseDirs::new() else {
        eprintln!("Failed to identify user directories through `directories` crate.");

        return;
    };

    let log_path = dirs.cache_dir().join(name);

    let result = fs::write(
        &log_path,
        format!("{error:?}\n")
    );

    match result {
        Ok(_) => eprintln!("Saved traceback to {}", log_path.display()),
        Err(e) => eprintln!("Failed to save traceback to {}: {e:?}", log_path.display())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    stable_eyre::install()?;
    
    if let Err(e) = run().await {
        save_error(&e);

        error!("Encountered error while running: {e}");
    }

    Ok(())
}
