use color_eyre::owo_colors::OwoColorize;
use eyre::Result;

use libasc::{action::Action, repository::Repository, utils::filter_with_glob};

#[derive(clap::Subcommand)]
pub enum Subcommands {
    /// List what branch you are currently on.
    Current,

    /// Create a new branch from the current commit.
    #[command(visible_alias = "create")]
    New {
        /// The name of the branch.
        name: String,

        /// The version this branch refers to.
        /// Defaults to the current version.
        basis: Option<String>
    },

    /// Move a branch to point to another commit.
    #[command(visible_alias = "mv")]
    Move {
        /// The name of the branch.
        name: String,
        
        /// Where to next point it to.
        new: String
    },

    /// Delete a branch.
    #[command(visible_aliases = ["rm", "remove"])]
    Delete {
        /// The name of the branch.
        names: Vec<String>,

        /// Whether to continue if a name doesn't exist
        #[arg(long)]
        keep_going: bool
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
        /// Globs to filter the search.
        globs: Option<Vec<String>>,

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

        New { name, basis } => {
            let base_version = if let Some(version) = basis {
                repo.normalise_version(&version)?
            }
            else {
                repo.current_hash
            };
            
            if repo.branches.contains(&name) {
                eprintln!("Branch {name:?} already exists.");

                return Ok(());
            }

            if let Some(branch_name) = repo.branches.get_name_for(base_version) {
                println!("Created new branch: {name} -> {branch_name} ({base_version})");
            }
            else {
                println!("Created new branch: {name} -> {base_version}");
            }

            repo.branches.create(name.clone(), base_version);

            repo.action_history.push(
                Action::CreateBranch {
                    hash: repo.current_hash,
                    name
                }
            );
        }

        Move { name, new } => {
            let version = repo.normalise_version(&new)?;

            let Some(previous) = repo.branches.remove(&name) else {
                eprintln!("Branch {new:?} does not exist.");

                return Ok(());
            };

            repo.branches.create(name.clone(), version);

            repo.action_history.push(
                Action::MoveBranch {
                    name,
                    old: previous,
                    new: version
                }
            );
        }

        Rename { old, new } => {
            let Some(commit_hash) = repo.branches.remove(&old) else {
                eprintln!("Branch {old:?} does not exist.");

                return Ok(());
            };

            println!("Renamed: {old} -> {new}");

            repo.branches.create(new.clone(), commit_hash);

            repo.action_history.push(
                Action::RenameBranch {
                    hash: commit_hash,
                    old,
                    new
                }
            );
        }

        Delete { names, keep_going } => {
            for name in names {
                let Some(was_pointing_to) = repo.branches.remove(&name) else {
                    eprintln!("Branch {name:?} does not exist.");

                    if keep_going {
                        continue;
                    }
                    
                    return Ok(());
                };

                println!("Branch {name:?} no longer points to {was_pointing_to}.");

                repo.action_history.push(
                    Action::DeleteBranch {
                        hash: was_pointing_to,
                        name
                    }
                );
            }
        }

        List { globs, verbose } => {
            if repo.is_head_detached() {
                let line = format!(" * HEAD detached at {}", repo.current_hash);

                println!("{}", line.bright_green().bold());
            }

            let globs = globs.unwrap_or(vec!["**/*".to_string()]);

            let branch_names: Vec<&String> = repo.branches.names().collect();

            let valid = filter_with_glob(globs, &branch_names);

            for branch_name in valid {
                let commit_hash = *repo.branches.get(branch_name).unwrap();
                
                let mut s = format!(" * {branch_name}");

                if verbose {
                    s = format!("{s} ({commit_hash})");
                }
                
                if repo.current_hash == commit_hash {
                    s = format!("{}", s.bright_green().bold());
                }
                
                println!("{s}")
            }
        }
    }

    repo.save()?;

    Ok(())
}
