use std::{env::current_dir, fs};

use eyre::Result;
use libasc::{change::FileChange, repository::Repository, utils::{filter_paths_with_glob_strict, hash_raw_bytes}};
use relative_path::{PathExt, RelativePathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// The pattern to glob against. Omitting this lists from the repository root.
    patterns: Vec<RelativePathBuf>,

    /// Include hidden files.
    #[arg(short = 'a', long = "all")]
    include_hidden: bool,

    /// List contents from another version. Omitting this uses the current version.
    #[arg(short, long)]
    version: Option<String>,

    /// List file change information.
    #[arg(short = 'c')]
    include_changes: bool
}

pub fn parse(mut args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let current_dir = current_dir()
        .unwrap_or(repo.root_dir.clone());

    let snapshot = if let Some(raw_version) = args.version {
        let version = repo.normalise_version(&raw_version)?;

        repo.fetch_snapshot(version)?
    }
    else {
        repo.fetch_current_snapshot()?
    };

    if args.patterns.is_empty() {
        args.patterns.push(RelativePathBuf::from("."));
    }

    let filter_result = filter_paths_with_glob_strict(
        &args.patterns,
        &repo.staged_files,
        &repo.root_dir
    );

    let mut valid_paths = match filter_result {
        Ok(matches) => matches,
        
        Err(invalid_path) => {
            eprintln!("Path outside of tree: {invalid_path}");

            return Ok(());
        }
    };

    if valid_paths.is_empty() {
        eprintln!("No paths found.");

        return Ok(());
    }

    valid_paths.sort();

    for path in valid_paths {
        let absolute = path.to_logical_path(&repo.root_dir);

        let display_path = absolute.relative_to(&current_dir)?;

        if !args.include_changes {
            println!("{display_path}");

            continue;
        }
        
        if !absolute.exists() {
            println!("{}", FileChange::Missing(display_path));

            continue;
        }

        let data = fs::read(absolute)?;

        let hash = hash_raw_bytes(data);

        if hash == snapshot.files[path] {
            println!("{}", FileChange::Unchanged(display_path));
        }
        else {
            println!("{}", FileChange::Edited(display_path));
        }
    }

    Ok(())
}
