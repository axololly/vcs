use clap::{Args as A, ValueEnum};
use eyre::eyre;

use crate::backend::{commit::CommitHeader, repository::Repository};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Short,
    Medium,
    Long
}

#[derive(A)]
pub struct Args {
    /// How many commits to display.
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// The branch to display commits on. Defaults to current branch.
    #[arg(short, long)]
    branch: Option<String>,

    /// The format to use when listing commits.
    #[arg(short, long, value_enum)]
    format: Option<Format>
}

pub fn parse(args: Args) -> eyre::Result<()> {
    let repo = Repository::load()?;

    let mut current_hash = if let Some(branch) = args.branch {
        *repo.branches
            .get(&branch)
            .ok_or(eyre!("branch {branch:?} does not exist"))?
    }
    else {
        repo.current_hash
    };

    let mut commit_headers: Vec<CommitHeader> = vec![];

    loop {
        let current = repo.fetch_commit_header(current_hash)?;

        commit_headers.push(current);

        let parents = repo.commit_history
            .get_parents(current_hash)
            .ok_or(eyre!("commit hash {current_hash} is not referenced in the commit tree"))?;

        // 0 parents -> root
        // 2+ parents -> merge
        if parents.len() != 1 {
            break;
        }

        current_hash = parents[0];
    }

    if commit_headers.is_empty() {
        return Err(eyre!("no commits have been made on this branch"));
    }

    for header in commit_headers {
        let hash = header.hash.shorten();

        match args.format.unwrap_or(Format::Medium) {
            Format::Short => {
                println!("{hash}");
            }
            
            Format::Medium => {
                println!("[{hash}]\t{} (user: {})", header.message, header.author);
            }

            Format::Long => {
                println!("Hash: {:?}", header.hash);
                println!("Author: {}", header.author);
                println!("Message: {}", header.message);
                println!("Timestamp: {}", header.timestamp);
                println!();
            }
        }
    }

    Ok(())
}