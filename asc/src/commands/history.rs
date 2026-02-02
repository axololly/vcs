use clap::{Args as A, ValueEnum};
use color_eyre::owo_colors::OwoColorize;
use eyre::{Result, bail};

use libasc::{repository::Repository, snapshot::Snapshot, unwrap};

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

fn first_line_only(message: &str) -> &str {
    message.lines().next().unwrap()
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let mut current_hash = if let Some(branch) = args.branch {
        unwrap!(
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
                let line = format!("{}", snapshot.hash);

                if repo.current_hash == snapshot.hash {
                    println!("{}", line.green());
                }
                else {
                    println!("{line}");
                }
            }
            
            Format::Medium => {
                let line = format!(
                    "[{}]  {} (user: {})",
                    snapshot.hash,
                    first_line_only(&snapshot.message),
                    snapshot.author
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
