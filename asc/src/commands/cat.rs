use std::path::Path;

use eyre::Result;

use libasc::{filter_with_glob, repository::Repository};

#[derive(clap::Args)]
pub struct Args {
    /// The paths to display the contents of.
    globs: Vec<String>,

    /// The version of the file to use.
    #[arg(short, long)]
    version: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let version = if let Some(version) = args.version {
        repo.normalise_version(&version)?
    }
    else {
        repo.current_hash
    };

    let snapshot = repo.fetch_snapshot(version)?;

    let paths: Vec<String> = snapshot.files
        .keys()
        .map(|p| p.display().to_string())
        .collect();

    let valid_paths = filter_with_glob(args.globs, &paths);

    if valid_paths.is_empty() {
        eprintln!("No files found.");

        return Ok(());
    }

    for path in valid_paths {
        let path = Path::new(path);

        let content_hash = snapshot.files[path];

        let content = repo.fetch_string_content(content_hash)?;

        println!("{content}");
    }
    
    Ok(())
}
