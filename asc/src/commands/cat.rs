use std::io::{stdout, Write};

use eyre::Result;

use libasc::{repository::Repository, utils::filter_paths_with_glob};
use relative_path::RelativePathBuf;

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

    let paths: Vec<&RelativePathBuf> = snapshot.files
        .keys()
        .collect();

    let valid_paths = filter_paths_with_glob(&args.globs, &paths, &repo.root_dir);

    if valid_paths.is_empty() {
        eprintln!("No files found.");

        return Ok(());
    }

    let mut stdout = stdout();

    for &path in valid_paths {
        let content_hash = snapshot.files[path];

        let content = repo.fetch_string_content(content_hash)?;

        stdout.write_all(content.as_bytes())?;

        stdout.flush()?;
    }
    
    Ok(())
}
