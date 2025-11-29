use clap::{Args as A, ValueEnum};
use eyre::{Result, bail, eyre};

use crate::backend::{snapshot::Snapshot, repository::Repository};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Short,
    Medium,
    Long
}

#[derive(A)]
pub struct Args {
    /// How many snapshots to display.
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// The branch to display snapshots on. Defaults to current branch.
    #[arg(short, long)]
    branch: Option<String>,

    /// The format to use when listing snapshots.
    #[arg(short, long, value_enum)]
    format: Option<Format>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let mut current_hash = if let Some(branch) = args.branch {
        *repo.branches
            .get(&branch)
            .ok_or(eyre!("branch {branch:?} does not exist"))?
    }
    else {
        repo.current_hash
    };

    let mut snapshots: Vec<Snapshot> = vec![];

    loop {
        let current = repo.fetch_snapshot(current_hash)?;

        snapshots.push(current);

        let parents = repo.history
            .get_parents(current_hash)
            .ok_or(eyre!("snapshot hash {current_hash} is not referenced in the snapshot tree"))?;

        // 0 parents -> root
        // 2+ parents -> merge
        if parents.len() != 1 {
            break;
        }
        
        let &next_hash = parents.iter().next().unwrap();

        current_hash = next_hash;
    }

    if snapshots.is_empty() {
        bail!("no snapshots have been made on this branch");
    }

    for header in snapshots {
        match args.format.unwrap_or(Format::Medium) {
            Format::Short => {
                println!("{}", header.hash);
            }
            
            Format::Medium => {
                println!("[{}]\t{} (user: {})", header.hash, header.message, header.author);
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