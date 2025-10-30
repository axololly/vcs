use std::{collections::BTreeMap, fs::{self, File}, io, path::{Path, PathBuf}};

use crate::{backend::{commit::{Commit, CommitHeader}, hash::{CommitHash, ROOT_HASH_STR}, repository::Repository, tree::Tree}, io::info::{ProjectInfo, ProjectInfoError}};

use chrono::Local;
use eyre::Context;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("root directory already exists: {0}")]
    AlreadyExists(String),

    #[error("failed interaction with disk: {0}")]
    IO(#[from] io::Error),

    #[error("failed operation with ProjectInfo: {0}")]
    ProjectInfo(#[from] ProjectInfoError),

    #[error("failed to deserialise data: {0}")]
    Deserialise(#[from] rmp_serde::decode::Error)
}

impl Repository {
    pub fn open(root: &Path, author: &str, project_name: &str) -> eyre::Result<Repository> {
        // Where everything will live.
        let root_dir = root.to_path_buf();

        if !root_dir.exists() {
            return Err(RepositoryError::AlreadyExists(root.display().to_string()))
                .wrap_err("root directory already exists");
        }

        fs::create_dir(&root_dir)?;

        // Where the staged files will go.
        File::create(root_dir.join("index"))?;

        // Where each commit's data will go.
        let blobs_dir = root_dir.join("blobs");

        fs::create_dir(&blobs_dir)?;

        // Put the root commit in there.
        let root_commit = Commit {
            header: CommitHeader {
                author: author.to_string(),
                hash: CommitHash::root(),
                message: "Initial commit.".to_string(),
                timestamp: Local::now()
            },
            files: BTreeMap::new()
        };

        root_commit.to_file(&blobs_dir.join(ROOT_HASH_STR))?;

        let mut commit_history = Tree::empty();

        commit_history.insert_orphan(CommitHash::root());

        // Where the commit history will go.
        commit_history.to_file(&root.join("tree"))?;

        let mut branches = BTreeMap::new();

        branches.insert("main".to_string(), CommitHash::root());

        let info = ProjectInfo {
            project_name: project_name.to_string(),
            branches: branches
                .iter()
                .map(|(name, hash)| (name.clone(), hash.to_string()))
                .collect(),
            current_hash: ROOT_HASH_STR.to_string()
        };

        // Where the repository information will go.
        info.to_file(&root_dir.join("info"))?;

        Ok(Repository {
            project_name: project_name.to_string(),
            root_dir,
            commit_history,
            branches,
            current_hash: CommitHash::root(),
            staged_files: vec![]
        })
    }

    pub fn load(root: &Path) -> eyre::Result<Repository> {
        let info = ProjectInfo::from_file(&root.join("info"))?;

        let branches = info.branches
            .iter()
            .map(|(name, hash)| (name.clone(), CommitHash::from(hash.as_str())))
            .collect();
        
        let commit_history = Tree::from_file(&root.join("tree"))?;

        let fp = File::open(root.join("index"))?;
        let raw_staged_files: Vec<String> = rmp_serde::from_read(fp)?;

        let repo = Repository {
            project_name: info.project_name,
            root_dir: root.to_path_buf(),
            commit_history,
            branches,
            current_hash: CommitHash::from(info.current_hash.as_str()),
            staged_files: raw_staged_files
                .iter()
                .map(PathBuf::from)
                .collect()
        };

        Ok(repo)
    }

    pub fn save(&self) -> eyre::Result<()> {
        let info = ProjectInfo {
            project_name: self.project_name.clone(),
            branches: self.branches
                .iter()
                .map(|(name, hash)| (name.clone(), hash.to_string()))
                .collect(),
            current_hash: self.current_hash.to_string()
        };

        info.to_file(&self.root_dir.join("info"))?;

        self.commit_history.to_file(&self.root_dir.join("tree"))?;

        let mut index: Vec<String> = vec![];
        let cwd = Path::new(".").canonicalize()?;
        
        for path in &self.staged_files {
            let full = path.canonicalize()?;

            let relative = pathdiff::diff_paths(full, &cwd);

            if let Some(rel) = relative {
                index.push(rel.to_string_lossy().to_string())
            }
        }

        fs::write(self.root_dir.join("index"), index.join("\n"))?;
        
        Ok(())
    }
}