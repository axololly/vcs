use clap::Subcommand;
use color_eyre::owo_colors::OwoColorize;
use eyre::{Result, bail};

use crate::backend::repository::Repository;

#[derive(Subcommand)]
pub enum Subcommands {
    /// List what branch you are currently on.
    Current,

    /// Create a new branch from the current commit.
    #[command(visible_alias = "create")]
    New {
        /// The name of the branch.
        name: String
    },

    /// Delete a branch.
    #[command(visible_aliases = ["rm", "remove"])]
    Delete {
        /// The name of the branch.
        name: String
    },

    /// Rename a branch.
    Rename {
        /// The current name of the branch.
        old: String,

        /// The new name of the branch.
        new: String
    },

    /// List all the branches in the repository.
    #[command(visible_alias = "ls")]
    List {
        /// Include the hashes each branch points to.
        #[arg(short, long)]
        verbose: bool
    }
}

pub fn parse(command: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;
    
    use Subcommands::*;

    match command {
        Current => {
            if let Some(name) = repo.current_branch() {
                println!("{name}");
            }
            else {
                println!("HEAD detached at {}", repo.current_hash);
            }
        }

        New { name } => {
            if repo.branches.contains_key(&name) {
                bail!("branch {name:?} already exists");
            }

            println!("Created new branch: {name}");

            repo.branches.insert(name, repo.current_hash);
        }

        Rename { old, new } => {
            let Some(commit_hash) = repo.branches.remove(&old) else {
                bail!("branch {old:?} does not exist");
            };

            println!("Renamed: {old} -> {new}");

            repo.branches.insert(new, commit_hash);
        }

        Delete { name } => {
            let Some(was_pointing_to) = repo.branches.remove(&name) else {
                bail!("branch {name:?} does not exist");
            };

            println!("Branch {name:?} no longer points to {was_pointing_to}.");
        }

        List { verbose } => {
            if repo.is_head_detached() {
                println!("{}", format!(" * HEAD detached at {}", repo.current_hash).bright_green());
            }

            for (branch_name, &commit_hash) in &repo.branches {
                let mut s = format!(" * {branch_name}");

                if verbose {
                    s = format!("{s} ({commit_hash})");
                }
                
                if repo.current_hash == commit_hash {
                    s = format!("{}", s.bright_green());
                }
                
                println!("{s}")
            }
        }
    }

    repo.save()?;

    Ok(())
}