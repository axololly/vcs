use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque}, env::current_dir, fs, path::{Path, PathBuf}, str::FromStr, sync::{Arc, RwLock}};

use crate::{action::{Action, ActionHistory}, change::FileChange, compress_data, content::{Content, Delta}, create_file, graph::Graph, hash::ObjectHash, hash_raw_bytes, key::PublicKey, load_as_msgpack, open_file, remove_path, resolve_wildcard_path, save_as_msgpack, set, snapshot::Snapshot, stash::Stash, sync::remote::Remote, trash::{Entry, Trash, TrashStatus}, unwrap, user::{User, Users}};

use chrono::Utc;
use expand_tilde::ExpandTilde;
use eyre::{bail, eyre, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use relative_path::{PathExt, RelativePathBuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NamedItems<T: Clone> {
    inner: HashMap<String, T>
}

impl<T: Clone> NamedItems<T> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new()
        }
    }

    pub fn create(&mut self, name: String, value: T) -> Option<T> {
        self.inner.insert(name, value)
    }

    pub fn get(&self, name: &str) -> Option<&T> {
        self.inner.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }

    pub fn rename(&mut self, old: &str, new: String) -> bool {
        let Some(hash) = self.inner.remove(old) else {
            return false;
        };
        
        self.inner.insert(new, hash);

        true
    }
    
    pub fn remove(&mut self, name: &str) -> Option<T> {
        self.inner.remove(name)
    }

    // TODO: maybe add &T in case this isn't just used for HashMap<String, ObjectHash>
    pub fn get_name_for(&self, item: T) -> Option<&str> where T: PartialEq<T> {
        self.inner
            .iter()
            .find(|(_, value)| **value == item)
            .map(|(name, _)| name.as_str())
    }

    pub fn get_names_for(&self, item: T) -> Vec<&str> where T: PartialEq<T> {
        self.inner
            .iter()
            .filter(|(_, value)| **value == item)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &T)> {
        self.inner.iter()
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "Easier to make a method"
    )]
    pub fn into_iter(self) -> impl Iterator<Item = (String, T)> {
        self.inner.into_iter()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.inner.values()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

pub struct Repository {
    pub project_name: String,
    pub project_code: ObjectHash,
    pub root_dir: PathBuf,
    pub history: Graph,
    pub action_history: ActionHistory,
    pub branches: NamedItems<ObjectHash>,
    pub current_hash: ObjectHash,
    pub staged_files: Vec<RelativePathBuf>,
    pub ignore_matcher: Gitignore,
    pub stash: Stash,
    pub trash: Trash,
    pub tags: NamedItems<ObjectHash>,
    pub users: Users,
    pub remotes: NamedItems<Remote>,

    pub(crate) current_user: Arc<RwLock<Option<PublicKey>>>
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
        let lock = self.current_user.read().unwrap();

        let key = (*lock)?;

        let user = self.users.get_user(&key)?;

        if user.closed || user.private_key.is_none() {
            let mut lock = self.current_user.write().unwrap();

            *lock = None;

            return None;
        }

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
        let lock = self.current_user.read().unwrap();

        let key = (*lock)?;

        let user = self.users.get_user_mut(&key)?;

        if user.closed || user.private_key.is_none() {
            let mut lock = self.current_user.write().unwrap();

            *lock = None;
            
            return None;
        }

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

        let mut lock = self.current_user.write().unwrap();
        
        *lock = Some(user.public_key);

        Ok(())
    }

    /// Get the branch the repository is currently on.
    /// 
    /// This is only found if the `current_hash` points to a branch tip.
    /// Any other snapshot will result in `None`.
    pub fn current_branch(&self) -> Option<&str> {
        self.branches.get_name_for(self.current_hash)
    }

    /// Find if the `current_hash` doesn't point to the tip of any branch.
    pub fn is_head_detached(&self) -> bool {
        self.current_branch().is_none()
    }

    fn append_snapshot_internal(&mut self, snapshot: Snapshot, branch_name: Option<String>) -> Result<()> {
        let hash = snapshot.hash;

        if let Some(name) = branch_name {
            self.branches.create(name.clone(), hash);

            self.action_history.push(
                Action::CreateBranch {
                    name,
                    hash
                }
            );
        }

        self.save_snapshot(snapshot)?;

        self.action_history.push(
            Action::SwitchVersion {
                before: self.current_hash,
                after: hash
            }
        );

        self.current_hash = hash;
        
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

    /// Convert a smaller hash in string form into its full [`ObjectHash`] version.
    /// 
    /// This works for snapshots and content blobs.
    pub fn normalise_hash(&self, raw_hash: &str) -> Result<ObjectHash> {
        let (dir, file) = raw_hash.split_at(2);

        if dir.is_empty() {
            bail!("expected input to normalise, got an empty string.");
        }

        let mut glob = self.blobs_dir();

        if dir.len() == 2 {
            glob.push(dir);
        }
        else {
            glob.push(format!("{dir}*"));
        }

        if file.len() == 30 {
            glob.push(file);
        }
        else {
            glob.push(format!("{file}*"));
        }

        let results = resolve_wildcard_path(&glob)?;

        if results.is_empty() {
            bail!("found no results when looking for {raw_hash:?} ({glob:?})");
        }

        if results.len() != 1 {
            bail!("found multiple results when looking for {raw_hash:?} ({glob:?}) - {results:?}");
        }

        let path = results[0].relative_to(self.blobs_dir())?;

        let second = path
            .file_stem()
            .unwrap();

        let first = path
            .parent()
            .and_then(|p| p.file_stem())
            .unwrap();

        let raw_hash = format!("{first}{second}");

        let hash = ObjectHash::from_str(&raw_hash)?;

        Ok(hash)
    }

    /// Convert a version in string form into its full [`ObjectHash`] version
    /// by trying to interpret it as a branch name, then trying to interpret
    /// it as the hash of a snapshot.
    pub fn normalise_version(&self, raw_version: &str) -> Result<ObjectHash> {
        if let Some(&corresponding_hash) = self.branches.get(raw_version) {
            Ok(corresponding_hash)
        }
        else if let Some((prefix, name)) = raw_version.split_once("tag:")
            && prefix.split_whitespace().next().is_none()
        {
            self.tags.get(name)
                .cloned()
                .ok_or(eyre!("no tag called {name:?}"))
        }
        else {
            self.normalise_hash(raw_version)
        }
    }

    fn apply_action(&mut self, action: Action) -> Result<()> {
        use Action::*;

        match action {
            CreateBranch { name, .. } => {
                self.branches.remove(&name);
            }

            DeleteBranch { name, hash } => {
                self.branches.create(name, hash);
            }

            MoveBranch { name, new, .. } => {
                self.branches.create(name, new);
            },

            RenameBranch { old, new, .. } => {
                self.branches.rename(&old, new);
            }

            SwitchVersion { after, .. } => {
                self.current_hash = after;
            },

            CreateTag { name, hash } => {
                self.tags.create(name, hash);
            },

            MoveTag { name, new, .. } => {
                self.tags.create(name, new);
            },

            RemoveTag { name, .. } => {
                self.tags.remove(&name);
            },

            RenameTag { old, new, .. } => {
                self.tags.rename(&old, new);
            },

            OpenAccount { id, .. } => {
                let user = unwrap!(
                    self.users.get_user_mut(&id),
                    "no user account with public key {id}"
                );

                user.closed = false;
            },

            CloseAccount { id, .. } => {
                let user = unwrap!(
                    self.users.get_user_mut(&id),
                    "no user account with public key {id}"
                );

                user.closed = true;
            },

            RenameAccount { new, id, .. } => {
                let user = unwrap!(
                    self.users.get_user_mut(&id),
                    "no user account with public key {id}"
                );

                user.name = new;
            },

            TrashAdd { hash } => {
                self.trash.add(hash);
            },

            TrashRecover { hash } => {
                self.trash.remove(hash);
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
            CreateBranch { name, hash } => DeleteBranch { name, hash },
            DeleteBranch { name, hash } => CreateBranch { name, hash },
            MoveBranch { name, old, new } => MoveBranch { name, old: new, new: old },
            RenameBranch { hash, old, new } => RenameBranch { hash, old: new, new: old },

            SwitchVersion { before, after } => SwitchVersion { before: after, after: before },

            CreateTag { name, hash } => RemoveTag { name, hash },
            RemoveTag { name, hash } => CreateTag { name, hash },
            MoveTag { name, old, new } => MoveTag { name, old: new, new: old },
            RenameTag { old, new, hash } => RenameTag { old: new, new: old, hash },

            OpenAccount { id, name } => CloseAccount { id, name },
            CloseAccount { id, name } => OpenAccount { id, name },
            RenameAccount { old, new, id } => RenameAccount { old: new, new: old, id },

            TrashAdd { hash } => TrashRecover { hash },
            TrashRecover { hash } => TrashAdd { hash },
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
    pub current_user: Option<PublicKey>,
    pub branches: NamedItems<ObjectHash>,
    pub current_hash: ObjectHash,
    pub stash: Stash,
    pub remotes: NamedItems<Remote>
}

impl ProjectInfo {
    pub fn from_file(path: impl AsRef<Path>) -> Result<ProjectInfo> {
        let fp = open_file(path)?;

        let info = rmp_serde::from_read(fp)?;

        Ok(info)
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

        let now = Utc::now().timestamp();

        let project_code = hash_raw_bytes(now.to_le_bytes());
        
        let mut users = Users::new();

        let first_user = {
            let user = users.create_user(author.clone())?;

            user.private_key.clone().unwrap()
        };

        let current_user = Arc::new(RwLock::new(Some(first_user.public_key())));

        let mut history = Graph::new();

        let root_snapshot = Snapshot::new(
            first_user,
            "initial snapshot".to_string(),
            Utc::now(),
            BTreeMap::new(),
            set![]
        );

        history.insert_orphan(root_snapshot.hash);

        let mut branches = NamedItems::<ObjectHash>::new();

        branches.create("main".to_string(), root_snapshot.hash);

        let mut repo = Repository {
            project_name,
            project_code,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history: ActionHistory::new(),
            history,
            branches,
            current_hash: root_snapshot.hash,
            current_user,
            staged_files: vec![],
            stash: Stash::new(),
            trash: Trash::new(),
            tags: NamedItems::new(),
            users,
            remotes: NamedItems::new()
        };

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

        let info: ProjectInfo = load_as_msgpack(content_dir.join("info"))?;

        let history = load_as_msgpack(content_dir.join("tree"))?;

        let staged_files = load_as_msgpack(content_dir.join("index"))?;

        let action_history = load_as_msgpack(content_dir.join("history"))?;

        let trash = load_as_msgpack(content_dir.join("trash"))?;

        let tags = load_as_msgpack(content_dir.join("tags"))?;

        let users = load_as_msgpack(content_dir.join("users"))?;

        let repo = Repository {
            project_name: info.project_name,
            project_code: info.project_code,
            ignore_matcher: get_ignore_matcher(&root_dir)?,
            root_dir,
            action_history,
            history,
            branches: info.branches,
            current_hash: info.current_hash,
            current_user: Arc::new(RwLock::new(info.current_user)),
            staged_files,
            stash: info.stash,
            trash,
            tags,
            users,
            remotes: info.remotes
        };

        Ok(repo)
    }

    /// Save the current state of the repository to disk.
    pub fn save(&mut self) -> Result<()> {
        self.validate_state()?;
        
        let current_user = *self.current_user.read().unwrap();

        let content_dir = self.main_dir();

        let info = ProjectInfo {
            project_name: self.project_name.clone(),
            project_code: self.project_code,
            current_user,
            branches: self.branches.clone(),
            current_hash: self.current_hash,
            stash: self.stash.clone(),
            remotes: self.remotes.clone()
        };

        save_as_msgpack(&info, content_dir.join("info"))?;

        save_as_msgpack(&self.history, content_dir.join("tree"))?;

        let set: BTreeSet<RelativePathBuf> = self.staged_files
            .iter()
            .map(|path| path.normalize())
            .collect();

        self.staged_files = set.into_iter().collect();

        save_as_msgpack(&self.staged_files, content_dir.join("index"))?;

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

    /// Fetch a [`Content`] object from the repository, addressed by its hash.
    pub fn fetch_content_object(&self, content_hash: ObjectHash) -> Result<Content> {
        let path = self.hash_to_path(content_hash);

        let raw = unwrap!(
            fs::read(&path),
            "failed to read bytes from: {}", path.display()
        );

        let content: Content = rmp_serde::from_slice(&raw)?;

        Ok(content)
    }
    
    /// Fetch a `String` from the repository, addressed by its hash.
    pub fn fetch_string_content(&self, content_hash: ObjectHash) -> Result<String> {
        let content = self.fetch_content_object(content_hash)?;

        content.resolve(self)
    }

    /// Fetch a [`Snapshot`] from the repository, addressed by its hash.
    pub fn fetch_snapshot(&self, snapshot_hash: ObjectHash) -> Result<Snapshot> {
        let path = self.hash_to_path(snapshot_hash);
        
        let fp = open_file(path)?;

        let snapshot: Snapshot = rmp_serde::from_read(fp)?;

        snapshot.verify()?;

        Ok(snapshot)
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
        let original = self.fetch_string_content(basis)?;

        let hash = hash_raw_bytes(content);

        let Some(delta) = Delta::new(
            &original,
            content,
            MIN_DELTA_SIMILARITY
        ) else {
            return Ok(None);
        };

        self.save_content_object(Content::Delta(delta), hash)?;

        Ok(Some(hash))
    }

    /// Save a string as a delta of some other string on disk, regardless of the similarity
    /// of the two strings.
    /// 
    /// For a method that considers similarity, use the safer [`Repository::save_content_delta`],
    /// or the higher-level [`Repository::save_content`].
    pub fn save_content_delta_unchecked(&self, content: &str, basis: ObjectHash) -> Result<ObjectHash> {
        let original = self.fetch_string_content(basis)?;

        let hash = hash_raw_bytes(content);

        let delta = Content::Delta(Delta::new_unchecked(&original, content));

        self.save_content_object(delta, hash)?;

        Ok(hash)
    }

    /// Save a [`Content`] object, most likely obtained from network transfer.
    pub fn save_content_object(&self, object: Content, hash: ObjectHash) -> Result<()> {
        let path = self.hash_to_path(hash);

        if path.exists() {
            return Ok(());
        }
        
        save_as_msgpack(&object, path)
    }

    /// Save a snapshot as a compressed blob to disk.
    pub fn save_snapshot(&mut self, mut snapshot: Snapshot) -> Result<()> {
        snapshot.rehash();

        self.history.insert_orphan(snapshot.hash);

        for &parent in &snapshot.parents {
            self.history.insert(snapshot.hash, parent);
        }

        if self.users.get_user(&snapshot.signature.key()).is_none()  {
            bail!("snapshot is authored by an unknown user.");
        }

        snapshot.verify()?;

        let path = self.hash_to_path(snapshot.hash);

        save_as_msgpack(&snapshot, path)
    }

    /// Assemble a [`Snapshot`] from the repository's tracked files.
    /// 
    /// This saves the tracked files' contents to disk, as well as the [`Snapshot`].
    pub fn commit_current_state(&self, message: String) -> Result<Snapshot> {
        let user = unwrap!(
            self.current_user(),
            "cannot commit state: no valid user.",
        );
        
        let key = user.private_key.clone().unwrap();

        let base_files = self.fetch_current_snapshot()?.files;

        let mut files = BTreeMap::new();
        
        for path in &self.staged_files {
            let full_path = path.to_logical_path(&self.root_dir);

            let content = unwrap!(
                fs::read_to_string(full_path),
                "could not read from path: {path}"
            );

            let hash = self.save_content(&content, base_files.get(path).cloned())?;

            files.insert(path.clone(), hash);
        }

        let snapshot = Snapshot::new(
            key,
            message,
            Utc::now(),
            files,
            set![self.current_hash]
        );

        Ok(snapshot)
    }
}

impl Repository {
    fn cwd_differs_from_snapshot(&self, files: &BTreeMap<RelativePathBuf, ObjectHash>) -> Result<bool> {
        let mut paths_remaining: HashSet<_> = files.keys().collect();

        for path in &self.staged_files {
            paths_remaining.remove(path);

            let full_path = path.to_logical_path(&self.root_dir);

            if !full_path.exists() {
                return Ok(true);
            }

            let current_content = unwrap!(
                fs::read_to_string(full_path),
                "failed to read path: {path}"
            );

            let current_content_hash = hash_raw_bytes(&current_content);

            let Some(&previous_content_hash) = files.get(path) else {
                return Ok(true)
            };

            if previous_content_hash != current_content_hash {
                return Ok(true);
            }
        }

        if !paths_remaining.is_empty() {
            return Ok(true);
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
        if !self.cwd_differs_from_snapshot(&current.files)? {
            return Ok(false);
        }

        // If the CWD matches a snapshot in the stash,
        // no changes are made, and content is safe.
        for entry in self.stash.iter_entries() {
            if !self.cwd_differs_from_snapshot(&entry.state.files)? {
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

        self.replace_cwd_with_files(&snapshot.files)
    }

    /// Replace the state of the current working directory with that
    /// from a set of files, but **DO NOT** check if there are
    /// unsaved changes.
    /// 
    /// For a safer alternative, use [`Repository::replace_cwd_with_snapshot`].
    pub fn replace_cwd_with_files(&mut self, files: &BTreeMap<RelativePathBuf, ObjectHash>) -> Result<()> {
        let current = self.fetch_current_snapshot()?;

        // Delete paths that are in this snapshot but not the destination snapshot.
        for path in current.files.keys() {
            let full_path = path.to_logical_path(&self.root_dir);

            if !files.contains_key(path) {
                remove_path(full_path, &self.root_dir)?;
            }
        }

        for (path, &new) in files {
            let content = self.fetch_string_content(new)?;

            let full_path = path.to_logical_path(&self.root_dir);

            unwrap!(
                fs::create_dir_all(full_path.parent().unwrap()),
                "failed to create directory for: {path}"
            );

            unwrap!(
                fs::write(&full_path, content),
                "failed to write to path: {path}"
            );
        }

        self.staged_files = files
            .keys()
            .cloned()
            .collect();

        Ok(())
    }

    /// List all the changes as [`FileChange`] objects between
    /// the current snapshot and the current working directory.
    pub fn list_changes(&self) -> Result<Vec<FileChange>> {
        let old_files = self.fetch_current_snapshot()?.files;

        let old_paths: HashSet<&RelativePathBuf> = old_files
            .keys()
            .collect();

        let new_paths: HashSet<&RelativePathBuf> = self.staged_files
            .iter()
            .collect();

        let mut file_changes: Vec<FileChange> = vec![];

        file_changes.extend(
            new_paths
                .difference(&old_paths)
                .map(|&p| FileChange::Added(p.clone()))
        );

        file_changes.extend(
            old_paths
                .difference(&new_paths)
                .map(|&p| FileChange::Removed(p.clone()))
        );

        file_changes.extend(
            new_paths
                .iter()
                .filter_map(|&p| {
                    let full_path = p.to_logical_path(&self.root_dir);

                    (!full_path.exists()).then_some(FileChange::Missing(p.clone()))
                })
        );

        for (path, hash) in old_files {
            let full_path = path.to_logical_path(&self.root_dir);

            if !full_path.exists() {
                continue;
            }

            let content = fs::read_to_string(&full_path)?;

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

    /// Performs a check across the entire repository to see if:
    /// 
    /// * the commit history is intact
    /// * all commit signatures are valid
    /// * all commit authors are valid users
    /// * all commit parents are correct
    /// * all content is present
    /// * all branch pointers and tags are valid
    /// 
    /// This only considers reachable commits.
    pub fn validate_state(&self) -> Result<()> {
        let mut queue = VecDeque::new();

        queue.extend(self.branches.values().cloned());

        while let Some(current) = queue.pop_back() {
            let snapshot = self.fetch_snapshot(current)?;

            let parents = unwrap!(
                self.history.get_parents(current),
                "cannot get parents for hash {current:?}"
            );

            if parents != &snapshot.parents {
                bail!("snapshot {current} has invalid parents (parents in graph differ from parents in signature)");
            }

            let author = snapshot.signature.key();

            if self.users.get_user(&author).is_none() {
                bail!("snapshot {current} was created by an unknown user (key {author} matches no user)");
            }

            snapshot.verify()?;

            for hash in snapshot.files.into_values() {
                self.fetch_content_object(hash)?;
            }

            queue.extend(parents);
        }

        for (name, &hash) in self.tags.iter() {
            if !self.history.contains(hash) {
                bail!("tag {name:?} points to an unrecorded ({hash:?})");
            }
        }

        Ok(())
    }
}
