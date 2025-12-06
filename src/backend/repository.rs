use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::backend::{action::{Action, ActionHistory}, graph::Graph, hash::ObjectHash, snapshot::Snapshot, stash::Stash, trash::{Entry, Trash, TrashStatus}};

use eyre::{Result, bail};
use ignore::gitignore::Gitignore;

pub struct Repository {
    pub project_name: String,
    pub root_dir: PathBuf,
    pub history: Graph,
    pub action_history: ActionHistory,
    pub branches: HashMap<String, ObjectHash>,
    pub current_hash: ObjectHash,
    pub current_user: String,
    pub staged_files: Vec<PathBuf>,
    pub ignore_matcher: Gitignore,
    pub stashes: Vec<Stash>,
    pub trash: Trash,
    pub tags: HashMap<String, ObjectHash>
}

impl Repository {
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

        self.history.insert(snapshot.hash, self.current_hash);

        if let Some(name) = branch_name {
            self.branches.insert(name, snapshot.hash);
        }

        self.current_hash = snapshot.hash;
        
        self.save_snapshot(&snapshot)?;
        
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
            bail!("attempted to normalise stash hash.");
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

            ModifySnapshot { after, .. } => {
                self.save_snapshot(&after)?;
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
            ModifySnapshot { before, after, .. } => ModifySnapshot { hash: after.hash, before: after, after: before },

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
            if self.history.is_descendant(hash, *trash_hash) {
                return Some(TrashStatus::Indirect(*trash_hash));
            }
        }

        None
    }
}