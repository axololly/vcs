use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::hash::ObjectHash;

#[allow(unused_imports, reason = "")]
use super::repository::Repository;

/// Represents the state of the project within the repository.
/// 
/// Each snapshot can be identified by its [`ObjectHash`] which is
/// assigned at creation. While its metadata like `author` and `message`
/// can be altered, its hash remains immutable.
/// 
/// ### About `files`
/// 
/// The `files` attribute does not include each file's content directly
/// in order to reduce memory overhead in the case of large file. Instead,
/// a hash is kept, and is resolved with [`Repository::fetch_string_content`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Snapshot {
    pub hash: ObjectHash,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    
    // A BTreeMap is used to preserve order, so that
    // reconstructing and validating the hash is easier.
    pub files: BTreeMap<PathBuf, ObjectHash>
}

impl Snapshot {
    pub fn from_parts(
        author: String,
        message: String,
        timestamp: DateTime<Local>,
        files: BTreeMap<PathBuf, ObjectHash>
    ) -> Snapshot
    {
        let mut snapshot_hasher = Sha1::new();

        snapshot_hasher.update(author.as_bytes());
        
        snapshot_hasher.update(message.as_bytes());

        snapshot_hasher.update(timestamp.timestamp().to_le_bytes());

        for (_, hash) in &files {
            snapshot_hasher.update(hash.as_bytes());
        }

        let raw_snapshot_hash: [u8; 20] = snapshot_hasher.finalize().into();

        Snapshot {
            hash: raw_snapshot_hash.into(),
            author,
            message,
            timestamp,
            files
        }
    }
}