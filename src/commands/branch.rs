use clap::Subcommand;
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;

use crate::backend::repository::Repository;

#[derive(Subcommand)]
pub enum Subcommands {
    /// List what branch you are currently on.
    Current,

    /// Create a new branch from the current commit.
    New {
        /// The name of the branch.
        name: String
    },

    /// Delete a branch.
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
    List
}

pub fn parse(command: Subcommands) -> eyre::Result<()> {
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
                return Err(eyre!("branch {name:?} already exists"));
            }

            println!("Created new branch: {name}");

            repo.branches.insert(name, repo.current_hash);

            repo.save()?;
        }

        Rename { old, new } => {
            let Some(commit_hash) = repo.branches.remove(&old) else {
                return Err(eyre!("branch {old:?} does not exist"))
            };

            println!("Renamed: {old} -> {new}");

            repo.branches.insert(new, commit_hash);
        }

        Delete { name } => {
            let Some(was_pointing_to) = repo.branches.remove(&name) else {
                return Err(eyre!("branch {name:?} does not exist"));
            };

            println!("Branch {name:?} no longer points to {was_pointing_to}.");
        }

        List => {
            if repo.is_head_detached() {
                println!("{}", format!(" * HEAD detached at {}", repo.current_hash).green());
            }

            for (branch_name, &commit_hash) in &repo.branches {
                if repo.current_hash == commit_hash {
                    println!("{}", format!(" * {branch_name}").green());
                }
                else {
                    println!(" * {branch_name}");
                }
            }
        }
    }

    Ok(())
}