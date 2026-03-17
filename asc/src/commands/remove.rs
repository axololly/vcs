use eyre::Result;
use libasc::{change::FileChange, repository::Repository, utils::filter_paths_with_glob_strict};

#[derive(clap::Args)]
pub struct Args {
    /// The paths to remove from tracking. Wildcards will be expanded.
    paths: Vec<String>
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let staged_files = std::mem::take(&mut repo.staged_files);

    let filter_result = filter_paths_with_glob_strict(
        &args.paths,
        &staged_files,
        &repo.root_dir
    );

    let to_remove = match filter_result {
        Ok(matches) => matches,
        
        Err(invalid_path) => {
            eprintln!("Path outside of tree: {invalid_path}");

            return Ok(());
        }
    };

    if to_remove.is_empty() {
        eprintln!("Nothing to remove.");

        return Ok(());
    }

    for path in &to_remove {
        println!("{}", FileChange::Removed(path));
    }

    repo.staged_files = staged_files
        .iter()
        .filter(|p| !to_remove.contains(p))
        .cloned()
        .collect();

    repo.save()?;

    Ok(())
}
