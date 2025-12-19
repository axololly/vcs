use derive_more::Display;

use serde::{Deserialize, Serialize};

use crate::{core::{hash::ObjectHash, snapshot::Snapshot}, utils::DisplaySeq};

/// Represents an action made on the repository.
/// 
/// These are (currently) all reversible.
#[derive(Clone, Debug, Display, Deserialize, Serialize, PartialEq)]
pub enum Action {
    // Snapshots
    #[display("modified snapshot {hash}")]
    ModifySnapshot {
        hash: ObjectHash,
        before: Snapshot,
        after: Snapshot
    },
    #[display("rebased snapshot {hash} from {:?} to {:?}", DisplaySeq(from), DisplaySeq(to))]
    RebaseSnapshot {
        hash: ObjectHash,
        from: Vec<ObjectHash>,
        to: Vec<ObjectHash>
    },

    // Branches
    #[display("created branch {name:?} pointing to {hash}")]
    CreateBranch {
        name: String,
        hash: ObjectHash
    },
    #[display("deleted branch {name:?} that was pointing to {hash}")]
    DeleteBranch {
        name: String,
        hash: ObjectHash
    },
    #[display("renamed branch {old} to {new} ({hash})")]
    RenameBranch {
        hash: ObjectHash,
        old: String,
        new: String
    },

    // Checkouts
    #[display("switched versions from {before} to {after}")]
    SwitchVersion {
        before: ObjectHash,
        after: ObjectHash
    }
}

/// A stack of [`Action`] enum members with undo and redo capabilities.
#[derive(Default, Deserialize, Serialize)]
pub struct ActionHistory {
    inner: Vec<Action>,
    index: usize
}

impl ActionHistory {
    /// Create a new [`ActionHistory`].
    pub fn new() -> ActionHistory {
        ActionHistory {
            inner: vec![],
            index: 0
        }
    }

    /// Add a new [`Action`] to the history.
    /// 
    /// ### Warning
    /// This truncates the internal [`Action`] stack,
    /// removing the ability to redo any undone actions.
    pub fn push(&mut self, action: Action) {
        self.inner.truncate(self.index);
        
        self.inner.push(action);
        
        self.index += 1;
    }

    /// Get the topmost [`Action`] on the stack.
    pub fn current(&self) -> Option<&Action> {
        if self.index == 0 {
            return None;
        }
        
        Some(&self.inner[self.index - 1])
    }

    /// Undo an [`Action`] in the history, returning the undone action.
    /// 
    /// This is only permanently lost if [`ActionHistory::push`] is called.
    pub fn undo(&mut self) -> Option<&Action> {
        if self.index == 0 {
            return None;
        }

        self.index -= 1;

        Some(&self.inner[self.index])
    }

    /// Redo an [`Action`] in the history, returning the redone action.
    pub fn redo(&mut self) -> Option<&Action> {
        if self.index + 1 > self.inner.len() {
            return None;
        }

        self.index += 1;

        self.current()
    }

    /// Clear the history.
    pub fn clear(&mut self) {
        self.inner.clear();

        self.index = 0;
    }

    /// Get a reference to the internal stack.
    pub fn as_slices(&self) -> (&[Action], &[Action]) {
        self.inner.split_at(self.index)
    }
}