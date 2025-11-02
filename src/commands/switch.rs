use std::str::FromStr;

use clap::Args as A;

use crate::backend::{hash::CommitHash, repository::Repository};

#[derive(A)]
pub struct Args {
    /// The version to change to.
    /// This can be a branch name or a commit hash.
    version: String
}

pub fn parse(args: Args) -> eyre::Result<()> {
    let mut repo = Repository::load()?;

    let new_hash = match repo.branches.get(&args.version) {
        Some(&hash) => hash,
        None => CommitHash::from_str(&args.version)?
    };

    let before = repo.current_branch()
        .map(String::from)
        .unwrap_or(format!("{}", repo.current_hash));

    repo.current_hash = new_hash;

    repo.save()?;

    let after = repo.current_branch()
        .map(String::from)
        .unwrap_or(format!("{}", repo.current_hash));

    println!("Switched versions: {before} -> {after}");

    Ok(())
}