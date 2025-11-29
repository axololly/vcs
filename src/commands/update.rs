use std::{collections::HashSet, path::{Path, PathBuf}};

use crate::{backend::repository::Repository, utils::resolve_wildcard_path};

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

    for path in new.difference(&old) {
        println!("ADDED     {}", path.display());

        added += 1;
    }
    
    for path in old.difference(&new) {
        println!("REMOVED   {}", path.display());

        removed += 1;
    }

    println!("Added {} files, removed {} files", added, removed);

    repo.save()?;

    Ok(())
}