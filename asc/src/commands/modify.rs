use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use clap::Args as A;
use eyre::{Result, bail};

use libasc::{hash::ObjectHash, repository::Repository, unwrap};

#[derive(A)]
pub struct Args {
    /// The snapshot hash to modify.
    hash: String,

    /// The new author of the snapshot.
    #[arg(short, long)]
    author: Option<String>,

    /// The new message for the snapshot.
    #[arg(short, long)]
    message: Option<String>,

    /// The new datetime for the snapshot.
    #[arg(long)]
    datetime: Option<String>
}

fn update_recursively(
    old: ObjectHash,
    new: ObjectHash,
    inverted: &HashMap<ObjectHash, HashSet<ObjectHash>>,
    repo: &mut Repository,
    updated_branches: &mut Vec<(String, (ObjectHash, ObjectHash))>
) -> Result<usize>
{
    let mut count = 0;

    if let Some(parents) = repo.history.get_parents(old).cloned() {
        for parent in parents {
            repo.history.insert(new, parent);
        }
    }

    let children = &inverted[&old];

    for &child in children {
        let mut child_snapshot = repo.fetch_snapshot(child)?;

        child_snapshot.parents.remove(&old);
        child_snapshot.parents.insert(new);

        let old = child_snapshot.hash;

        child_snapshot.rehash();

        let new = child_snapshot.hash;

        repo.save_snapshot(child_snapshot)?;

        let search = repo.branches
            .iter()
            .find(|(_, hash)| **hash == old)
            .map(|(name, _)| name)
            .cloned();

        if let Some(name) = search {
            repo.branches.create(name.clone(), new);

            updated_branches.push((name, (old, new)));
        }

        count += update_recursively(old, new, inverted, repo, updated_branches)?;
    }

    Ok(count)
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    unwrap!(
        repo.current_user(),
        "no valid user is set for this repository."
    );

    let version = repo.normalise_hash(&args.hash)?;

    let mut snapshot = repo.fetch_snapshot(version)?;

    snapshot.verify()?;

    if let Some(author) = args.author {
        let new_owner = unwrap!(
            repo.users.get_user(&author),
            "no user by the name of {author:?} in the repository."
        );

        if new_owner.private_key.is_none() {
            bail!("cannot rename author of commit to user {:?} (no private key)", new_owner.name);
        }

        snapshot.author = new_owner.public_key;
    }

    if let Some(message) = args.message {
        if message.is_empty() {
            eprintln!("Empty messages for snapshots are disallowed.");

            return Ok(());
        }

        snapshot.message = message;
    }

    if let Some(datetime) = args.datetime {
        let timestamp: DateTime<Utc> = unwrap!(
            datetime.parse(),
            "failed to parse raw datetime {datetime:?}"
        );
        
        snapshot.timestamp = timestamp;
    }

    let old_hash = snapshot.hash;

    snapshot.rehash();

    if old_hash == snapshot.hash {
        eprintln!("No changes were made to the snapshot.");

        return Ok(());
    }

    let inverted = repo.history.invert();

    let mut updated_branches = vec![];

    let updated_nodes = update_recursively(
        old_hash,
        snapshot.hash,
        &inverted,
        &mut repo,
        &mut updated_branches
    )?;

    println!("Updated {updated_nodes} nodes.");

    if updated_branches.is_empty() {
        println!("No branches were updated.");
    }
    else {
        println!("Updated branches:");
    }

    for (name, (old, new)) in updated_branches {
        println!(" * {name} ({old} -> {new})");
    }

    repo.save()?;

    Ok(())
}
