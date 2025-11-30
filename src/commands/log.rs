use clap::Args as A;
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;

use crate::backend::repository::Repository;

#[derive(A)]
pub struct Args {
    /// The maximum number of actions to list.
    #[arg(short = 'n', long)]
    limit: Option<usize>
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let mut actions = repo.action_history.as_vec().as_slice();

    if let Some(limit) = args.limit {
        actions = actions.rchunks(limit).next().unwrap();
    }

    let Some(current) = repo.action_history.current() else {
        println!("No actions have been performed on this repository.");

        return Ok(());
    };

    println!("Actions performed:");

    for (count, action) in actions.iter().enumerate() {
        let mut s = format!(" * {action}");
        
        if action == current {
            s = format!("{} (you are here)", s.green());
        }

        println!("{s}");

        if let Some(limit) = args.limit && count + 1 == limit {
            break;
        }
    }

    Ok(())
}