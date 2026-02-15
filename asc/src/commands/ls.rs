use eyre::Result;

use libasc::{filter_with_glob, repository::Repository};

#[derive(clap::Args)]
pub struct Args {
    /// The pattern to glob against. Omitting this lists from the repository root.
    patterns: Option<Vec<String>>,

    /// Include hidden files.
    #[arg(short = 'a', long = "all")]
    include_hidden: bool,

    /// List contents from another version. Omitting this uses the current version.
    #[arg(short, long)]
    version: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let paths: Vec<String> = if let Some(raw_version) = &args.version {
        let version = repo.normalise_version(raw_version)?;

        let snapshot = repo.fetch_snapshot(version)?;

        snapshot.files
            .keys()
            .map(|p| format!("{p:?}"))
            .collect()
    }
    else {
        repo.staged_files
            .iter()
            .map(|p| format!("{p:?}"))
            .collect()
    };

    let patterns = args.patterns.unwrap_or(vec!["**/*".to_string()]);

    let mut valid_paths: Vec<&String> = filter_with_glob(patterns, &paths);

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
