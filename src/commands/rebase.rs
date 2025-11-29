use std::collections::VecDeque;

use clap::Args as A;
use eyre::{Result, bail, eyre};

use crate::backend::{action::Action, graph::Graph, hash::ObjectHash, repository::Repository};

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

    let new_parent = repo.normalise_version(&args.new_parent)?;

    if current == new_parent {
        bail!("the child and new parent cannot be the same commit.");
    }

    // You can't rebase a snapshot onto one of its children, so that needs to be checked.
    if check_parenthood(&repo.history, new_parent, current) {
        bail!("cannot rebase {current} onto {new_parent} because {current} is a direct child of {new_parent}");
    }

    let parents = repo
        .history
        .links
        .get_mut(&current)
        .ok_or(eyre!("{current} does not exist in the repository."))?;

    let previous_parents = parents.clone();

    parents.clear();

    parents.insert(new_parent);

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
