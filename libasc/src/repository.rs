use std::{cell::RefCell, collections::{BTreeMap, HashMap, HashSet}, env::current_dir, fs, io::Write, path::{Path, PathBuf}};

use crate::{change::FileChange, compress_data, content::{Content, Delta}, action::{Action, ActionHistory}, graph::Graph, hash::ObjectHash, snapshot::Snapshot, stash::Stash, trash::{Entry, Trash, TrashStatus}, user::{User, Users}, create_file, hash_raw_bytes, open_file, remove_path, save_as_msgpack, snapshot::SignedSnapshot, unwrap, user::Permissions};

use chrono::Utc;
use expand_tilde::ExpandTilde;
use eyre::{Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::{Deserialize, Serialize};
use similar::TextDiff;

pub struct Repository {
    pub project_name: String,
    pub project_code: ObjectHash,
    pub root_dir: PathBuf,
    pub history: Graph,
    pub action_history: ActionHistory,
    pub branches: HashMap<String, ObjectHash>,
    pub current_hash: ObjectHash,
    pub staged_files: Vec<PathBuf>,
    pub ignore_matcher: Gitignore,
    pub stashes: Vec<Stash>,
    pub trash: Trash,
    pub tags: HashMap<String, ObjectHash>,
    pub users: Users,

    current_username: RefCell<Option<String>>
}

impl Repository {
    /// Get the current user of the repository as a [`User`].
    /// 
    /// If the current user has any issues preventing its use,
    /// it will automatically be reset to `None`. These are if:
    /// 
    /// - the user doesn't exist
    /// - the user has no associated private key
    /// - the user's account is marked as closed
    pub fn current_user(&self) -> Option<&User> {
        let username = self.current_username.take()?;

        let user = self.users.get_user(&username)?;

        if user.closed || user.private_key.is_none() {
            return None;
        }

        self.current_username.replace(Some(username));

        Some(user)
    }

    /// Get the current user of the repository as a mutable [`User`].
    /// 
    /// If the current user has any issues preventing its use,
    /// it will automatically be reset to `None`. These are if:
    /// 
    /// - the user doesn't exist
    /// - the user has no associated private key
    /// - the user's account is marked as closed
    pub fn current_user_mut(&mut self) -> Option<&mut User> {
        let username = self.current_username.take()?;

        let user = self.users.get_user_mut(&username)?;

        if user.closed || user.private_key.is_none() {
            return None;
        }

        self.current_username.replace(Some(username));

        Some(user)
    }

    /// Change the current user of the repository.
    pub fn set_current_user(&mut self, username: &str) -> Result<()> {
        let user = unwrap!(
            self.users.get_user(username),
            "no user with name {username:?} exists in the repository."
        );

        if user.closed {
            bail!("cannot switch to closed user {username:?}");
        }

        if user.private_key.is_none() {
            bail!("cannot switch to user {username:?} (no private key)");
        }

        self.current_username.replace(Some(username.to_string()));

        Ok(())
    }

    /// Get the branch the repository is currently on.
    /// 
    /// This is only found if the `current_hash` points to a branch tip.
    /// Any other snapshot will result in `None`.
    pub fn current_branch(&self) -> Option<&str> {
        self.branch_from_hash(self.current_hash)
    }

    /// Get the name of a branch from its hash.
    /// 
    /// This is only found if the hash points to a branch tip.
    /// Any other snapshot will result in `None`.
    pub fn branch_from_hash(&self, commit_hash: ObjectHash) -> Option<&str> {
        self.branches
            .iter()
            .filter_map(|(name, &hash)| {
                if hash == commit_hash {
                    Some(name.as_str())
                }
                else {
                    None
                }
            })
            .next()
    }

    /// Find if the `current_hash` doesn't point to the tip of any branch.
    pub fn is_head_detached(&self) -> bool {
        self.current_branch().is_none()
    }

    fn append_snapshot_internal(&mut self, snapshot: Snapshot, branch_name: Option<String>) -> Result<()> {
        if !self.has_unsaved_changes()? {
            bail!("no changes have been made in the current working directory.");
        }

        self.history.insert(snapshot.hash, self.current_hash)?;

        if let Some(name) = branch_name {
            self.branches.insert(name, snapshot.hash);
        }

        self.current_hash = snapshot.hash;
        
        self.save_snapshot(snapshot)?;
        
        Ok(())
    }

    /// Append a snapshot to the tip of the current branch,
    /// moving the branch pointer to point to the added snapshot.
    pub fn append_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        self.append_snapshot_internal(snapshot, self.current_branch().map(String::from))
    }

    /// Append a snapshot to the tip of any branch,
    /// moving that branch's pointer to point to the added snapshot.
    pub fn append_snapshot_to_branch(&mut self, snapshot: Snapshot, branch_name: String) -> Result<()> {
        self.append_snapshot_internal(snapshot, Some(branch_name))
    }

    /// Check if a given path is ignored by the `.ascignore`
    /// file in the repository, if it is present.
    pub fn is_ignored_path(&self, path: &Path) -> bool {
        self.ignore_matcher.matched(path, path.is_dir()).is_ignore()
    }

    fn normalise_hash_internal(&self, iter: impl Iterator<Item = ObjectHash>, needle: &[u8]) -> Option<ObjectHash> {
        for hash in iter {
            let bytes: &[u8] = hash.as_bytes();

            if bytes.starts_with(needle) {
                return Some(hash);
            }
        }

        None
    }

    /// Convert a smaller hash in string form into its full [`ObjectHash`] version.
    /// 
    /// This works only for snapshots. For identifying stash hashes, use
    /// [`Repository::normalise_stash_hash`].
    pub fn normalise_hash(&self, raw_hash: &str) -> Result<ObjectHash> {
        let as_hex = hex::decode(raw_hash)?;

        if as_hex.is_empty() {
            bail!("attempted to normalise empty snapshot hash.");
        }

        let commit_hashes = self
            .history
            .iter_hashes();

        if let Some(normalised) = self.normalise_hash_internal(commit_hashes, &as_hex) {
            Ok(normalised)
        }
        else {
            bail!("could not resolve hash: {raw_hash:?}");
        }
    }

    /// Convert a smaller hash in string form into its full [`ObjectHash`] version.
    /// 
    /// This works only for stashes. For identifying snapshot hashes, use
    /// [`Repository::normalise_hash`].
    pub fn normalise_stash_hash(&self, raw_hash: &str) -> Result<ObjectHash> {
        let as_hex = hex::decode(raw_hash)?;

        if as_hex.is_empty() {
            bail!("attempted to normalise empty stash hash.");
        }

        let stash_ids = self.stashes
            .iter()
            .map(|s| s.snapshot);

        if let Some(normalised) = self.normalise_hash_internal(stash_ids, &as_hex) {
            Ok(normalised)
        }
        else {
            bail!("could not resolve hash: {raw_hash:?}");
        }
    }

    /// Convert a version in string form into its full [`ObjectHash`] version
    /// by trying to interpret it as a branch name, then trying to interpret
    /// it as the hash of a snapshot.
    pub fn normalise_version(&self, raw_version: &str) -> Result<ObjectHash> {
        if let Some(&corresponding_hash) = self.branches.get(raw_version) {
            Ok(corresponding_hash)
        }
        else {
            self.normalise_hash(raw_version)
        }
    }

    fn apply_action(&mut self, action: Action) -> Result<()> {
        use Action::*;

        match action {
            RebaseSnapshot { hash, to, .. } => {
                let previous = self.history.upsert(hash, to);

                if previous.is_none() {
                    bail!("{hash} does not exist in the repository")
                }
            }

            CreateBranch { name, .. } => {
                self.branches.remove(&name);
            }

            DeleteBranch { name, hash } => {
                self.branches.insert(name, hash);
            }

            RenameBranch { hash, old, new } => {
                self.branches.remove(&old);

                self.branches.insert(new, hash);
            }

            SwitchVersion { after, .. } => {
                self.current_hash = after;
            }
        }

        Ok(())
    }

    /// Undo an [`Action`] on the repository, returning the action
    /// if any changes were made.
    pub fn undo_action(&mut self) -> Result<Option<Action>> {
        let Some(action) = self.action_history.undo().cloned() else {
            return Ok(None)
        };

        use Action::*;

        let inverse = match action {
            RebaseSnapshot { hash, from, to } => RebaseSnapshot { hash, from: to, to: from },

            CreateBranch { name, hash } => DeleteBranch { name, hash },
            DeleteBranch { name, hash } => CreateBranch { name, hash },
            RenameBranch { hash, old, new } => RenameBranch { hash, old: new, new: old },

            SwitchVersion { before, after } => SwitchVersion { before: after, after: before },
        };

        self.apply_action(inverse.clone())?;

        Ok(Some(inverse))
    }

    /// Redo an [`Action`] on the repository, returning the action
    /// if any changes were made.
    pub fn redo_action(&mut self) -> Result<Option<Action>> {
        let Some(action) = self.action_history.redo().cloned() else {
            return Ok(None)
        };

        self.apply_action(action.clone())?;

        Ok(Some(action))
    }

    /// Check if an [`ObjectHash`] of a snapshot is included in the trash.
    pub fn trash_contains(&self, hash: ObjectHash) -> Option<TrashStatus> {
        if self.trash.contains(hash) {
            return Some(TrashStatus::Direct);
        }

        for Entry { hash: trash_hash, .. } in self.trash.entries() {
            if self.history.is_descendant(hash, *trash_hash).unwrap() {
                return Some(TrashStatus::Indirect(*trash_hash));
            }
        }

        None
    }
}

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

