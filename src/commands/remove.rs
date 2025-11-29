use std::path::PathBuf;

use clap::Args as A;
use eyre::Result;

use crate::{backend::repository::Repository, utils::resolve_wildcard_path};

#[derive(A)]
pub struct Args {
    /// The paths to remove from tracking. Wildcards will be expanded.
    paths: Vec<PathBuf>
}

pub fn parse(args: Args) -> Result<()> {
    let resolved_paths = args.paths
        .iter()
        .flat_map(resolve_wildcard_path)
        .flatten();

    let mut repo = Repository::load()?;

    for path in resolved_paths {
        if let Some(index) = repo.staged_files.iter().position(|p| p == &path) {
            repo.staged_files.remove(index);
        }
    }

    Ok(())
}