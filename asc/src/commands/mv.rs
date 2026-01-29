use std::path::PathBuf;

use clap::Args as A;
use eyre::Result;

use libasc::{repository::Repository, unwrap};

#[derive(A)]
pub struct Args {
    /// The current location of a file.
    /// This path must be part of the repository.
    old: PathBuf,

    /// The new location for the file, or
    /// the new directory to put it under.
    new: PathBuf
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let index = unwrap!(
        repo.staged_files
            .iter()
            .position(|p| p == &args.old),
        
        "path is not currently being tracked in repository."
    );

    repo.staged_files.remove(index);

    let mut new_path = args.new;
    
    if new_path.is_dir() {
        let file_name = unwrap!(
            args.old
                .file_name()
                .unwrap()
                .to_str(),
            
            "file name of {} contains invalid UTF-8.", args.old.display()
        );

        new_path.push(file_name);
    }

    println!("Moved: {} -> {}", args.old.display(), new_path.display());

    repo.staged_files.push(new_path);

    repo.save()?;

    Ok(())
}
