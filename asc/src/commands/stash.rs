use std::{collections::BTreeMap, io::Read};

use clap::Subcommand;

use libasc::{get_content_from_editor, open_file, repository::Repository, stash::State, unwrap};

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
        id: Option<usize>
    },

    /// Replace the working directory with a snapshot
    /// from the stash, deleting the snapshot in the process.
    Pop {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<usize>
    },

    /// Functions like `pop` but does not delete the snapshot.
    Apply {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<usize>
    },
    
    /// Functions like `apply` but changes the HEAD to the basis of
    /// the applied stash.
    Goto {
        /// The stash ID of the snapshot to use.
        /// Defaults to the topmost stash ID.
        id: Option<usize>
    }
}

static TEMPLATE_MESSAGE: &str = "
# Enter a message for this stash.
# Lines starting with '#' are ignored.
# Whitespace before and after the message is also ignored.
";

pub fn parse(subcommand: Subcommands) -> eyre::Result<()> {
    let mut repo = Repository::load()?;
    
    use Subcommands::*;

    match subcommand {
        New { message, editor } => {
            parse(Subcommands::Save { message, editor })?;

            let current = repo.fetch_current_snapshot()?;

            repo.replace_cwd_with_files(&current.files)?;

            println!("Reverted back to: {:?}", current.hash);
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

            let mut files = BTreeMap::new();
            
            for path in &repo.staged_files {
                let content = {
                    let mut buf = String::new();
                    
                    let mut fp = open_file(path)?;

                    fp.read_to_string(&mut buf)?;

                    buf
                };

                repo.save_content(&content, Some(repo.current_hash))?;

                files.insert(path.to_path_buf(), repo.current_hash);
            }

            let state = State {
                files,
                message
            };

            let stash_id = repo.stash.add_state(state, repo.current_hash);

            println!("Created new stash with ID {stash_id}");
        }

        Delete { id: Some(id) } => {
            if repo.stash.remove_state(id).is_some() {
                println!("Removed stash {id}.");
            }
            else {
                eprintln!("No stash with ID {id}.");
            }
        }

        Delete { id: None } => {
            repo.stash.clear();

            println!("Removed all stashes.");
        }

        Pop { id } => {
            let Some(topmost) = repo.stash.topmost_id() else {
                eprintln!("The stash is empty.");

                return Ok(());
            };

            let id = id.unwrap_or(topmost);
            
            let Some(entry) = repo.stash.get_state(id) else {
                eprintln!("No stash with ID {id}.");

                return Ok(());
            };

            if repo.has_unsaved_changes()? {
                eprintln!("Cannot update working directory with unsaved changes.");

                return Ok(());
            }
            
            repo.replace_cwd_with_files(&entry.state.files.clone())?;

            println!("Popped stash with ID {id}");
        }

        Apply { id } => {
            let Some(topmost) = repo.stash.topmost_id() else {
                eprintln!("The stash is empty.");

                return Ok(());
            };

            let id = id.unwrap_or(topmost);
            
            let Some(entry) = repo.stash.get_state(id) else {
                eprintln!("No stash with ID {id}.");

                return Ok(());
            };

            if repo.has_unsaved_changes()? {
                eprintln!("Cannot update working directory with unsaved changes.");

                return Ok(());
            }
            
            repo.replace_cwd_with_files(&entry.state.files.clone())?;

            println!("Restored working directory to stash ID {id}");
        }

        Goto { id } => {
            let Some(topmost) = repo.stash.topmost_id() else {
                eprintln!("The stash is empty.");

                return Ok(());
            };

            let id = id.unwrap_or(topmost);
            
            let Some(entry) = repo.stash.get_state(id) else {
                eprintln!("No stash with ID {id}.");

                return Ok(());
            };

            let snapshot = repo.fetch_snapshot(entry.basis)?;

            let before = repo.current_hash;

            let after = entry.basis;

            repo.replace_cwd_with_snapshot(&snapshot)?;

            repo.current_hash = after;

            println!("Restored working directory to stash {} (HEAD switched: {before} -> {after})", snapshot.hash);
        },

        List => {
            if repo.stash.is_empty() {
                eprintln!("The stash is empty.");

                return Ok(());
            }

            println!("Stashes:");

            for (id, entry) in repo.stash.iter() {
                println!("    {}: [{}] on {}", id, entry.basis, entry.timestamp);
                println!("        {}", entry.state.message);
            }
        }
    }

    repo.save()?;

    Ok(())
}
