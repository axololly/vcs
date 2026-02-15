use eyre::Result;

use libasc::{change::FileChange, repository::Repository};

#[derive(clap::Args)]
pub struct Args {
    /// Include unchanged files in the list of changes.
    #[arg(short, long)]
    verbose: bool
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let mut file_changes = repo.list_changes()?;

    if !args.verbose {
        file_changes.retain(|f| !matches!(f, FileChange::Unchanged(_)));
    }

    if file_changes.is_empty() {
        eprintln!("No changes have been made.");

        return Ok(());
    }
    
    for change in file_changes {
        println!("{change}");
    }
    
    Ok(())
}
