mod add;
mod branch;
mod commit;
mod history;
mod init;
mod remove;
mod rename;
mod switch;

use clap::{Parser, Subcommand};

/// A version control system in Rust, made by axololly.
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new repository.
    Init(init::Args),

    /// Add files to be included in the next commit.
    Add(add::Args),

    /// Remove files from being included in the next commit.
    #[command(visible_aliases = ["forget", "rm"])]
    Remove(remove::Args),

    /// Rename a path, or move it to another directory.
    #[command(name = "rename", visible_alias = "mv")]
    Rename(rename::Args),

    /// Make a commit to the repository.
    #[command(name = "commit", visible_alias = "ci")]
    Commit(commit::Args),

    /// View the commit history of the repository.
    #[command(name = "history", visible_alias = "log")]
    History(history::Args),

    /// Interact with branches in the repository.
    #[command(subcommand)]
    Branch(branch::Subcommands),

    /// Switch between versions in the repository.
    Switch(switch::Args)

}

pub fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    use Commands::*;

    match cli.command {
        Init(args) => init::parse(args),
        Add(args) => add::parse(args),
        Remove(args) => remove::parse(args),
        Rename(args) => rename::parse(args),
        Commit(args) => commit::parse(args),
        History(args) => history::parse(args),
        Branch(subcommand) => branch::parse(subcommand),
        Switch(args) => switch::parse(args)
    }
}