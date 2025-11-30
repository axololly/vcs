use serde::{Deserialize, Serialize};

use crate::backend::hash::ObjectHash;

/// Represents a snapshot independent of the history.
/// 
/// This also stores the snapshot from which the stash was made from.
#[derive(Clone, Deserialize, Serialize)]
pub struct Stash {
    pub snapshot: ObjectHash,
    pub basis: ObjectHash
}

impl PartialEq for Stash {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot.eq(&other.snapshot)
    }
}