use clap::Args as A;
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;

use libasc::repository::Repository;

#[derive(A)]
pub struct Args {
    /// The maximum number of actions to list.
    #[arg(short = 'n', long)]
    limit: Option<usize>,

    /// Show hidden redoable actions.
    #[arg(long)]
    all: bool
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let (mut actions, redoable) = repo.action_history.as_slices();

    if let Some(limit) = args.limit {
        actions = actions.rchunks(limit).next().unwrap();
    }

    if actions.is_empty() && redoable.is_empty() {
        println!("No actions have been performed on this repository.");

        return Ok(());
    };

    if repo.action_history.current().is_none() && !args.all {
        println!("No more actions to be undone in this repository.");
        println!("(hint: rerun with '--all' to see redoable actions)");

        return Ok(());
    }

    println!("Actions performed:");

    if args.all {
        for action in redoable {
            let s = format!(" * {action}");

            println!("{}", s.dimmed());
        }
    }

    for action in actions.iter().rev() {
        let mut s = format!(" * {action}");
        
        if Some(action) == repo.action_history.current() {
            s = format!("{} (you are here)", s.bright_green());
        }

        println!("{s}");
    }

    Ok(())
}
