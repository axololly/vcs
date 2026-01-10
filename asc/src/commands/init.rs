use std::{env::current_dir, path::PathBuf};

use clap::Args as A;
use eyre::Result;

use libasc::repository::Repository;

#[derive(A)]
pub struct Args {
    /// The directory for the project.
    /// Defaults to where this command was invoked.
    directory: Option<PathBuf>,

    /// The name of the project.
    /// Defaults to the name of the directory.
    #[arg(short = 'n', long = "project-name")]
    project_name: Option<String>,

    /// The username of the author of the project.
    /// Defaults to the current system's user.
    #[arg(short, long)]
    author: Option<String>
}

pub fn parse(args: Args) -> Result<()> {
    let root_dir = args.directory.unwrap_or(current_dir()?);

    let dir_name = root_dir
        .file_name()
        .unwrap()
        .to_str()
        .unwrap_or_else(|| {
            panic!("Directory name contains invalid UTF-8, which is disallowed by this program.");
        })
        .to_string();

    let project_name = args.project_name.unwrap_or(dir_name);

    let author = args.author.unwrap_or_else(whoami::username);

    let repo = Repository::create_new(&root_dir, author, project_name)?;

    println!(
        "Created new project {:?} in {} (user: {})",
        repo.project_name,
        repo.root_dir.display(),
        repo.current_user().unwrap().name
    );

    Ok(())
}