#[derive(Deserialize, Serialize)]
pub struct ProjectInfo {
    pub project_name: String,
    pub project_code: ObjectHash,
    pub current_user: Option<String>,
    pub branches: HashMap<String, ObjectHash>,
    pub current_hash: ObjectHash,
    pub stashes: Vec<Stash>
}

impl ProjectInfo {
    pub fn from_file(path: impl AsRef<Path>) -> Result<ProjectInfo> {
        let fp = open_file(path)?;

        let info = rmp_serde::from_read(fp)?;

        Ok(info)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> eyre::Result<()> {
        let bytes = rmp_serde::to_vec(self)?;
        
        let mut fp = create_file(path)?;

        fp.write_all(&bytes)?;

        Ok(())
    }
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
            bail!("root directory {} already contains a repository", root_dir.display());
        }

        let blobs_dir = content_dir.join("blobs");

        for x in 0 ..= u8::MAX {
            let label = blobs_dir.join(hex::encode([x]));

            fs::create_dir_all(label)?;
        }

        create_file(root_dir.join(".ascignore"))?;

        let mut branches = HashMap::new();

        branches.insert("main".to_string(), ObjectHash::root());

        let now = Utc::now().timestamp();

        let project_code = hash_raw_bytes(now.to_le_bytes());
        
