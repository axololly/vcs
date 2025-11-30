mod add;
mod blame;
mod branch;
mod cat;
mod clean;
mod commit;
mod diff;
mod history;
mod init;
mod merge;
mod modify;
mod mv;
mod ls;
mod log;
mod rebase;
mod redo;
mod remove;
mod tag;
mod trash;
mod stash;
mod switch;
mod undo;
mod update;

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
    Mv(mv::Args),

    /// Make a commit to the repository.
    #[command(name = "commit", visible_alias = "ci")]
    Commit(commit::Args),

    /// View the commit history of the repository.
    History(history::Args),

    /// Interact with branches in the repository.
    #[command(subcommand)]
    Branch(branch::Subcommands),

    /// Switch between versions in the repository.
    Switch(switch::Args),

    /// Compare versions in the repository.
    Diff(diff::Args),

    /// Update staged files to match the ignore file.
    Update,

    /// Clean out unused objects and reset the edit stack.
    Clean,

    /// Undo an action.
    Undo(undo::Args),

    /// Redo an action.
    Redo(redo::Args),

    /// Review previous actions on the repository.
    Log(log::Args),

    /// List the contents of a directory in the repository.
    Ls(ls::Args),

    /// Display the contents of a file in the repository.
    Cat(cat::Args),

    /// Interact with stashes in the repository.
    #[command(subcommand)]
    Stash(stash::Subcommands),

    /// Merge another branch's tip with the current snapshot.
    Merge(merge::Args),
    
    /// Remove snapshots from the repository.
    #[command(subcommand)]
    Trash(trash::Subcommands),

    /// Modify snapshots in the repository.
    #[command(visible_aliases = ["mod", "edit"])]
    Modify(modify::Args),

    /// Change the parent of a snapshot in the repository.
    Rebase(rebase::Args),

    /// See which user in the repository modified each line in a file.
    Blame(blame::Args),

    /// Alias a snapshot in the repository.
    #[command(subcommand)]
    Tag(tag::Subcommands)
}

pub fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    use Commands::*;

    match cli.command {
        Init(args) => init::parse(args),
        Add(args) => add::parse(args),
        Remove(args) => remove::parse(args),
        Mv(args) => mv::parse(args),
        Commit(args) => commit::parse(args),
        History(args) => history::parse(args),
        Branch(subcommand) => branch::parse(subcommand),
        Switch(args) => switch::parse(args),
        Diff(args) => diff::parse(args),
        Update => update::parse(),
        Clean => clean::parse(),
        Undo(args) => undo::parse(args),
        Redo(args) => redo::parse(args),
        Log(args) => log::parse(args),
        Ls(args) => ls::parse(args),
        Cat(args) => cat::parse(args),
        Stash(subcommand) => stash::parse(subcommand),
        Merge(args) => merge::parse(args),
        Trash(subcommand) => trash::parse(subcommand),
        Modify(args) => modify::parse(args),
        Rebase(args) => rebase::parse(args),
        Blame(args) => blame::parse(args),
        Tag(subcommand) => tag::parse(subcommand)
    }
}