use eyre::Result;

use libasc::{filter_with_glob_indexes, repository::Repository};
use relative_path::{PathExt, RelativePathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// The current location of a file.
    /// This path must be part of the repository.
    old: RelativePathBuf,

    /// The new location for the file, or
    /// the new directory to put it under.
    new: RelativePathBuf
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let staged_files = repo.staged_files.clone();

    let paths_to_move = filter_with_glob_indexes(
        vec![&args.old],
        &staged_files
    );

    if paths_to_move.is_empty() {
        eprintln!("No files found to move from {:?}.", args.old);

        return Ok(());
    }
    
    let new_path = args.new.to_logical_path(&repo.root_dir);
    
    if new_path.is_dir() {
        for (index, path) in paths_to_move {
            let relative = path.relative(&args.old);
            
            let resolved_path = relative.to_logical_path(&new_path);

            let new_path = resolved_path.relative_to(&repo.root_dir)?;

            println!("Moved: {path} -> {new_path}");

            repo.staged_files[index] = new_path;
        }
    }

    repo.save()?;

    Ok(())
}
