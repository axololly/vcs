use std::{collections::{BTreeMap, HashMap, HashSet}, env::current_dir, fs, path::{Path, PathBuf}};

use crate::{backend::{action::ActionHistory, change::FileChange, content::{Content, Delta, RawContent}, graph::Graph, hash::ObjectHash, repository::Repository, snapshot::Snapshot, trash::Trash, user::{get_random_password, Permissions, Users}}, io::info::ProjectInfo, unwrap, utils::{compress_data, create_file, hash_raw_bytes, open_file, remove_path, save_as_msgpack}};

use chrono::Local;
use eyre::{Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use similar::TextDiff;

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
    /// Create a new repository in a given directory.
    /// 
    /// This currently requires that:
    /// - `root` is a directory
    /// - `root` does not contain a folder called `.asc` (where the repository lives)
    /// 
    /// This returns the [`Repository`] that was created.
    pub fn create_new(root: impl AsRef<Path>, author: String, project_name: String) -> Result<Repository> {
        let root_dir = root.as_ref().canonicalize()?;

        if !root_dir.is_dir() {
            bail!("{} is not a directory.", root_dir.display());
        }
        
        let content_dir = root_dir.join(".asc");

        if content_dir.exists() && content_dir.is_dir() {
            bail!("root directory {} already contains a repository. Remove it and rerun the command.", root_dir.display());
        }

        let blobs_dir = content_dir.join("blobs");

        for x in 0 ..= u8::MAX {
            let label = blobs_dir.join(hex::encode([x]));

            fs::create_dir_all(label)?;
        }

        create_file(root_dir.join(".ascignore"))?;

        let mut branches = HashMap::new();

        branches.insert("main".to_string(), ObjectHash::root());

        let mut users = Users::new();

        users.create_user_with_permissions(
            &author,
            &get_random_password(10),
            Permissions::all()
        )?;

        users.create_user(
            "everyone",
            &get_random_password(10)
        )?;

        let now = Local::now().timestamp();

        let project_code = hash_raw_bytes(now.to_le_bytes());

        let repo = Repository {
            project_name: project_name,
            project_code,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history: ActionHistory::new(),
            history: Graph::new(),
            branches,
            current_hash: ObjectHash::root(),
            current_username: author.clone(),
            staged_files: vec![],
            stashes: vec![],
            trash: Trash::new(),
            tags: HashMap::new(),
            users
        };

        let root_snapshot = Snapshot {
            author,
            hash: ObjectHash::root(),
            message: "initial snapshot".to_string(),
            timestamp: Local::now(),
            files: BTreeMap::new()
        };

        repo.save_snapshot(&root_snapshot)?;

        repo.save()?;

        Ok(repo)
    }

    /// Load the repository in the current directory, searching
    /// upwards from the current working directory until a directory
    /// containing an `.acs` directory is found.
    pub fn load() -> Result<Repository> {
        let start = current_dir()?;

        let Some(root_dir) = locate_root_dir(&start)? else {
            bail!("no .acs directory found when searching recursively from: {}", start.display());
        };

        let content_dir = root_dir.join(".asc");

        let info = ProjectInfo::from_file(content_dir.join("info"))?;
        
        let history = Graph::from_file(content_dir.join("tree"))?;

        let fp = open_file(content_dir.join("index"))?;
        let staged_files: Vec<PathBuf> = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("history"))?;
        let action_history = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("trash"))?;
        let trash = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("tags"))?;
        let tags: HashMap<String, ObjectHash> = rmp_serde::from_read(fp)?;

        let fp = open_file(content_dir.join("users"))?;
        let users: Users = rmp_serde::from_read(fp)?;

        let repo = Repository {
            project_name: info.project_name,
            project_code: info.project_code,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history,
            history,
            branches: info.branches,
            current_hash: info.current_hash,
            current_username: info.current_user,
            staged_files,
            stashes: info.stashes,
            trash,
            tags,
            users
        };

        Ok(repo)
    }

    /// Save the current state of the repository to disk.
    pub fn save(&self) -> Result<()> {
        let info = ProjectInfo {
            project_name: self.project_name.clone(),
            project_code: self.project_code,
            current_user: self.current_username.clone(),
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
            let full = unwrap!(
                path.canonicalize(),
                "failed to canonicalise path: {}",
                path.display()
            );

            let relative = pathdiff::diff_paths(full, &cwd).unwrap();

            index.push(relative);
        }

        save_as_msgpack(&index, content_dir.join("index"))?;

        save_as_msgpack(&self.action_history, content_dir.join("history"))?;
        
        save_as_msgpack(&self.trash, content_dir.join("trash"))?;

        save_as_msgpack(&self.tags, content_dir.join("tags"))?;

        let users_dir = content_dir.join("users");

        save_as_msgpack(&self.users, users_dir.join("credentials"))?;

        Ok(())
    }
}

