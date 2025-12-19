use std::collections::{HashSet, VecDeque};

use clap::Args as A;
use eyre::{Result, bail};

use libasc::{action::Action, graph::Graph, hash::ObjectHash, repository::Repository, unwrap};

#[derive(A)]
pub struct Args {
    /// The snapshot whose parent will be being changed.
    snapshot: String,

    /// The new parent for the snapshot.
    new_parent: String,
}

pub fn check_parenthood(history: &Graph, child: ObjectHash, parent: ObjectHash) -> bool {
    let mut queue = VecDeque::new();

    queue.push_back(child);

    while let Some(next) = queue.pop_front() {
        if next == parent {
            return true;
        }

        queue.extend(history.get_parents(next).unwrap());
    }

    false
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let current = repo.normalise_version(&args.snapshot)?;

    if current.is_root() {
        bail!("cannot rebase the root snapshot.");
    }

    let new_parent = repo.normalise_version(&args.new_parent)?;

    if current == new_parent {
        bail!("the child and new parent cannot be the same commit.");
    }

    // You can't rebase a snapshot onto one of its children, so that needs to be checked.
    if check_parenthood(&repo.history, new_parent, current) {
        bail!("cannot rebase {current} onto {new_parent} because {current} is a direct child of {new_parent}");
    }

    let previous_parents = unwrap!(
        repo.history.upsert(current, HashSet::from([new_parent])),
        "snapshot {current} does not exist in the repository."
    );

    repo.action_history.push(
        Action::RebaseSnapshot {
            hash: current,
            from: previous_parents.iter().cloned().collect(),
            to: vec![new_parent]
        }
    );

    repo.save()?;

    Ok(())
}
