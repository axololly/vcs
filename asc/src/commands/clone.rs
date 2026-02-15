use std::path::{Path, PathBuf};

use eyre::Result;
use libasc::{key::PrivateKey, repository::Repository, sync::{client::Client, remote::Remote}, unwrap};

#[derive(clap::Args)]
pub struct Args {
    /// The repository to clone.
    url: String,

    /// Where to clone the repository to.
    path: PathBuf,

    /// The private key to log into the
    /// server with.
    #[arg(long)]
    login_key: String,

    /// The path to the SSH executable to
    /// use instead of `ssh`.
    #[arg(long)]
    ssh_command: Option<String>,

    /// Create the repository, even if
    /// the directory is not empty.
    #[arg(long)]
    allow_not_empty: bool
}

fn check_dir_is_empty(path: &Path) -> Result<bool> {
    let mut entries = unwrap!(
        path.read_dir(),
        "failed to check if {} was empty",
        path.display()
    );

    if let Some(entry) = entries.next() {
        unwrap!(
            entry,
            "failed to check if {} was empty",
            path.display()
        );

        return Ok(false);
    }

    Ok(true)
}

#[tokio::main]
pub async fn parse(args: Args) -> Result<()> {
    if !args.allow_not_empty && !check_dir_is_empty(&args.path)? {
        eprintln!("Cannot make repository at {} (not empty)", args.path.display());

        return Ok(());
    }

    let remote = Remote::from_url(&args.url)?;

    let user_key = {
        let bytes = hex::decode(args.login_key)?;

        PrivateKey::from_bytes(&bytes)?
    };

    let ssh_command: Option<&str> = args.ssh_command.as_deref();
    
    let mut client = Client::connect(remote, ssh_command).await?;

    client.clone_repo(&args.path, user_key).await?;

    let repo = Repository::load_from(&args.path)?;

    let mut blobs = 0;

    for hash in repo.history.iter_hashes() {
        let snapshot = repo.fetch_snapshot(hash)?;

        blobs += snapshot.files.len();
    }

    let current_branch = repo
        .current_branch()
        .unwrap_or("none");

    println!("Cloned repository {:?}", repo.project_name);
    println!("Commits: {}", repo.history.size());
    println!("Blobs: {blobs}");
    println!("Branch: {current_branch} ({})", repo.current_hash);

    Ok(())
}
