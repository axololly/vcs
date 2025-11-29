use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::backend::{action::{Action, ActionHistory}, graph::Graph, hash::ObjectHash, snapshot::Snapshot, stash::Stash, trash::Trash};

use eyre::{Result, bail, eyre};
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
    pub trash: Trash
}

impl Repository {
    pub fn current_branch(&self) -> Option<&str> {
        self.branch_from_hash(self.current_hash)
    }

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

    /// The head is detached when it doesn't point to the tip of any branch.
    pub fn is_head_detached(&self) -> bool {
        self.current_branch().is_none()
    }

    fn append_snapshot_internal(&mut self, snapshot: Snapshot, branch_name: Option<String>) -> Result<()> {
        if !self.cwd_differs_from_current()? {
            bail!("No changes have been made in the current working directory.");
        }

        self.history.insert(snapshot.hash, self.current_hash);

        self.action_history.push(
            Action::CreateSnapshot {
                hash: snapshot.hash,
                parents: vec![self.current_hash],
            }
        );

        if let Some(name) = branch_name {
            self.branches.insert(name, snapshot.hash);
        }

        self.current_hash = snapshot.hash;
        
        self.save_snapshot(&snapshot)?;
        
        Ok(())
    }

    pub fn append_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        self.append_snapshot_internal(snapshot, self.current_branch().map(String::from))
    }

    pub fn append_snapshot_to_branch(&mut self, snapshot: Snapshot, branch_name: String) -> Result<()> {
        self.append_snapshot_internal(snapshot, Some(branch_name))
    }

    pub fn is_ignored_path(&self, path: &Path) -> bool {
        self.ignore_matcher.matched(path, path.is_dir()).is_ignore()
    }

    fn normalise_hash_internal(&self, iter: impl Iterator<Item = ObjectHash>, needle: &[u8]) -> Option<ObjectHash> {
        for item in iter {
            let bytes: &[u8] = item.as_ref();

            if bytes.starts_with(needle) {
                return Some(item);
            }
        }

        None
    }

    pub fn normalise_hash(&self, raw_hash: &str) -> Result<ObjectHash> {
        let as_hex = hex::decode(raw_hash)?;

        if as_hex.len() <= 2 {
            bail!("hash {raw_hash:?} is too short to normalise");
        }

        let commit_hashes = self
            .history
            .links
            .keys()
            .cloned();

        if let Some(normalised) = self.normalise_hash_internal(commit_hashes, &as_hex) {
            Ok(normalised)
        }
        else {
            bail!("could not resolve hash: {raw_hash:?}");
        }
    }

    pub fn normalise_stash_hash(&self, raw_hash: &str) -> Result<ObjectHash> {
        let as_hex = hex::decode(raw_hash)?;

        if as_hex.len() <= 2 {
            bail!("hash {raw_hash:?} is too short to normalise");
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
            CreateSnapshot { hash, parents } => {
                for parent in parents {
                    self.history.insert(hash, parent);
                }
            }

            DeleteSnapshot { hash, .. } => {
                self.history.remove(hash);
            }

            RebaseSnapshot { hash, to, .. } => {
                let parents = self.history.links.get_mut(&hash)
                    .ok_or(eyre!("{hash} does not exist in the repository."))?;

                parents.clear();

                parents.extend(to.iter());
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

    pub fn redo_action(&mut self) -> Result<Option<Action>> {
        let Some(action) = self.action_history.undo().cloned() else {
            return Ok(None)
        };

        self.apply_action(action.clone())?;

        Ok(Some(action))
    }

    pub fn undo_action(&mut self) -> Result<Option<Action>> {
        let Some(action) = self.action_history.undo().cloned() else {
            return Ok(None)
        };

        use Action::*;

        let inverse = match action {
            CreateSnapshot { hash, parents } => DeleteSnapshot { hash, parents },
            DeleteSnapshot { hash, parents } => CreateSnapshot { hash, parents },
            RebaseSnapshot { hash, from, to } => RebaseSnapshot { hash, from: to, to: from },
            ModifySnapshot { hash, before, after } => ModifySnapshot { hash, before: after, after: before },

            CreateBranch { name, hash } => DeleteBranch { name, hash },
            DeleteBranch { name, hash } => CreateBranch { name, hash },
            RenameBranch { hash, old, new } => RenameBranch { hash, old: new, new: old },

            SwitchVersion { before, after } => SwitchVersion { before: after, after: before },
        };

        self.apply_action(inverse.clone())?;

        Ok(Some(inverse))
    }
}