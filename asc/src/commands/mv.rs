use std::path::PathBuf;

use eyre::Result;

use libasc::repository::Repository;

#[derive(clap::Args)]
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

    let raw_index = repo.staged_files
        .iter()
        .position(|p| p == &args.old);
    
    let Some(index) = raw_index else {
        eprintln!("Path {} is not currently being tracked in repository.", args.old.display());

        return Ok(());
    };

    let mut new_path = args.new;
    
    if new_path.is_dir() {
        let raw_file_name = args.old.file_name().unwrap().to_str();

        let Some(file_name) = raw_file_name else {
            eprintln!("File name of {} contains invalid UTF-8.", args.old.display());

            return Ok(());
        };

        new_path.push(file_name);
    }

    println!("Moved: {} -> {}", args.old.display(), new_path.display());

    repo.staged_files[index] = new_path;

    repo.save()?;

    Ok(())
}
