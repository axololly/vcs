use std::{collections::{HashSet, VecDeque}, fs, path::PathBuf};

use crate::{backend::{hash::ObjectHash, repository::Repository}, unwrap, utils::resolve_wildcard_path};

use eyre::Result;

pub fn parse() -> Result<()> {
    let mut repo = Repository::load()?;

    repo.action_history.clear();

    let mut valid_blobs: HashSet<PathBuf> = HashSet::new();

    let mut queue: VecDeque<ObjectHash> = VecDeque::from_iter(repo.branches.values().cloned());

    loop {
        let Some(current) = queue.pop_front() else { break };

        if current.is_root() {
            break;
        }

        // TODO: This doesn't work if the hash isn't directly in the trash
        // This needs to check children, probably requiring an inverted graph
        if repo.trash.contains(current) {
            continue;
        }

        valid_blobs.insert(repo.hash_to_path(current));

        let snapshot = repo.fetch_snapshot(current)?;

        valid_blobs.extend(snapshot.files.values().map(|&hash| repo.hash_to_path(hash)));

        let parents = repo.history.get_parents(current).unwrap();
        
        queue.extend(parents.iter());
    }

    for stash in &repo.stashes {
        let snapshot = repo.fetch_snapshot(stash.snapshot)?;

        valid_blobs.insert(repo.hash_to_path(snapshot.hash));

        valid_blobs.extend(snapshot.files.values().map(|&hash| repo.hash_to_path(hash)));
    }
    
    let all_blobs = HashSet::from_iter(
        resolve_wildcard_path(repo.blobs_dir().join("**/*"))?
    );

    let mut removed: usize = 0;

    for path in all_blobs.symmetric_difference(&valid_blobs) {
        unwrap!(
            fs::remove_file(path),
            "failed to delete path {} when cleaning repository", path.display()
        );

        removed += 1;
    }

    println!("Removed {removed} objects.");

    repo.save()?;

    Ok(())
}