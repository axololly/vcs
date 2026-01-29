use std::io::{Read, stdin};

use clap::Subcommand;
use eyre::{Result, bail};

use libasc::{action::Action, repository::Repository};

#[derive(Subcommand)]
pub enum Subcommands {
    /// Create a new tag in the repository.
    #[command(visible_aliases = ["new", "add"])]
    Create {
        /// The name of the tag.
        name: String,

        /// The version to tag.
        version: String
    },

    /// List all the tags in the repository.
    List {
        #[arg(short = 'n', long)]
        limit: Option<usize>
    },

    /// Delete tags in the repository.
    #[command(visible_aliases = ["rm", "remove"])]
    Delete {
        /// The names of tags to delete.
        names: Vec<String>,

        /// Continue deleting tags if removing one raises an error.
        #[arg(long = "keep-going")]
        keep_going: bool
    },

    /// Rename a tag in the repository.
    Rename {
        /// The name of the tag.
        old: String,

        /// The new name of the tag.
        new: String
    }
}

fn prompt_user(message: impl AsRef<str>) -> Result<bool> {
    let mut stdin = stdin().lock();
    
    loop {
        print!("{} [y/n] ", message.as_ref());

        let mut input = String::new();

        stdin.read_to_string(&mut input)?;

        match input.as_str() {
            "y" | "Y" => break Ok(true),
            "n" | "N" => break Ok(false),

            _ => {
                println!("Invalid input: {input:?}");
            }
        }
    }
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    use Subcommands::*;
    
    match subcommand {
        Create { name, version } => {
            let hash = repo.normalise_version(&version)?;

            if let Some(previous) = repo.tags.create(name.clone(), hash) {
                let prompt = format!("You are going to override the tag {name:?} ({previous}) with {hash}. Are you sure you want to do this?");

                if !prompt_user(prompt)? {
                    repo.tags.create(name.clone(), previous);
                }
            }
            else {
                repo.action_history.push(
                    Action::CreateTag { name, hash }
                );
            }
        },

        List { limit } => {
            let mut tags = repo.tags
                .iter()
                .take(limit.unwrap_or(usize::MAX));

            if let Some((name, &hash)) = tags.next() {
                println!("Tags:");
                println!(" * {name} -> {hash}");
            }
            else {
                println!("There are no tags in this repository.");
                println!("Create a new one with `asc tag create`.");

                return Ok(());
            }

            for (name, &hash) in tags {
                println!(" * {name} -> {hash}");
            }
        },

        Delete { names, keep_going } => {
            for name in names {
                if let Some(removed) = repo.tags.remove(&name) {
                    println!("Removed tag {name:?} ({removed}) from the repository.");

                    repo.action_history.push(
                        Action::RemoveTag {
                            name,
                            hash: removed
                        }
                    );
                }
                else if keep_going {
                    println!("Warning: tag {name:?} does not exist in the repository. Continuing...");
                }
                else {
                    bail!("tag {name:?} does not exist in the repository.");
                }
            }
        },

        Rename { old, new } => {
            if let Some(hash) = repo.tags.remove(&old) {
                println!("Renamed {old:?} to {new:?} ({hash})");

                repo.tags.create(new.clone(), hash);

                repo.action_history.push(
                    Action::RenameTag {
                        old,
                        new,
                        hash
                    }
                );
            }
            else {
                bail!("tag {old:?} does not exist in the repository.")
            }
        }
    }

    repo.save()?;
    
    Ok(())
}
