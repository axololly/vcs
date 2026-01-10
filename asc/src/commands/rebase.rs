use std::collections::HashSet;

use clap::Args as A;
use eyre::{Result, bail};

use libasc::{action::Action, repository::Repository};

#[derive(A)]
pub struct Args {
    /// The snapshot whose parent will be being changed.
    snapshot: String,

    /// The new parent for the snapshot.
    new_parents: Vec<String>,
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let to_rebase = repo.normalise_version(&args.snapshot)?;

    if to_rebase.is_root() {
        bail!("cannot rebase the root snapshot.");
    }

    let new_parents = {
        let mut v = HashSet::new();
        
        for parent in args.new_parents {
            let new_parent = repo.normalise_version(&parent)?;

            if to_rebase == new_parent {
                bail!("cannot rebase commit on itself.");
            }

            if repo.history.is_descendant(new_parent, to_rebase)? {
                bail!("cannot rebase {to_rebase} onto {new_parent} because {new_parent} is a direct child of {new_parent}.");
            }

            v.insert(new_parent);
        }

        v
    };
    
    let previous_parents = repo.history.remove(to_rebase).unwrap();

    for &parent in &new_parents {
        repo.history.insert(to_rebase, parent)?;
    }

    repo.action_history.push(
        Action::RebaseSnapshot {
            hash: to_rebase,
            from: previous_parents.iter().cloned().collect(),
            to: new_parents.iter().cloned().collect()
        }
    );

    repo.save()?;

    Ok(())
}
