use clap::Args as A;
use eyre::{Result, bail};

use crate::{backend::repository::Repository, unwrap, utils::get_content_from_editor};

#[derive(A)]
pub struct Args {
    /// The message to be attached to the commit.
    #[arg(short, long)]
    message: Option<String>,

    /// A path to the interactive editor used to write an message.
    /// Found from the environment variable 'EDITOR'.
    #[arg(short, long)]
    editor: Option<String>,

    /// The branch for this snapshot to go on.
    /// This will override existing branch names.
    #[arg(short, long)]
    branch: Option<String>
}

pub static COMMIT_TEMPLATE_MESSAGE: &str = "
# Enter a message for this commit.
# Lines starting with '#' are ignored.
# Whitespace before and after the message is also ignored.
";

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    if !repo.has_unsaved_changes()? {
        bail!("no changes to document in the upcoming commit.");
    }

    if repo.staged_files.is_empty() {
        bail!("no files are being tracked - empty snapshots are disallowed.");
    }

    let message = if let Some(msg) = args.message {
        msg
    }
    else {
        let editor = args.editor.unwrap_or(
            unwrap!(std::env::var("EDITOR"), "environment variable 'EDITOR' is not set.")
        );

        let snapshot_message_path = &repo.main_dir().join("SNAPSHOT_MESSAGE");

        get_content_from_editor(&editor, snapshot_message_path, COMMIT_TEMPLATE_MESSAGE)?
    };

    let author = repo.current_user().name.clone();

    let snapshot = repo.capture_current_state(author, message)?;

    if let Some(new_branch) = args.branch {
        if let Some(previous_hash) = repo.branches.get(&new_branch) {
            println!("Branch {new_branch} has moved: {previous_hash} -> {}", snapshot.hash);
        }

        let before = repo
            .current_branch()
            .map(String::from)
            .unwrap_or(repo.current_hash.to_string());

        repo.append_snapshot_to_branch(snapshot, new_branch.clone())?;

        println!("Switched branches: {before} -> {new_branch}");
    }
    else {
        repo.append_snapshot(snapshot)?;
    }

    repo.save()?;

    println!("New version: {:?}", repo.current_hash);
    
    Ok(())
}