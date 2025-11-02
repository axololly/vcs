use std::{collections::BTreeMap, fs::File, io::Read, path::{Path, PathBuf}};

use crate::backend::{commit::{Commit, CommitHeader}, hash::CommitHash, tree::Tree};

use eyre::eyre;
use ignore::gitignore::Gitignore;

pub struct Repository {
    pub project_name: String,
    pub root_dir: PathBuf,
    pub commit_history: Tree,
    pub branches: BTreeMap<String, CommitHash>,
    pub current_hash: CommitHash,
    pub current_user: String,
    pub staged_files: Vec<PathBuf>,
    pub ignore_matcher: Gitignore
}

impl Repository {
    pub fn current_branch(&self) -> Option<&str> {
        self.branches
            .iter()
            .filter_map(|(name, &hash)| {
                if hash == self.current_hash {
                    Some(name.as_str())
                }
                else {
                    None
                }
            })
            .next()
    }

    /// The head is detached when it doesn't point to the tip of any branch.
    pub fn is_head_detached(&self) -> bool {
        self.current_branch().is_none()
    }

    pub fn any_changes_since_latest(&self) -> eyre::Result<bool> {
        let latest_commit = self.fetch_current_commit()?;

        for path in &self.staged_files {
            if !path.exists() {
                return Ok(true);
            }

            let mut fp = File::open(path)?;

            let mut current_content = String::new();
            
            fp.read_to_string(&mut current_content)?;

            let Some(previous_content) = latest_commit.files.get(path) else {
                return Ok(true)
            };

            if previous_content != &current_content {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    fn _append_commit(&mut self, commit: Commit, new_branch_name: Option<String>) -> eyre::Result<()> {
        if !self.any_changes_since_latest()? {
            return Err(eyre!("No changes have been made in the current working directory."));
        }

        self.commit_history.insert(commit.hash(), self.current_hash);

        if let Some(name) = new_branch_name {
            self.branches.insert(name, commit.hash());
        }

        self.current_hash = commit.hash();

        let commit_path = self.root_dir
            .join(".asc")
            .join("blobs")
            .join(commit.hash().to_string());

        commit.to_file(&commit_path)?;
        
        Ok(())
    }

    pub fn append_commit(&mut self, commit: Commit) -> eyre::Result<()> {
        self._append_commit(commit, self.current_branch().map(String::from))
    }

    pub fn append_commit_on_branch(&mut self, commit: Commit, branch_name: String) -> eyre::Result<()> {
        self._append_commit(commit, Some(branch_name))
    }

    pub fn fetch_commit_header(&self, commit_hash: CommitHash) -> eyre::Result<CommitHeader> {
        CommitHeader::from_file(
            &self.root_dir
                .join(".asc")
                .join("blobs")
                .join(commit_hash.to_string())
        )
    }

    pub fn fetch_commit(&self, commit_hash: CommitHash) -> eyre::Result<Commit> {
        Commit::from_file(
            &self.root_dir
                .join(".asc")
                .join("blobs")
                .join(commit_hash.to_string())
        )
    }

    pub fn fetch_current_commit(&self) -> eyre::Result<Commit> {
        self.fetch_commit(self.current_hash)
    }

    pub fn is_ignored_path(&self, path: &Path) -> bool {
        self.ignore_matcher.matched(path, path.is_dir()).is_ignore()
    }
}