use std::collections::HashSet;

use eyre::Result;
use glob_match::glob_match;
use libasc::{change::FileChange, repository::Repository};
use relative_path::RelativePath;

#[derive(clap::Args)]
pub struct Args {
    /// The paths to remove from tracking. Wildcards will be expanded.
    paths: Vec<String>
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let mut dirs = HashSet::new();

    let globs: HashSet<&str> = args.paths
        .iter()
        .map(|glob| glob.strip_suffix("/").unwrap_or(glob))
        .collect();

    let check = |data: &str| {
        for glob in &globs {
            if glob_match(glob, data) {
                return true;
            }
        }

        false
    };

    for path in &repo.staged_files {
        let mut path = path.as_relative_path();

        while let Some(parent) = path.parent() && parent != "" {
            dirs.insert(parent.as_str());

            path = parent;
        }
    }

    dirs.retain(|&dir| check(dir));

    let check_path = |mut path: &RelativePath| {
        loop {
            if check(path.as_str()) {
                break true;
            }

            let Some(parent) = path.parent() else { 
                break false;
            };

            if parent == "" {
                break false;
            }
            
            path = parent;
        }
    };

    let mut to_keep = vec![];
    let mut to_remove = vec![];

    let staged_files = std::mem::take(&mut repo.staged_files);

    for path in staged_files {
        if check_path(path.as_relative_path()) {
            to_remove.push(path);
        }
        else {
            to_keep.push(path);
        }
    }

    repo.staged_files = to_keep;

    if to_remove.is_empty() {
        eprintln!("Nothing to remove.");

        return Ok(());
    }

    for path in to_remove {
        println!("{}", FileChange::Removed(path));
    }

    repo.save()?;

    Ok(())
}
