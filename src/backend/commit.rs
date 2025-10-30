use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Local};

use super::hash::CommitHash;

/// Represents the header of a commit.
/// 
/// This doesn't contain any information about the files in the commit,
/// making it fairly lightweight to interpret.
pub struct CommitHeader {
    pub hash: CommitHash,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Local>
}

pub struct Commit {
    pub header: CommitHeader,
    pub files: BTreeMap<PathBuf, Vec<u8>>
}

impl Commit {
    pub fn hash(&self) -> CommitHash {
        self.header.hash
    }
}