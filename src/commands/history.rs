use clap::{Args as A, ValueEnum};
use eyre::{Result, bail};

use crate::{backend::{repository::Repository, snapshot::Snapshot}, unwrap};

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
        *unwrap!(
            repo.branches.get(&branch),
            "branch {branch:?} does not exist."
        )
    }
    else {
        repo.current_hash
    };

    let mut snapshots: Vec<Snapshot> = vec![];

    loop {
        let current = repo.fetch_snapshot(current_hash)?;

        snapshots.push(current);

        if current_hash.is_root() {
            break;
        }

        let parents = unwrap!(
            repo.history.get_parents(current_hash),
            "snapshot hash {current_hash} is not referenced in the snapshot tree."
        );

        // 0 parents -> root
        // 2+ parents -> merge
        if parents.len() != 1 {
            break;
        }
        
        let &next_hash = parents.iter().next().unwrap();

        current_hash = next_hash;
    }

    if snapshots.is_empty() {
        bail!("no snapshots have been made on this branch.");
    }

    let snapshots_to_show = args.limit
        .map(|v| &snapshots[..v])
        .unwrap_or(&snapshots);

    for snapshot in snapshots_to_show {
        match args.format.unwrap_or(Format::Medium) {
            Format::Short => {
                println!("{}", snapshot.hash);
            }
            
            Format::Medium => {
                println!("[{}]  {} (user: {})", snapshot.hash, snapshot.message, snapshot.author);
            }

            Format::Long => {
                println!("Hash: {:?}", snapshot.hash);
                println!("Message: {}", snapshot.message);
                println!("Author: {}", snapshot.author);
                println!("Timestamp: {}", snapshot.timestamp);
                println!();
            }
        }
    }

    Ok(())
}