use std::{collections::BTreeMap, env::current_dir, fs::{self, File}, io::{self, Write}, path::{Path, PathBuf}};

use crate::{backend::{commit::{Commit, CommitHeader}, hash::{CommitHash, ROOT_HASH_STR}, repository::Repository, tree::Tree}, io::info::{ProjectInfo, ProjectInfoError}};

use chrono::Local;
use eyre::Context;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
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
    Deserialise(#[from] rmp_serde::decode::Error),

    #[error("failed to find a repository")]
    NoRepositoryFound
}

fn locate_root_dir(from: &Path) -> eyre::Result<Option<PathBuf>> {
    let absolute = from.canonicalize()?;
    let mut current: &Path = &absolute;

    while !current.join(".asc").is_dir() {
        let Some(parent) = current.parent() else {
            return Ok(None);
        };

        current = parent;
    }

    Ok(Some(current.to_path_buf()))
}

fn get_ignore_matcher(root_dir: &Path) -> eyre::Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root_dir);

    builder.add(".ascignore");

    let matcher = builder.build()
        .wrap_err("failed to build ignore matcher to test file with.")?;

    Ok(matcher)
}

impl Repository {
    pub fn create_new(root: &Path, author: String, project_name: String) -> eyre::Result<Repository> {
        let root_dir = root.canonicalize()?;
        
        // Where everything will live.
        let content_dir = root_dir.join(".asc").to_path_buf();

        if content_dir.exists() {
            return Err(RepositoryError::AlreadyExists(content_dir.display().to_string()))
                .wrap_err("root directory already exists");
        }

        fs::create_dir(&content_dir)?;

        // Where the staged files will go.
        let mut fp = File::create(content_dir.join("index"))?;

        let empty: Vec<String> = vec![];

        fp.write_all(&rmp_serde::to_vec(&empty)?)?;

        // Where each commit's data will go.
        let blobs_dir = content_dir.join("blobs");

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
        commit_history.to_file(&content_dir.join("tree"))?;

        let mut branches = BTreeMap::new();

        branches.insert("main".to_string(), CommitHash::root());

        let info = ProjectInfo {
            project_name,
            current_user: author,
            branches: branches
                .iter()
                .map(|(name, hash)| (name.clone(), hash.to_string()))
                .collect(),
            current_hash: ROOT_HASH_STR.to_string()
        };

        // Where the repository information will go.
        info.to_file(&content_dir.join("info"))?;

        // An ignore file for the repository.
        File::create(root.join(".ascignore"))?;

        Ok(Repository {
            project_name: info.project_name,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            commit_history,
            current_user: info.current_user,
            branches,
            current_hash: CommitHash::root(),
            staged_files: vec![],
        })
    }

    pub fn load() -> eyre::Result<Repository> {
        let root = current_dir()?;

        let Some(root_dir) = locate_root_dir(&root)? else {
            return Err(RepositoryError::NoRepositoryFound).wrap_err_with(|| format!("invalid root directory: {}", root.display()));
        };

        let content_dir = root_dir.join(".asc");

        let info = ProjectInfo::from_file(&content_dir.join("info"))?;

        let branches = info.branches
            .iter()
            .map(|(name, hash)| (name.clone(), CommitHash::from(hash.as_str())))
            .collect();
        
        let commit_history = Tree::from_file(&content_dir.join("tree"))?;

        let fp = File::open(content_dir.join("index"))?;
        let raw_staged_files: Vec<String> = rmp_serde::from_read(fp)?;

        let repo = Repository {
            project_name: info.project_name,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            commit_history,
            branches,
            current_hash: CommitHash::from(info.current_hash.as_str()),
            current_user: info.current_user,
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
            current_user: self.current_user.clone(),
            branches: self.branches
                .iter()
                .map(|(name, hash)| (name.clone(), hash.to_string()))
                .collect(),
            current_hash: self.current_hash.to_string()
        };

        let content_dir = self.root_dir.join(".asc");

        info.to_file(&content_dir.join("info"))?;

        self.commit_history.to_file(&content_dir.join("tree"))?;

        let mut index: Vec<String> = vec![];
        let cwd = Path::new(".").canonicalize()?;
        
        for path in &self.staged_files {
            let full = path.canonicalize()?;

            let relative = pathdiff::diff_paths(full, &cwd);

            if let Some(rel) = relative {
                index.push(rel.to_string_lossy().to_string())
            }
        }

        let bytes = rmp_serde::to_vec(&index)?;

        let mut fp = File::create(content_dir.join("index"))?;

        fp.write_all(&bytes)?;
        
        Ok(())
    }
}