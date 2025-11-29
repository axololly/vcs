use std::{collections::HashMap, env::current_dir, fs, io::Write, path::{Path, PathBuf}};

use crate::{backend::{action::ActionHistory, graph::Graph, hash::ObjectHash, repository::Repository, snapshot::Snapshot, trash::Trash}, io::info::ProjectInfo, unwrap, utils::{compress_data, create_file, decompress_data, hash_raw_bytes, open_file, remove_path}};

use chrono::Local;
use eyre::{Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use sha1::{Digest, Sha1};

fn locate_root_dir(from: impl AsRef<Path>) -> Result<Option<PathBuf>> {
    let absolute = from.as_ref().canonicalize()?;
    let mut current: &Path = &absolute;

    while !current.join(".asc").is_dir() {
        let Some(parent) = current.parent() else {
            return Ok(None);
        };

        current = parent;
    }

    Ok(Some(current.to_path_buf()))
}

fn get_ignore_matcher(root_dir: impl AsRef<Path>) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root_dir);

    builder.add(".ascignore");

    let matcher = unwrap!(builder.build(), "failed to build ignore matcher");

    Ok(matcher)
}

impl Repository {
    pub fn create_new(root: impl AsRef<Path>, author: String, project_name: String) -> Result<Repository> {
        let root_dir = root.as_ref().canonicalize()?;
        
        // Where everything will live.
        let content_dir = root_dir.join(".asc").to_path_buf();

        if content_dir.exists() {
            bail!("root directory {} already exists", content_dir.display());
        }

        fs::create_dir(&content_dir)?;

        // Where the staged files will go.
        let mut fp = create_file(content_dir.join("index"))?;

        let empty: Vec<String> = vec![];

        fp.write_all(&rmp_serde::to_vec(&empty)?)?;

        // Where the action history will go.
        let mut fp = create_file(content_dir.join("history"))?;

        let action_history = ActionHistory::new();

        fp.write_all(&rmp_serde::to_vec(&action_history)?)?;

        // Where each snapshot's data will go.
        let blobs_dir = content_dir.join("blobs");

        fs::create_dir(&blobs_dir)?;

        // Create the 0-255 thing that Git has
        for x in 0 ..= u8::MAX {
            let prefix = hex::encode([x]);

            fs::create_dir(blobs_dir.join(prefix))?;
        }

        let mut history = Graph::empty();

        history.insert_orphan(ObjectHash::root());

        // Where the snapshot history will go.
        history.to_file(content_dir.join("tree"))?;

        let mut branches = HashMap::new();

        branches.insert("main".to_string(), ObjectHash::root());

        let info = ProjectInfo {
            project_name,
            current_user: author.clone(),
            branches: branches.clone(),
            current_hash: ObjectHash::root(),
            stashes: vec![]
        };

        // Where the repository information will go.
        info.to_file(content_dir.join("info"))?;

        // An ignore file for the repository.
        create_file(root_dir.join(".ascignore"))?;

        let repo = Repository {
            project_name: info.project_name,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history,
            history,
            current_user: info.current_user,
            branches,
            current_hash: ObjectHash::root(),
            staged_files: vec![],
            stashes: vec![],
            trash: Trash::new()
        };

        let root_snapshot = Snapshot {
            author: author.to_string(),
            hash: ObjectHash::root(),
            message: "initial snapshot".to_string(),
            timestamp: Local::now(),
            files: HashMap::new()
        };

        repo.save_snapshot(&root_snapshot)?;

        Ok(repo)
    }

    pub fn load() -> Result<Repository> {
        let root = current_dir()?;

        let Some(root_dir) = locate_root_dir(&root)? else {
            bail!("invalid root directory: {}", root.display());
        };

        let content_dir = root_dir.join(".asc");

        let info = ProjectInfo::from_file(content_dir.join("info"))?;
        
        let history = Graph::from_file(content_dir.join("tree"))?;

        let fp = open_file(content_dir.join("index"))?;
        let staged_files: Vec<PathBuf> = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("history"))?;
        let action_history = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("trash"))?;
        let trash = Trash {
            entries: rmp_serde::from_read(fp)?
        };

        let repo = Repository {
            project_name: info.project_name,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history,
            history,
            branches: info.branches,
            current_hash: info.current_hash,
            current_user: info.current_user,
            staged_files,
            stashes: info.stashes,
            trash
        };

        Ok(repo)
    }

    pub fn save(&self) -> Result<()> {
        let info = ProjectInfo {
            project_name: self.project_name.clone(),
            current_user: self.current_user.clone(),
            branches: self.branches.clone(),
            current_hash: self.current_hash,
            stashes: self.stashes.clone()
        };

        let content_dir = self.root_dir.join(".asc");

        info.to_file(content_dir.join("info"))?;

        self.history.to_file(content_dir.join("tree"))?;

        let mut index: Vec<PathBuf> = vec![];
        let cwd = Path::new(".").canonicalize()?;
        
        for path in &self.staged_files {
            let full = path.canonicalize()?;

            let relative = pathdiff::diff_paths(full, &cwd);

            if let Some(rel) = relative {
                index.push(rel);
            }
        }

        let mut fp = create_file(content_dir.join("index"))?;

        fp.write_all(&rmp_serde::to_vec(&index)?)?;

        let mut fp = create_file(content_dir.join("history"))?;

        fp.write_all(&rmp_serde::to_vec(&self.action_history)?)?;

        let mut fp = open_file(content_dir.join("trash"))?;
        
        fp.write_all(&rmp_serde::to_vec(&self.trash.entries)?)?;
        
        Ok(())
    }
}

