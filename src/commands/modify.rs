use clap::Args as A;
use eyre::{Result, bail};

use crate::backend::{action::Action, repository::Repository, snapshot::Snapshot};

#[derive(A)]
pub struct Args {
    /// The commit hash to modify.
    commit: String,

    /// The new author of the snapshot.
    #[arg(short, long)]
    author: Option<String>,

    /// The new message for the snapshot.
    #[arg(short, long)]
    message: Option<String>,

    /// The new datetime for the snapshot.
    #[arg(long)]
    datetime: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    if args.author.is_none() && args.message.is_none() && args.datetime.is_none() {
        bail!("a modification to the requested commit must be specified.");
    }

    let mut repo = Repository::load()?;

    let version = repo.normalise_hash(&args.commit)?;

    let original = repo.fetch_snapshot(version)?;

    let snapshot_before = original.clone();

    let author = args.author.unwrap_or(original.author);

    let message = args.message.unwrap_or(original.message);

    if message.is_empty() {
        bail!("messages for snapshots cannot be empty.");
    }

    let timestamp = args.datetime
        .map(|s| s.parse())
        .unwrap_or(Ok(original.timestamp))?;

    let snapshot = Snapshot::from_parts(author, message, timestamp, original.files);

    if snapshot_before.hash == snapshot.hash {
        bail!("no changes were made to the commit (hashes were equal).");
    }

    repo.save_snapshot(&snapshot)?;

    repo.history.rename(original.hash, snapshot.hash)?;

    println!("Snapshot hash changed: {} -> {}", snapshot_before.hash, snapshot.hash);

    repo.action_history.push(
        Action::ModifySnapshot {
            hash: snapshot_before.hash,
            before: snapshot_before,
            after: snapshot
        }
    );

    repo.save()?;

    Ok(())
}