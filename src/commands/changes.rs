use std::{collections::HashSet, fs, path::PathBuf};

use clap::Args as A;
use eyre::Result;

use crate::{backend::{change::FileChange, repository::Repository}, utils::hash_raw_bytes};

#[derive(A)]
pub struct Args {
    /// Include unchanged files in the list of changes.
    verbose: bool
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let old_files = repo.fetch_current_snapshot()?.files;

    let old_paths: HashSet<&PathBuf> = old_files
        .keys()
        .collect();

    let new_paths: HashSet<&PathBuf> = repo.staged_files
        .iter()
        .collect();

    let mut file_changes: Vec<FileChange> = vec![];

    file_changes.extend(
        new_paths
            .difference(&old_paths)
            .map(|&p| FileChange::Added(p))
    );

    file_changes.extend(
        old_paths
            .difference(&new_paths)
            .map(|&p| FileChange::Removed(p))
    );

    file_changes.extend(
        new_paths
            .iter()
            .filter_map(|&p| (!p.exists()).then_some(FileChange::Missing(p)))
    );

    for (path, &hash) in &old_files {
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(path)?;

        let content_hash = hash_raw_bytes(&content);
        
        if hash != content_hash {
            file_changes.push(FileChange::Edited(path));
            continue;
        }

        if args.verbose {
            file_changes.push(FileChange::Unchanged(path));
        }
    }

    if file_changes.is_empty() {
        println!("No changes can be displayed.");

        return Ok(());
    }
    
    for change in file_changes {
        println!("{change}");
    }
    
    Ok(())
}