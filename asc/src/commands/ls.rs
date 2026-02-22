use eyre::Result;

use libasc::{filter_with_glob, repository::Repository};
use relative_path::RelativePathBuf;

#[derive(clap::Args)]
pub struct Args {
    /// The pattern to glob against. Omitting this lists from the repository root.
    patterns: Vec<String>,

    /// Include hidden files.
    #[arg(short = 'a', long = "all")]
    include_hidden: bool,

    /// List contents from another version. Omitting this uses the current version.
    #[arg(short, long)]
    version: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let snapshot = if let Some(raw_version) = args.version {
        let version = repo.normalise_version(&raw_version)?;

        repo.fetch_snapshot(version)?
    }
    else {
        repo.fetch_current_snapshot()?
    };

    let paths: Vec<RelativePathBuf> = snapshot.files
        .into_keys()
        .collect();

    if args.patterns.is_empty() {
        for path in paths {
            println!("{path}");
        }

        return Ok(());
    }

    let mut valid_paths = filter_with_glob(args.patterns, &paths);

    if valid_paths.is_empty() {
        eprintln!("No paths found.");

        return Ok(());
    }

    valid_paths.sort();

    for path in valid_paths {
        println!("{path}");
    }

    Ok(())
}