pub static MIN_DELTA_SIMILARITY: f32 = 0.65;

impl Repository {
    /// Get the directory the repository operates in.
    pub fn main_dir(&self) -> PathBuf {
        self.root_dir.join(".asc")
    }

    /// Get the directory where data in the repository is stored.
    /// 
    /// Fundamentally identical to `.git/objects`.
    pub fn blobs_dir(&self) -> PathBuf {
        self.main_dir().join("blobs")
    }
    
    /// Convert an [`ObjectHash`] to its location on disk.
    pub fn hash_to_path(&self, hash: ObjectHash) -> PathBuf {
        let full = hash.full();

        let (dir, rest) = full.split_at(2);

        self.blobs_dir()
            .join(dir)
            .join(rest)
    }
    
    /// Fetch a `String` from the repository, addressed by its hash.
    pub fn fetch_string_content(&self, content_hash: ObjectHash) -> Result<Content> {
        let path = self.hash_to_path(content_hash);
        
        let raw = unwrap!(
            fs::read(&path),
            "failed to read bytes from: {}", path.display()
        );

        let raw_content: RawContent = rmp_serde::from_slice(&raw)?;

        raw_content.into_content()
    }

    /// Fetch a [`Snapshot`] from the repository, addressed by its hash.
    pub fn fetch_snapshot(&self, snapshot_hash: ObjectHash) -> Result<Snapshot> {
        let path = self.hash_to_path(snapshot_hash);
        
        let fp = open_file(path)?;

        Ok(rmp_serde::from_read(fp)?)
    }

    /// Fetch the [`Snapshot`] the HEAD is currently on from the repository.
    pub fn fetch_current_snapshot(&self) -> Result<Snapshot> {
        self.fetch_snapshot(self.current_hash)
    }

    /// Save a string to disk with optional delta compression if `basis` is provided
    /// and the basis is similar enough to `content` (determined by [`MIN_DELTA_SIMILARITY`]).
    pub fn save_content(&self, content: &str, basis: Option<ObjectHash>) -> Result<ObjectHash> {
        let Some(basis) = basis else {
            return self.save_content_raw(content);
        };

        let Some(hash) = self.save_content_delta(content, basis)? else {
            return self.save_content_raw(content);
        };

        Ok(hash)
    }

    /// Save a string as a compressed blob to disk and return the hash used to load it.
    pub fn save_content_raw(&self, content: &str) -> Result<ObjectHash> {
        let hash = hash_raw_bytes(content);

        let path = self.hash_to_path(hash);

        unwrap!(
            fs::write(&path, compress_data(content)),
            "failed to write string {content:?} to: {}", path.display()
        );

        Ok(hash)
    }

    /// Save a string as a delta of some other string on disk, but reject the delta
    /// if the two strings have a similarity lower than [`MIN_DELTA_SIMILARITY`].
    pub fn save_content_delta(&self, content: &str, basis: ObjectHash) -> Result<Option<ObjectHash>> {
        let original = self.fetch_string_content(basis)?.resolve(self)?;

        let diff = TextDiff::from_words(original.as_str(), content);

        if diff.ratio() < MIN_DELTA_SIMILARITY {
            return Ok(None);
        }

        let hash = self.save_content_delta_unchecked(content, basis)?;

        Ok(Some(hash))
    }

    /// Save a string as a delta of some other string on disk, regardless of the similarity
    /// of the two strings.
    /// 
    /// For a method that considers similarity, use the safer [`Repository::save_content_delta`],
    /// or the higher-level [`Repository::save_content`].
    pub fn save_content_delta_unchecked(&self, content: &str, basis: ObjectHash) -> Result<ObjectHash> {
        let base = self.fetch_string_content(basis)?;

        let original = base.resolve(self)?;

        let delta = Delta::new_unchecked(&original, content);

        let hash = hash_raw_bytes(content);

        let path = self.hash_to_path(hash);

        save_as_msgpack(&delta, path)?;

        Ok(hash)
    }

