use std::{collections::{BTreeMap, HashSet}, path::PathBuf};

use chrono::{DateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{hash::{ObjectHash, RawObjectHash}, key::{PrivateKey, PublicKey, Signature}, unwrap};

#[allow(unused_imports, reason = "used for documentation.")]
use super::repository::Repository;

/// Represents a collection of files, with some metadata about them.
/// 
/// ### About `files`
/// 
/// The `files` attribute does not include each file's content directly
/// in order to reduce memory overhead in the case of large file. Instead,
/// a hash is kept, and is resolved with [`Repository::fetch_string_content`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Snapshot {
    pub hash: ObjectHash,
    pub author: PublicKey,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    
    // A BTreeMap is used to preserve order, so that
    // reconstructing and validating the hash is easier.
    pub files: BTreeMap<PathBuf, ObjectHash>,

    pub parents: HashSet<ObjectHash>,
    pub signature: Signature
}

fn hash_from_parts(
    author: PublicKey,
    message: &str,
    timestamp: &DateTime<Utc>,
    files: &BTreeMap<PathBuf, ObjectHash>,
    parents: &HashSet<ObjectHash>
) -> ObjectHash
{
    let mut hasher = Sha256::new();

    hasher.update(author.to_bytes());

    hasher.update(message.as_bytes());

    hasher.update(timestamp.timestamp().to_be_bytes());

    for (path, hash) in files {
        hasher.update(path.as_os_str().as_encoded_bytes());

        hasher.update(hash.as_bytes());
    }

    for parent in parents {
        hasher.update(parent.as_bytes());
    }

    let raw_hash: RawObjectHash = hasher.finalize().into();

    raw_hash.into()
}

impl Snapshot {
    /// Create a new [`SignedSnapshot`].
    /// 
    /// Ensure that the [`PrivateKey`] here belongs to the snapshot's author.
    pub fn new(
        mut creator: PrivateKey,
        message: String,
        timestamp: DateTime<Utc>,
        files: BTreeMap<PathBuf, ObjectHash>,
        parents: HashSet<ObjectHash>
    ) -> Snapshot
    {
        let author = creator.public_key();
        
        let hash = hash_from_parts(
            author,
            &message,
            &timestamp,
            &files,
            &parents
        );

        let signature = creator.sign(hash.as_bytes());

        Snapshot {
            hash,
            author,
            message,
            timestamp,
            files,
            parents,
            signature
        }
    }

    /// Rehash the [`Snapshot`] in case anything has changed.
    pub fn rehash(&mut self) {
        self.hash = hash_from_parts(
            self.author,
            &self.message,
            &self.timestamp,
            &self.files,
            &self.parents
        );
    }

    /// Check if the snapshot is authentic.
    /// 
    /// This will return `false` if an error unrelated to verifying the signature arises.
    pub fn is_valid(&self) -> bool {
        let hash = hash_from_parts(
            self.author,
            &self.message,
            &self.timestamp,
            &self.files,
            &self.parents
        );

        if self.hash != hash {
            return false;
        }
        
        self.signature.verify(hash.as_bytes())
    }

    /// Verify the [`Signature`] on the [`Snapshot`], returning any
    /// errors from the signature verification process.
    pub fn verify(&self) -> Result<()> {
        unwrap!(
            self.signature.check(self.hash.as_bytes()),
            "failed to verify signature of snapshot {:?} using key {:?}",
            self.hash, self.signature.key()
        );

        Ok(())
    }
}
