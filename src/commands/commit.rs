use std::{collections::BTreeMap, fs::{self, File}, path::{Path, PathBuf}, process::Command};

use chrono::Local;
use clap::Args as A;
use eyre::{eyre, Context};
use sha1::{Digest, Sha1};

use crate::backend::{commit::{Commit, CommitHeader}, repository::Repository};

#[derive(A)]
pub struct Args {
    /// The message to be attached to the commit.
    #[arg(short, long)]
    message: Option<String>,

    /// A path to the interactive editor used to write an message.
    /// Found from the environment variable 'EDITOR'.
    editor: Option<PathBuf>,

    /// The branch for this commit to go on.
    /// This will override existing branch names.
    #[arg(short, long)]
    branch: Option<String>
}

fn get_content_from_editor(editor: &str, commit_message_path: &Path) -> eyre::Result<String> {
    // TODO: Fill it with a template like Git and Fossil have
    File::create(commit_message_path)?;

    let mut editor_cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        
        cmd.arg(format!("/c {editor} '{}'", commit_message_path.display()));

        cmd
    }
    else if cfg!(unix) {
        let mut cmd = Command::new("bash");
        
        cmd.arg(format!("-c {editor} '{}'", commit_message_path.display()));

        cmd
    }
    else {
        return Err(eyre!("what the fuck are you running bro 😭"));
    };

    let mut child = editor_cmd.spawn()?;

    if !child.wait()?.success() {
        return Err(eyre!("editor process exited with a non-zero exit code."));
    }

    let content = fs::read_to_string(".asc/COMMIT_MESSAGE")?;

    Ok(content)
}

pub fn parse(args: Args) -> eyre::Result<()> {
    let mut repo = Repository::load()?;

    if repo.staged_files.is_empty() {
        return Err(eyre!("no files are being tracked - empty commits are disallowed"));
    }

    let message = match args.message {
        Some(msg) => msg,

        None => {
            let editor = std::env::var("EDITOR")
                .wrap_err("environment variable 'EDITOR' is not set.")?;

            let commit_message_path = &repo.root_dir.join("COMMIT_MESSAGE");

            get_content_from_editor(&editor, commit_message_path)?
        }
    };

    let mut hasher = Sha1::new();

    let author = repo.current_user.to_string();

    hasher.update(&author);

    hasher.update(&message);

    let now = Local::now();

    hasher.update(now.timestamp().to_be_bytes());

    let mut files = BTreeMap::new();

    for path in &repo.staged_files {
        if !path.exists() {
            return Err(eyre!("path {} is missing from disk", path.display()));
        }

        let content = fs::read_to_string(path)?;

        hasher.update(&content);

        files.insert(path.clone(), content);
    }

    let raw_hash: [u8; 20] = hasher.finalize().into();

    let header = CommitHeader {
        author,
        message,
        hash: raw_hash.into(),
        timestamp: now
    };

    let commit = Commit {
        header,
        files
    };

    if let Some(new_branch) = args.branch {
        if let Some(previous_hash) = repo.branches.get(&new_branch) {
            println!("Branch {new_branch} has moved: {} -> {}", previous_hash.shorten(), commit.hash().shorten());
        }

        let before = repo
            .current_branch()
            .map(String::from)
            .unwrap_or(repo.current_hash.shorten());

        repo.append_commit_on_branch(commit, new_branch.clone())?;

        println!("Switched branches: {before} -> {new_branch}");
    }
    else {
        repo.append_commit(commit)?;
    }

    repo.save()?;

    println!("New version: {:?}", repo.current_hash);
    
    Ok(())
}