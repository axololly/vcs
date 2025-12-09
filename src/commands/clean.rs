use std::{collections::{HashSet, VecDeque}, fs, path::PathBuf};

use crate::{backend::{hash::ObjectHash, repository::Repository}, unwrap, utils::resolve_wildcard_path};

use clap::Args as A;
use eyre::Result;

#[derive(A)]
pub struct Args {
    /// Only clean out invalid commits from the repository.
    /// This does not remove anything from disk.
    #[arg(long = "commits-only")]
    commits_only: bool
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let mut valid_commits: HashSet<ObjectHash> = HashSet::new();
    let mut valid_blobs: HashSet<PathBuf> = HashSet::new();

    let mut queue: VecDeque<ObjectHash> = VecDeque::from_iter(repo.branches.values().cloned());

    loop {
        let Some(current) = queue.pop_front() else { break };

        if current.is_root() {
            break;
        }

        if repo.trash_contains(current).is_some() {
            continue;
        }

        valid_commits.insert(current);

        let parents = repo.history.get_parents(current).unwrap();
        
        queue.extend(parents.iter());

        if args.commits_only {
            continue;
        }

        valid_blobs.insert(repo.hash_to_path(current));

        let snapshot = repo.fetch_snapshot(current)?;

        valid_blobs.extend(snapshot.files.values().map(|&hash| repo.hash_to_path(hash)));
    }

    let all_commits: HashSet<ObjectHash> = repo.history.iter_hashes().collect();
    let removed = all_commits.difference(&valid_commits).count();

    for &to_remove in all_commits.difference(&valid_commits) {
        repo.history.remove(to_remove);
    }

    if args.commits_only {
        println!("Removed {removed} commits from the repository.");

        return Ok(());
    }

    for stash in &repo.stashes {
        let snapshot = repo.fetch_snapshot(stash.snapshot)?;

        valid_blobs.insert(repo.hash_to_path(snapshot.hash));

        valid_blobs.extend(snapshot.files.values().map(|&hash| repo.hash_to_path(hash)));
    }
    
    let all_blobs: HashSet<PathBuf> = resolve_wildcard_path(repo.blobs_dir().join("**/*"))?
        .into_iter()
        .collect();

    let mut removed: usize = 0;

    for path in all_blobs.symmetric_difference(&valid_blobs) {
        unwrap!(
            fs::remove_file(path),
            "failed to delete path {} when cleaning repository.", path.display()
        );

        removed += 1;
    }

    println!("Removed {removed} objects.");

    repo.action_history.clear();

    repo.save()?;

    Ok(())
}