impl Repository {
    pub fn main_dir(&self) -> PathBuf {
        self.root_dir.join(".asc")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.main_dir().join("blobs")
    }
    
    pub fn hash_to_path(&self, hash: ObjectHash) -> PathBuf {
        let full = hash.full();

        let (dir, rest) = full.split_at(2);

        self.blobs_dir()
            .join(dir)
            .join(rest)
    }
    
    pub fn fetch_string_content(&self, content_hash: ObjectHash) -> Result<String> {
        let path = self.hash_to_path(content_hash);
        
        let raw = unwrap!(
            fs::read(&path),
            "failed to read bytes from: {}", path.display()
        );

        let decompressed = decompress_data(&raw)?;

        let content = unwrap!(
            String::from_utf8(decompressed),
            "invalid utf8 in path: {}", path.display()
        );

        Ok(content)
    }

    pub fn fetch_snapshot(&self, snapshot_hash: ObjectHash) -> Result<Snapshot> {
        let path = self.hash_to_path(snapshot_hash);
        
        let fp = open_file(path)?;

        Ok(rmp_serde::from_read(fp)?)
    }

    pub fn fetch_current_snapshot(&self) -> Result<Snapshot> {
        self.fetch_snapshot(self.current_hash)
    }

    pub fn save_string_content(&self, content: &str) -> Result<ObjectHash> {
        let hash = hash_raw_bytes(content);

        let path = self.hash_to_path(hash);

        unwrap!(
            fs::write(&path, compress_data(content)),
            "failed to write string {content:?} to: {}", path.display()
        );

        Ok(hash)
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let bytes = rmp_serde::to_vec(snapshot)?;
        
        let path = self.hash_to_path(snapshot.hash);

        unwrap!(
            fs::write(&path, &bytes),
            "failed to write snapshot to: {}", path.display()
        );

        Ok(())
    }

    pub fn snapshot_from_paths(&self, paths: Vec<PathBuf>, author: String, message: String) -> Result<Snapshot> {
        let mut files = HashMap::new();
        
        let mut snapshot_hasher = Sha1::new();

        for path in paths {
            let content = fs::read_to_string(&path)?;

            let hash = self.save_string_content(&content)?;

            snapshot_hasher.update(*hash);

            files.insert(path, hash);
        }

        let raw_snapshot_hash: [u8; 20] = snapshot_hasher.finalize().into();

        let snapshot = Snapshot {
            hash: raw_snapshot_hash.into(),
            author,
            message,
            timestamp: Local::now(),
            files
        };
        
        Ok(snapshot)
    }
}

impl Repository {
    pub fn cwd_differs_from_current(&self) -> Result<bool> {
        let latest_snapshot = self.fetch_current_snapshot()?;

        for path in &self.staged_files {
            if !path.exists() {
                return Ok(true);
            }

            let current_content = unwrap!(
                fs::read_to_string(path),
                "failed to read path: {}", path.display()
            );

            let current_content_hash = hash_raw_bytes(&current_content);

            let Some(&previous_content_hash) = latest_snapshot.files.get(path) else {
                return Ok(true)
            };

            if previous_content_hash != current_content_hash {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    pub fn replace_cwd_with_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        if self.cwd_differs_from_current()? {
            bail!("cannot change snapshots with unsaved changes.");
        }

        let current = self.fetch_current_snapshot()?;

        // Delete paths that are in this snapshot but not the destination snapshot.
        for path in current.files.keys() {
            if !snapshot.files.contains_key(path) {
                remove_path(path, &self.root_dir)?;
            }
        }

        for (path, &new) in &snapshot.files {
            // File exists in both - if the hashes are different, replace the content.
            if let Some(&old) = current.files.get(path) && old == new {
                continue;
            }

            let content = self.fetch_string_content(new)?;

            unwrap!(
                fs::write(path, content),
                "failed to write to path: {}", path.display()
            );
        }

        Ok(())
    }
}