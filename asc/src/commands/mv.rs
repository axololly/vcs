use eyre::Result;
use libasc::{repository::Repository, utils::{IsGlob, filter_paths_with_glob_indexes_strict, normalise_with_root}};
use relative_path::RelativePathBuf;

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

    let globs = [&args.old];

    let filter_result = filter_paths_with_glob_indexes_strict(
        &globs,
        &repo.staged_files,
        &repo.root_dir
    );

    let paths_to_move = match filter_result {
        Ok(matches) => matches,

        Err(invalid_path) => {
            eprintln!("Path outside of tree: {invalid_path}");

            return Ok(());
        }
    };

    if paths_to_move.is_empty() {
        if args.old.is_glob() {
            eprintln!("No files found to move from expanding glob {:?}.", args.old);
        }
        else { 
            eprintln!("Path doesn't exist: {:?}", args.old);
        }

        return Ok(());
    }
    
    let mut new_path = normalise_with_root(args.new, &repo.root_dir);
    
    if paths_to_move.len() == 1 {
        let (index, path) = paths_to_move[0];

        if new_path.to_logical_path(&repo.root_dir).is_dir() {
            new_path = new_path.join(path.file_name().unwrap());
        }

        println!("Moved: {path} -> {new_path}");

        repo.staged_files[index] = new_path;
    }
    else {
        let new_paths: Vec<(usize, RelativePathBuf)> = paths_to_move
            .iter()
            .map(|(index, path)| (*index, new_path.join(path.file_name().unwrap())))
            .collect();

        for (index, path) in new_paths {
            println!("Moved: {} -> {path}", repo.staged_files[index]);

            repo.staged_files[index] = path;
        }
    }

    repo.save()?;

    Ok(())
}
