use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::hash::ObjectHash;

#[allow(unused_imports, reason = "used for documentation.")]
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

fn hash_from_parts(
    author: &str,
    message: &str,
    timestamp: &DateTime<Local>,
    files: &BTreeMap<PathBuf, ObjectHash>
) -> ObjectHash
{
    let mut hasher = Sha1::new();

    hasher.update(author.as_bytes());
    
    hasher.update(message.as_bytes());

    hasher.update(timestamp.timestamp().to_le_bytes());

    for (_, hash) in files {
        hasher.update(hash.as_bytes());
    }

    let raw_hash: [u8; 20] = hasher.finalize().into();

    raw_hash.into()
}

impl Snapshot {
    pub fn validate_hash(&self) -> bool {
        self.hash == hash_from_parts(
            &self.author,
            &self.message,
            &self.timestamp,
            &self.files
        )
    }

    pub fn from_parts(
        author: String,
        message: String,
        timestamp: DateTime<Local>,
        files: BTreeMap<PathBuf, ObjectHash>
    ) -> Snapshot
    {
        let hash = hash_from_parts(
            &author,
            &message,
            &timestamp,
            &files
        );
        
        Snapshot {
            hash,
            author,
            message,
            timestamp,
            files
        }
    }
}