use std::io::{Read, stdin};

use eyre::Result;

use libasc::{action::Action, repository::Repository, utils::filter_with_glob};

#[derive(clap::Subcommand)]
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
    #[command(visible_alias = "ls")]
    List {
        globs: Option<Vec<String>>,

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
                    Action::CreateTag {
                        name: name.clone(),
                        hash
                    }
                );
            }

            println!("Created tag: {name:?} -> {hash}");
        },

        List { globs, limit } => {
            let globs = globs.unwrap_or(vec!["**/*".to_string()]);

            let all_tags: Vec<&String> = repo.tags.names().collect();
            
            let tags: Vec<&&String> = filter_with_glob(globs, &all_tags);

            if tags.is_empty() {
                println!("No tags found.");

                return Ok(());
            }

            println!("Tags:");

            for name in tags.iter().take(limit.unwrap_or(usize::MAX)) {
                let hash = repo.tags.get(name).unwrap();
                
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
                    eprintln!("Tag {name:?} does not exist. Continuing...");
                }
                else {
                    eprintln!("Tag {name:?} does not exist. Aborting...");

                    break;
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
                eprintln!("Tag {old:?} does not exist.");
            }
        }
    }

    repo.save()?;
    
    Ok(())
}
