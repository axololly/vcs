use std::{collections::BTreeMap, io, path::PathBuf};

use crate::backend::{commit::Commit, hash::CommitHash, tree::Tree};

use eyre::Context;
use thiserror::Error;

pub struct Repository {
    pub project_name: String,
    pub root_dir: PathBuf,
    pub commit_history: Tree,
    pub branches: BTreeMap<String, CommitHash>,
    pub current_hash: CommitHash,
    pub staged_files: Vec<PathBuf>
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("invalid branch name: {0}")]
    InvalidBranchName(String),

    #[error("failed interaction with disk: {0}")]
    IO(#[from] io::Error)
}

impl Repository {
    pub fn is_head_detached(&self) -> bool {
        // Head is detached when it doesn't point to any branch's hash.
        !self.branches.values().any(|&v| v == self.current_hash)
    }

    pub fn switch_branch(&mut self, new_branch: &str) -> eyre::Result<()> {
        self.branches
            .get(new_branch)
            .map(|&new_hash| { self.current_hash = new_hash; })
            .ok_or(RepositoryError::InvalidBranchName(new_branch.to_string()))
            .wrap_err_with(|| format!("branch name {new_branch:?} does not exist"))
    }

    pub fn append_commit(&mut self, commit: Commit) -> eyre::Result<()> {
        self.commit_history.insert(commit.hash(), self.current_hash);

        self.current_hash = commit.hash();

        let commit_path = self.root_dir.join("blobs").join(self.current_hash.to_string());

        commit.to_file(&commit_path)?;
        
        Ok(())
    }
}