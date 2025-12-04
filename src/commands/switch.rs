use clap::Args as A;
use eyre::{Result, bail};

use crate::backend::{action::Action, repository::Repository};

#[derive(A)]
pub struct Args {
    /// The version to change to.
    /// This can be a branch name or a commit hash.
    version: String
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    if repo.has_unsaved_changes()? {
        bail!("cannot switch versions with unsaved changes.");
    }

    let previous_hash = repo.current_hash;

    let new_hash = repo.normalise_version(&args.version)?;

    let before = repo.branch_from_hash(previous_hash)
        .map(String::from)
        .unwrap_or(format!("{}", previous_hash));

    let after = repo.branch_from_hash(new_hash)
        .map(String::from)
        .unwrap_or(format!("{}", new_hash));

    repo.replace_cwd_with_snapshot(&repo.fetch_snapshot(new_hash)?)?;

    repo.action_history.push(
        Action::SwitchVersion {
            before: previous_hash,
            after: new_hash
        }
    );

    repo.current_hash = new_hash;
    
    repo.save()?;

    println!("Switched versions: {before} -> {after}");

    Ok(())
}