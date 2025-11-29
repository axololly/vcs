use derive_more::Display;

use serde::{Deserialize, Serialize};

use crate::{backend::{hash::ObjectHash, snapshot::Snapshot}, utils::DisplaySeq};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Change<T> {
    before: T,
    after: T
}

#[derive(Clone, Debug, Display, Deserialize, Serialize, PartialEq)]
pub enum Action {
    // Snapshots
    #[display("created snapshot {hash} with parents {:?}", DisplaySeq(parents))]
    CreateSnapshot {
        hash: ObjectHash,
        parents: Vec<ObjectHash>
    },
    #[display("deleted snapshot {hash} with parents {:?}", DisplaySeq(parents))]
    DeleteSnapshot {
        hash: ObjectHash,
        parents: Vec<ObjectHash>
    },
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


#[derive(Default, Deserialize, Serialize)]
pub struct ActionHistory {
    inner: Vec<Action>,
    index: usize
}

impl ActionHistory {
    pub fn new() -> ActionHistory {
        ActionHistory {
            inner: vec![],
            index: 0
        }
    }

    pub fn push(&mut self, action: Action) {
        self.inner.truncate(self.index);
        
        self.inner.push(action);
        
        self.index += 1;
    }

    pub fn current(&self) -> Option<&Action> {
        (self.index > 0).then(|| &self.inner[self.index - 1])
    }

    pub fn undo(&mut self) -> Option<&Action> {
        if self.index == 0 {
            return None;
        }

        self.index -= 1;

        Some(&self.inner[self.index])
    }

    pub fn redo(&mut self) -> Option<&Action> {
        if self.index + 1 > self.inner.len() {
            return None;
        }

        self.index += 1;

        self.current()
    }

    pub fn clear(&mut self) {
        self.inner.clear();

        self.index = 0;
    }

    pub fn as_vec(&self) -> &Vec<Action> {
        &self.inner
    }
}