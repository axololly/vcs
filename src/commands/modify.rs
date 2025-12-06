use clap::Args as A;
use eyre::{Result, bail};

use crate::backend::{action::Action, repository::Repository, snapshot::Snapshot};

#[derive(A)]
pub struct Args {
    /// The snapshot hash to modify.
    hash: String,

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
    let mut repo = Repository::load()?;

    let version = repo.normalise_hash(&args.hash)?;

    let mut original = repo.fetch_snapshot(version)?;
    let orig_clone = original.clone();

    let author = args.author.unwrap_or(original.author);

    let message = args.message.unwrap_or(original.message);

    if message.is_empty() {
        bail!("messages for snapshots cannot be empty.");
    }

    let timestamp = args.datetime
        .map(|s| s.parse())
        .unwrap_or(Ok(original.timestamp))?;

    // Root is the only one where the hash is not recomputed
    if version.is_root() {
        original.author = author;
        original.message = message;
        original.timestamp = timestamp;

        repo.save_snapshot(&original)?;

        return Ok(());
    }

    let snapshot = Snapshot::from_parts(author, message, timestamp, original.files);

    if original.hash == snapshot.hash {
        bail!("no changes were made to the commit (hashes were equal).");
    }

    repo.save_snapshot(&snapshot)?;

    repo.history.rename(original.hash, snapshot.hash)?;

    println!("Snapshot hash changed: {} -> {}", original.hash, snapshot.hash);

    repo.action_history.push(
        Action::ModifySnapshot {
            hash: original.hash,
            before: orig_clone,
            after: snapshot
        }
    );

    repo.save()?;

    Ok(())
}