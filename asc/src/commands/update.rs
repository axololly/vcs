use std::collections::HashSet;

use libasc::{change::FileChange, repository::Repository, utils::resolve_wildcard_path};

use eyre::Result;
use relative_path::{PathExt, RelativePathBuf};

pub fn parse() -> Result<()> {
    let mut repo = Repository::load()?;

    let mut staged_files: HashSet<RelativePathBuf> = repo.staged_files
        .drain(..)
        .collect();

    let mut added = vec![];
    let mut removed = vec![];
        
    for path in resolve_wildcard_path(&repo.root_dir)? {
        let relative = path.relative_to(&repo.root_dir)?;

        if repo.is_ignored_path(&path) {
            if staged_files.contains(&relative) {
                removed.push(relative);
            }
            
            continue;
        }

        if !staged_files.contains(&relative) {
            added.push(relative);
        }
    }

    let added_files = added.len();

    for path in added {
        staged_files.insert(path.clone());

        println!("{}", FileChange::Added(path));
    }

    let removed_files = removed.len();
    
    for path in removed {
        staged_files.remove(&path);

        println!("{}", FileChange::Removed(path));
    }

    println!("Added {added_files} files, removed {removed_files} files");

    repo.save()?;

    Ok(())
}
