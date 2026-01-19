use clap::Args as A;
use eyre::Result;

use libasc::repository::Repository;

#[derive(A)]
pub struct Args {
    /// Undo all actions. Overrides '--count'.
    #[arg(long)]
    all: bool,

    /// The number of actions to undo. Defaults to 1.
    #[arg(short, long)]
    count: Option<usize>
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    let count = args.all
        .then_some(usize::MAX)
        .or(args.count)
        .unwrap_or(1);

    let mut done = 0;

    for _ in 0..count {
        if let Some(action) = repo.undo_action()? {
            println!(" * {action}");
            
            done += 1;
        }
        else {
            break;
        }
    }

    repo.save()?;

    println!("Undid {done} actions.");

    Ok(())
}