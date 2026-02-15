use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;

use libasc::{hash::ObjectHash, repository::Repository, snapshot::Snapshot, unwrap};

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Short,
    Medium,
    Long
}

#[derive(clap::Args)]
pub struct Args {
    /// The path to filter commits based on.
    path: Option<PathBuf>,

    /// How many snapshots to display.
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// The branch to display snapshots on. Defaults to current branch.
    #[arg(short, long)]
    branch: Option<String>,

    /// The format to use when listing snapshots.
    #[arg(short, long, value_enum)]
    format: Option<Format>,

    /// Check for snapshots before a certain datetime.
    #[arg(long = "before")]
    snapshots_before: Option<DateTime<Utc>>,

    /// Check for snapshots after a certain datetime.
    #[arg(long = "after")]
    snapshots_after: Option<DateTime<Utc>>
}

fn first_line_only(message: &str) -> &str {
    message.lines().next().unwrap()
}

pub fn parse(args: Args) -> Result<()> {
    if args.snapshots_before.is_some() && args.snapshots_after.is_some() {
        eprintln!("'--before' and '--after' are mutually exclusive.");

        return Ok(());
    }

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

        let parents = unwrap!(
            repo.history.get_parents(current_hash),
            "snapshot hash {current_hash} is not referenced in the snapshot tree."
        );

        // TODO: allow traversing if node is a merge child?

        // 0 parents -> root
        // 2+ parents -> merge
        if parents.len() != 1 {
            break;
        }
        
        let &next_hash = parents.iter().next().unwrap();

        current_hash = next_hash;
    }

    if let Some(path) = &args.path {
        let mut valid_snapshots = vec![];

        let mut current_hash = ObjectHash::default();

        for snapshot in snapshots {
            let Some(&content_hash) = snapshot.files.get(path) else {
                continue;
            };

            if content_hash == current_hash {
                continue;
            }

            current_hash = content_hash;

            valid_snapshots.push(snapshot);
        }

        snapshots = valid_snapshots;
    }

    if let Some(datetime) = args.snapshots_before {
        snapshots.retain(|snapshot| snapshot.timestamp < datetime);
    }

    if let Some(datetime) = args.snapshots_after {
        snapshots.retain(|snapshot| snapshot.timestamp > datetime);
    }

    if snapshots.is_empty() {
        eprintln!("No snapshots found.");

        return Ok(());
    }

    let snapshots_to_show = snapshots
        .iter()
        .take(args.limit.unwrap_or(usize::MAX));

    for snapshot in snapshots_to_show {
        match args.format.unwrap_or(Format::Medium) {
            Format::Short => {
                let line = format!("{}", snapshot.hash);

                if repo.current_hash == snapshot.hash {
                    println!("{}", line.green());
                }
                else {
                    println!("{line}");
                }
            }
            
            Format::Medium => {
                let author = repo.users
                    .get_user(&snapshot.author)
                    .map(|u| u.name.as_str())
                    .unwrap_or("<unknown user>");

                let line = format!(
                    "[{}]  {} (user: {author})",
                    snapshot.hash,
                    first_line_only(&snapshot.message)
                );

                if repo.current_hash == snapshot.hash {
                    println!("{}", line.green());
                }
                else {
                    println!("{line}");
                }
            }

            Format::Long => {
                let line = format!("Hash: {:?}", snapshot.hash);

                if repo.current_hash == snapshot.hash {
                    println!("{}", line.green());
                }
                else {
                    println!("{line}");
                }

                let author = repo.users
                    .get_user(&snapshot.author)
                    .map(|u| u.name.as_str())
                    .unwrap_or("<unknown user>");

                println!("Message: {}", first_line_only(&snapshot.message));
                println!("Author: {}", author);
                println!("Timestamp: {}", snapshot.timestamp);
                println!();
            }
        }
    }

    Ok(())
}
