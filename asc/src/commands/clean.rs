use std::{collections::{HashSet, VecDeque}, fs, path::PathBuf};

use libasc::{hash::ObjectHash, repository::Repository, unwrap, resolve_wildcard_path};

use eyre::Result;

pub fn parse() -> Result<()> {
    let mut repo = Repository::load()?;

    let mut valid_blobs: HashSet<PathBuf> = HashSet::new();
    
    let mut valid_commits: HashSet<ObjectHash> = HashSet::new();

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

        valid_commits.insert(current);
        
        valid_blobs.insert(repo.hash_to_path(current));

        let snapshot = repo.fetch_snapshot(current)?;

        valid_blobs.extend(snapshot.files.values().map(|&hash| repo.hash_to_path(hash)));
    }

    let all_commits: HashSet<ObjectHash> = repo.history.iter_hashes().collect();
    let removed = all_commits.difference(&valid_commits).count();

    for &to_remove in all_commits.difference(&valid_commits) {
        repo.history.remove(to_remove);
    }

    println!("Removed {removed} snapshots from the repository history.");
    
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

    println!("Removed {removed} blobs from disk.");

    repo.action_history.clear();

    repo.save()?;

    Ok(())
}