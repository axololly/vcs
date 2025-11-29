use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::backend::hash::ObjectHash;

#[derive(Clone, Deserialize, Serialize)]
pub struct Stash {
    pub snapshot: ObjectHash,
    pub basis: ObjectHash,
    pub staged_files: Vec<PathBuf>
}

impl PartialEq for Stash {
    fn eq(&self, other: &Self) -> bool {
        self.snapshot.eq(&other.snapshot)
    }
}