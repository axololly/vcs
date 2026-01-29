use derive_more::Display;

use serde::{Deserialize, Serialize};

use crate::{hash::ObjectHash, key::PublicKey};

/// Represents an action made on the repository.
/// 
/// These are (currently) all reversible.
#[derive(Clone, Debug, Display, Deserialize, Serialize, PartialEq)]
pub enum Action {
    // Branches
    #[display("Created branch {name:?} pointing to {hash}")]
    CreateBranch {
        name: String,
        hash: ObjectHash
    },
    #[display("Deleted branch {name:?} that was pointing to {hash}")]
    DeleteBranch {
        name: String,
        hash: ObjectHash
    },
    #[display("Moved branch {name:?} from {old} to {new}")]
    MoveBranch {
        name: String,
        old: ObjectHash,
        new: ObjectHash
    },
    #[display("Renamed branch {old} to {new} ({hash})")]
    RenameBranch {
        hash: ObjectHash,
        old: String,
        new: String
    },

    // Checkouts
    #[display("Switched versions from {before} to {after}")]
    SwitchVersion {
        before: ObjectHash,
        after: ObjectHash
    },

    // Tags
    #[display("Added tag {name:?} at {hash}")]
    CreateTag {
        name: String,
        hash: ObjectHash
    },
    #[display("Removed tag {name:?} (previously was at {hash})")]
    RemoveTag {
        name: String,
        hash: ObjectHash
    },
    #[display("Renamed tag {old:?} to {new:?} ({hash})")]
    RenameTag {
        old: String,
        new: String,
        hash: ObjectHash
    },

    // Trash
    #[display("Added {hash} to trash")]
    TrashAdd {
        hash: ObjectHash
    },
    #[display("Recovered {hash} from trash")]
    TrashRecover {
        hash: ObjectHash
    },

    // Users
    #[display("Opened account {name:?} (key: {})", &id.to_string()[..8])]
    OpenAccount {
        name: String,
        id: PublicKey
    },
    #[display("Closed account {name:?} (key: {})", &id.to_string()[..8])]
    CloseAccount {
        name: String,
        id: PublicKey
    },
    #[display("Renamed account {old:?} to {new:?} (key: {})", &id.to_string()[..8])]
    RenameAccount {
        old: String,
        new: String,
        id: PublicKey
    }
}

/// A stack of [`Action`] enum members with undo and redo capabilities.
#[derive(Clone, Default, Deserialize, Serialize)]
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
