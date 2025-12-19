use std::{fs, path::PathBuf};

use clap::Args as A;
use eyre::{Result, bail};

use libasc::repository::Repository;

#[derive(A)]
pub struct Args {
    /// The paths to display the contents of.
    paths: Vec<PathBuf>,

    /// Write to a file instead of stdout.
    #[arg(short, long = "outfile")]
    out_file: Option<PathBuf>,

    /// The version of the file to use.
    #[arg(short, long)]
    version: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let mut to_write = String::new();
    
    for path in &args.paths {
        let content = if let Some(version) = &args.version {
            let snapshot_hash = repo.normalise_version(version)?;

            let snapshot = repo.fetch_snapshot(snapshot_hash)?;
            
            let Some(&content_hash) = snapshot.files.get(path) else {
                bail!("no such path in snapshot {snapshot_hash}: {}", path.display());
            };
        
            repo.fetch_string_content(content_hash)?.resolve(&repo)?
        }
        else if !repo.staged_files.contains(path) {
            bail!("path {} is not found in the staging area.", path.display());
        }
        else {
            fs::read_to_string(path)?
        };

        to_write.push_str(&content);
    }

    if let Some(out_file) = args.out_file {
        fs::write(out_file, to_write)?;
    }
    else {
        println!("{to_write}");
    }
    
    Ok(())
}