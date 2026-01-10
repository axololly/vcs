use chrono::{DateTime, Utc};
use clap::Args as A;
use eyre::{Result, bail};

use libasc::{repository::Repository, snapshot::{SignedSnapshotEdit, SnapshotEdit}, unwrap};

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
    let repo = Repository::load()?;

    let editing_user = unwrap!(
        repo.current_user(),
        "no valid user is set for this repository."
    );

    let editing_user_key = editing_user.private_key.unwrap();

    let version = repo.normalise_hash(&args.hash)?;

    let mut snapshot = repo.fetch_snapshot(version)?;

    let mut edits = vec![];

    if let Some(author) = args.author {
        let new_owner = unwrap!(
            repo.users.get_user(&author),
            "no user by the name of {author:?} in the repository."
        );

        let mut private_key = unwrap!(
            new_owner.private_key,
            "cannot rename author of commit to user {:?} (no private key)", new_owner.name
        );

        let signature = private_key.sign(snapshot.hash.as_bytes());

        edits.push(SnapshotEdit::ReplaceAuthor(author, signature));
    }

    if let Some(message) = args.message {
        if message.is_empty() {
            bail!("messages for snapshots cannot be empty.");
        }

        edits.push(SnapshotEdit::ReplaceMessage(message));
    }

    if let Some(datetime) = args.datetime {
        let timestamp: DateTime<Utc> = unwrap!(
            datetime.parse(),
            "failed to parse raw datetime {datetime:?}"
        );
        
        edits.push(SnapshotEdit::ReplaceTimestamp(timestamp));
    }

    if edits.is_empty() {
        bail!("no modifications were given for snapshot.");
    }

    let changes = edits.len();

    let signed_edit = SignedSnapshotEdit::new(edits, editing_user_key);

    snapshot.edits.push(signed_edit);

    println!("Modified snapshot {:?} ({} changes)", snapshot.hash, changes);

    repo.save_snapshot(snapshot)?;

    repo.save()?;

    Ok(())
}