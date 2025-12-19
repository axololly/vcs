use clap::Subcommand;
use eyre::{Result, bail};

use libasc::{repository::Repository, stash::Stash, unwrap, get_content_from_editor};

#[derive(Subcommand)]
pub enum Subcommands {
    /// Create a new stash in the repository,
    /// and then revert to the latest snapshot.
    #[command(visible_alias = "create")]
    New {
        /// The message to go with the stash.
        #[arg(short, long)]
        message: Option<String>,

        /// The editor to use to write the message. Not used if message is given.
        #[arg(short, long)]
        editor: Option<String>
    },

    /// Create a new stash in the repository,
    /// but do not revert to the latest snapshot.
    Save {
        /// The message to go with the stash.
        #[arg(short, long)]
        message: Option<String>,

        /// The editor to use to write the message. Not used if message is given.
        #[arg(short, long)]
        editor: Option<String>
    },

    /// List stashes on the repository.
    #[command(visible_alias = "ls")]
    List,

    /// Delete a stash, or all stashes if no ID is given.
    #[command(visible_alias = "rm")]
    Delete {
        /// The stash ID to remove.
        id: Option<String>
    },

    /// Replace the working directory with a snapshot
    /// from the stash, deleting the snapshot in the process.
    Pop {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<String>
    },

    /// Functions like `pop` but does not delete the snapshot.
    Apply {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<String>
    },
    
    /// Functions like `apply` but changes the HEAD to the basis of
    /// the applied stash.
    Goto {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<String>
    }
}

static TEMPLATE_MESSAGE: &str = "
# Enter a message for this stash.
# Lines starting with '#' are ignored.
# Whitespace before and after the message is also ignored.
";

fn resolve_stash<'a>(repo: &'a Repository, id: Option<&str>) -> Result<(usize, &'a Stash)> {
    if let Some(raw_hash) = id {
        let full = repo.normalise_stash_hash(raw_hash)?;

        let index = repo.stashes
            .iter()
            .position(|s| s.snapshot == full)
            .unwrap();

        Ok((index, &repo.stashes[index]))
    }
    else if let Some(topmost) = repo.stashes.last() {
        Ok((repo.stashes.len() - 1, topmost))
    }
    else {
        bail!("no stashes in the repository.")
    }
}

pub fn parse(subcommand: Subcommands) -> eyre::Result<()> {
    let mut repo = Repository::load()?;
    
    use Subcommands::*;

    match subcommand {
        New { message, editor } => {
            parse(Subcommands::Save { message, editor })?;

            let current = repo.fetch_current_snapshot()?;

            repo.replace_cwd_with_snapshot_unchecked(&current)?;

            println!("Reverted back to:   {:?}", current.hash);
        }

        Save { message, editor } => {
            let message = message
                .map(Ok)
                .unwrap_or_else(|| {
                    let editor = editor.unwrap_or(
                        unwrap!(std::env::var("EDITOR"), "environment variable 'EDITOR' is not set.")
                    );

                    let snapshot_message_path = &repo.root_dir.join("SNAPSHOT_MESSAGE");

                    get_content_from_editor(&editor, snapshot_message_path, TEMPLATE_MESSAGE)
                }
            )?;

            let snapshot = repo.capture_current_state(
                repo.current_user().name.clone(),
                message
            )?;

            let stash = Stash {
                snapshot: snapshot.hash,
                basis: repo.current_hash,
            };

            repo.stashes.push(stash);

            println!("Created new stash: {:?}", snapshot.hash);
        }

        Delete { id: Some(str_hash) } => {
            let stash_id = repo.normalise_stash_hash(&str_hash)?;

            let Some(index) = repo.stashes.iter().position(|s| s.snapshot == stash_id) else {
                bail!("no hash found for {str_hash:?}.")
            };

            let removed_id = repo.stashes.remove(index).snapshot;

            println!("Removed stash {removed_id}.");
        }

        Delete { id: None } => {
            repo.stashes.clear();

            println!("Removed all stashes.");
        }

        Pop { id } => {
            let (index, stash) = resolve_stash(&repo, id.as_deref())?;

            let snapshot = repo.fetch_snapshot(stash.snapshot)?;

            repo.replace_cwd_with_snapshot(&snapshot)?;

            println!("Restored working directory to stash {}", snapshot.hash);

            repo.stashes.remove(index);
        }

        Apply { id } => {
            let (_, stash) = resolve_stash(&repo, id.as_deref())?;

            let snapshot = repo.fetch_snapshot(stash.snapshot)?;

            repo.replace_cwd_with_snapshot(&snapshot)?;

            println!("Restored working directory to stash {}", snapshot.hash);
        }

        Goto { id } => {
            let (_, stash) = resolve_stash(&repo, id.as_deref())?;

            let snapshot = repo.fetch_snapshot(stash.snapshot)?;

            repo.replace_cwd_with_snapshot(&snapshot)?;

            let before = repo.current_hash;

            let after = stash.basis;

            repo.current_hash = after;

            println!("Restored working directory to stash {} (HEAD switched: {before} -> {after})", snapshot.hash);
        },

        List => {
            if repo.stashes.is_empty() {
                println!("There are no stashes in this repository.");

                return Ok(());
            }

            println!("Stashes:");

            for stash in &repo.stashes {
                println!(" * {} (from {})", stash.snapshot, stash.basis);
            }
        }
    }

    repo.save()?;

    Ok(())
}