    /// Save a snapshot as a compressed blob to disk.
    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let path = self.hash_to_path(snapshot.hash);

        save_as_msgpack(snapshot, path)
    }

    /// Assemble a [`Snapshot`] from the repository's tracked files.
    /// 
    /// This saves the [`Snapshot`] and its files' contents to disk before returning.
    pub fn capture_current_state(&self, author: String, message: String) -> Result<Snapshot> {
        let base_files = self.fetch_current_snapshot()?.files;

        let mut files = BTreeMap::new();
        
        for path in &self.staged_files {
            let content = fs::read_to_string(&path)?;

            let hash = self.save_content(&content, base_files.get(path).cloned())?;

            files.insert(path.clone(), hash);
        }

        let snapshot = Snapshot::from_parts(
            author,
            message,
            Local::now(),
            files
        );

        self.save_snapshot(&snapshot)?;
        
        Ok(snapshot)
    }
}

impl Repository {
    fn cwd_differs_from_snapshot(&self, snapshot: &Snapshot) -> Result<bool> {
        for path in &self.staged_files {
            if !path.exists() {
                return Ok(true);
            }

            let current_content = unwrap!(
                fs::read_to_string(path),
                "failed to read path: {}", path.display()
            );

            let current_content_hash = hash_raw_bytes(&current_content);

            let Some(&previous_content_hash) = snapshot.files.get(path) else {
                return Ok(true)
            };

            if previous_content_hash != current_content_hash {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Check if the repository has unsaved changes.
    /// 
    /// This checks both the current snapshot and any
    /// snapshots in the stash to ensure data is safe.
    pub fn has_unsaved_changes(&self) -> Result<bool> {
        let current = self.fetch_current_snapshot()?;

        // If the CWD matches the current snapshot,
        // no changes are made, and content is safe.
        if !self.cwd_differs_from_snapshot(&current)? {
            return Ok(false);
        }

        // If the CWD matches a snapshot in the stash,
        // no changes are made, and content is safe.
        for hash in self.stashes.iter().map(|s| s.snapshot) {
            let snapshot = self.fetch_snapshot(hash)?;

            if !self.cwd_differs_from_snapshot(&snapshot)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Replace the state of the current working directory with that
    /// from another [`Snapshot`].
    /// 
    /// This is used to switch the repository to a different version,
    /// and will fail if there are unsaved changes.
    pub fn replace_cwd_with_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        if self.has_unsaved_changes()? {
            bail!("cannot change snapshots with unsaved changes.");
        }

        self.replace_cwd_with_snapshot_unchecked(snapshot)
    }

    /// Replace the state of the current working directory with that
    /// from another [`Snapshot`], but **DO NOT** check if there are
    /// unsaved changes.
    /// 
    /// For a safer alternative, use [`Repository::replace_cwd_with_snapshot`].
    pub fn replace_cwd_with_snapshot_unchecked(&self, snapshot: &Snapshot) -> Result<()> {
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
                fs::write(path, content.resolve(self)?),
                "failed to write to path: {}", path.display()
            );
        }

        Ok(())
    }

    /// List all the changes as [`FileChange`] objects between
    /// the current snapshot and the current working directory.
    pub fn list_changes(&self) -> Result<Vec<FileChange>> {
        let old_files = self.fetch_current_snapshot()?.files;

        let old_paths: HashSet<PathBuf> = old_files
            .keys()
            .cloned()
            .collect();

        let new_paths: HashSet<PathBuf> = self.staged_files
            .iter()
            .cloned()
            .collect();

        let mut file_changes: Vec<FileChange> = vec![];

        file_changes.extend(
            new_paths
                .difference(&old_paths)
                .map(|p| FileChange::Added(p.clone()))
        );

        file_changes.extend(
            old_paths
                .difference(&new_paths)
                .map(|p| FileChange::Removed(p.clone()))
        );

        file_changes.extend(
            new_paths
                .iter()
                .filter_map(|p| (!p.exists()).then_some(FileChange::Missing(p.clone())))
        );

        for (path, hash) in old_files {
            if !path.exists() {
                continue;
            }

            let content = fs::read_to_string(&path)?;

            let content_hash = hash_raw_bytes(&content);
            
            if hash == content_hash {
                file_changes.push(FileChange::Unchanged(path));
            }
            else {
                file_changes.push(FileChange::Edited(path));
            }
        }

        Ok(file_changes)
    }
}