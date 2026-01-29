use std::{collections::HashSet, path::{Path, PathBuf}};

use libasc::{change::FileChange, repository::Repository, resolve_wildcard_path};

use eyre::Result;

pub fn parse() -> Result<()> {
    let mut repo = Repository::load()?;

    let old: HashSet<PathBuf> = HashSet::from_iter(std::mem::take(&mut repo.staged_files));

    let new: HashSet<PathBuf> = HashSet::from_iter(
        resolve_wildcard_path(Path::new("."))
            .into_iter()
            .flatten()
    );

    let mut added = 0;
    let mut removed = 0;

    for path in new.difference(&old).cloned() {
        println!("{}", FileChange::Added(path));

        added += 1;
    }
    
    for path in old.difference(&new).cloned() {
        println!("{}", FileChange::Removed(path));

        removed += 1;
    }

    println!("Added {} files, removed {} files", added, removed);

    repo.staged_files = new.into_iter().collect();

    repo.save()?;

    Ok(())
}