        let mut users = Users::new();

        users.create_user_with_permissions(
            author.clone(),
            Permissions::all()
        )?;

        let repo = Repository {
            project_name,
            project_code,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history: ActionHistory::new(),
            history: Graph::new(),
            branches,
            current_hash: ObjectHash::root(),
            current_username: Some(author.clone()).into(),
            staged_files: vec![],
            stashes: vec![],
            trash: Trash::new(),
            tags: HashMap::new(),
            users
        };

        let mut root_snapshot = Snapshot::new(
            author,
            "initial snapshot".to_string(),
            Utc::now(),
            BTreeMap::new()
        );

        root_snapshot.hash = ObjectHash::root();

        repo.save_snapshot(root_snapshot)?;

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

        Repository::load_from(root_dir)
    }

    /// Load the repository from a given directory.
    /// 
    /// This does **NOT** search upwards for a valid directory, and will simply fail.
    pub fn load_from(root_dir: impl AsRef<Path>) -> Result<Repository> {
        let root_dir = {
            let base = root_dir.as_ref().expand_tilde()?;
            
            unwrap!(
                base.canonicalize(),
                "could not canonicalise path: {}", base.display()
            )
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
            current_username: info.current_user.into(),
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
            current_user: self.current_username.borrow().clone(),
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

        save_as_msgpack(&self.users, content_dir.join("users"))?;

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

        let content: Content = rmp_serde::from_slice(&raw)?;

        Ok(content)
    }

    /// Fetch a [`Snapshot`] from the repository, addressed by its hash.
    pub fn fetch_snapshot(&self, snapshot_hash: ObjectHash) -> Result<Snapshot> {
        let path = self.hash_to_path(snapshot_hash);
        
        let fp = open_file(path)?;

        let signed_snapshot: SignedSnapshot = rmp_serde::from_read(fp)?;

        signed_snapshot.verify()
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

        let object = Content::Literal(compress_data(content));

        self.save_content_object(object, hash)?;

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

        let hash = hash_raw_bytes(&original);

        let delta = Content::Delta(Delta::new_unchecked(&original, content));

        self.save_content_object(delta, hash)?;

        Ok(hash)
    }

    /// Save a [`Content`] object, most likely obtained from network transfer.
    pub fn save_content_object(&self, object: Content, hash: ObjectHash) -> Result<()> {
        save_as_msgpack(&object, self.hash_to_path(hash))
    }

    /// Save a snapshot as a compressed blob to disk.
    pub fn save_snapshot(&self, snapshot: Snapshot) -> Result<()> {
        let path = self.hash_to_path(snapshot.hash);

        let user = unwrap!(
            self.current_user(),
            "cannot save snapshot under current user"
        );

        let private_key = unwrap!(
            user.private_key.clone(),
            "cannot save a snapshot under user {:?} (no private key)",
            user.name
        );

        let signed = SignedSnapshot::new(snapshot, private_key);

        save_as_msgpack(&signed, path)
    }

    /// Assemble a [`Snapshot`] from the repository's tracked files.
    /// 
    /// This saves the [`Snapshot`] and its files' contents to disk before returning.
    pub fn capture_current_state(&self, author: String, message: String) -> Result<Snapshot> {
        let base_files = self.fetch_current_snapshot()?.files;

        let mut files = BTreeMap::new();
        
        for path in &self.staged_files {
            let content = fs::read_to_string(path)?;

            let hash = self.save_content(&content, base_files.get(path).cloned())?;

            files.insert(path.clone(), hash);
        }

        let snapshot = Snapshot::new(
            author,
            message,
            Utc::now(),
            files
        );

        let hash = snapshot.hash;

        self.save_snapshot(snapshot)?;

        self.fetch_snapshot(hash)
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
    pub fn replace_cwd_with_snapshot(&mut self, snapshot: &Snapshot) -> Result<()> {
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
    pub fn replace_cwd_with_snapshot_unchecked(&mut self, snapshot: &Snapshot) -> Result<()> {
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

        self.staged_files = snapshot
            .files
            .keys()
            .cloned()
            .collect();

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
