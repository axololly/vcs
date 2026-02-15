use eyre::Result;
use libasc::{repository::Repository, sync::remote::Remote, unwrap};

#[derive(clap::Subcommand)]
pub enum Subcommands {
    /// Add a remote URL.
    Add {
        name: String,
        url: String
    },

    /// Remove a remote URL.
    Remove {
        name: String
    },

    /// Rename a remote URL.
    Rename {
        old: String,
        new: String
    },

    /// List URLs this repository can push and pull to.
    List
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    use Subcommands::*;

    let mut repo = Repository::load()?;

    match subcommand {
        Add { name, url } => {
            let remote = unwrap!(
                Remote::from_url(&url),
                "could not understand URL: {url:?}"
            );

            if let Some(original) = repo.remotes.create(name.clone(), remote) {
                eprintln!("There is already a remote under the name {name:?}: {original}.");
            }
            else {
                println!("Created the remote {name:?} at {url:?}");
            }
        },

        Remove { name } => {
            let Some(remote) = repo.remotes.remove(&name) else {
                eprintln!("No remote under the name {name:?}");

                return Ok(());
            };
            
            println!("Removed the remote {name:?} ({remote}).");
        },

        List => {
            if repo.remotes.is_empty() {
                eprintln!("No remotes are on this repository.");

                return Ok(());
            }

            let mut remotes: Vec<_> = repo.remotes.iter().collect();

            remotes.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));

            for (name, remote) in remotes {
                println!("{name}\t{remote}");
            }
        },

        Rename { old, new } => {
            if !repo.remotes.rename(&old, new) {
                eprintln!("No remote under the name {old:?}.");

                return Ok(());
            }
        }
    }
    
    Ok(())
}
