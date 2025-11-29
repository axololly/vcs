use chrono::{DateTime, Local};
use clap::Args as A;
use eyre::{Result, bail};

use crate::backend::{action::Action, repository::Repository};

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
        bail!("A modification to the requested commit must be specified.");
    }

    let mut repo = Repository::load()?;

    let version = repo.normalise_hash(&args.commit)?;

    let mut snapshot = repo.fetch_snapshot(version)?;

    let snapshot_before = snapshot.clone();

    if let Some(author) = args.author {
        snapshot.author = author;
    }

    if let Some(message) = args.message {
        if message.is_empty() {
            bail!("Messages for snapshots cannot be empty.");
        }

        snapshot.message = message;
    }

    if let Some(raw_datetime) = args.datetime {
        let new_datetime: DateTime<Local> = raw_datetime.parse()?;

        snapshot.timestamp = new_datetime;
    }

    repo.save_snapshot(&snapshot)?;

    repo.action_history.push(
        Action::ModifySnapshot {
            hash: snapshot.hash,
            before: snapshot_before,
            after: snapshot
        }
    );

    repo.save()?;

    Ok(())
}