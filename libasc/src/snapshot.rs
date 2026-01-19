use std::{collections::BTreeMap, path::PathBuf};

use chrono::{DateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{hash::{ObjectHash, RawObjectHash}, key::{PrivateKey, Signature}, unwrap};

#[allow(unused_imports, reason = "used for documentation.")]
use super::repository::Repository;

// TODO: could include something like AddFile(PathBuf, ObjectHash)
// and RemoveFile(...) for modifying the content of a snapshot 

/// Represents a metadata edit on a commit.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SnapshotEdit {
    ReplaceAuthor(String, Signature),
    ReplaceMessage(String),
    ReplaceTimestamp(DateTime<Utc>)
}

impl SnapshotEdit {
    pub fn hash(&self) -> ObjectHash {
        use SnapshotEdit::*;

        let (header, data): (&[u8], &[u8]) = match self {
            ReplaceAuthor(name, signature) => (name.as_bytes(), &signature.to_bytes()),
            ReplaceMessage(text) => (b"message", text.as_bytes()),
            ReplaceTimestamp(utc) => (b"timestamp", &utc.timestamp().to_be_bytes())
        };

        let mut hasher = Sha256::new();

        hasher.update(header);

        hasher.update(data);

        let raw_hash: RawObjectHash = hasher.finalize().into();

        raw_hash.into()
    }
}

/// Represents an edit that can be traced back to its author.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SignedSnapshotEdit {
    edits: Vec<SnapshotEdit>,
    signature: Signature
}

impl SignedSnapshotEdit {
    pub fn new(edits: Vec<SnapshotEdit>, mut private_key: PrivateKey) -> SignedSnapshotEdit {
        let mut hasher = Sha256::new();

        for edit in &edits {
            hasher.update(edit.hash().as_bytes());
        }

        let raw_hash: RawObjectHash = hasher.finalize().into();

        let signature = private_key.sign(&raw_hash);

        SignedSnapshotEdit {
            edits,
            signature
        }
    }

    pub fn edits(&self) -> &[SnapshotEdit] {
        &self.edits
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

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
    
    author: String,
    message: String,
    timestamp: DateTime<Utc>,
    
    // A BTreeMap is used to preserve order, so that
    // reconstructing and validating the hash is easier.
    pub files: BTreeMap<PathBuf, ObjectHash>,

    pub edits: Vec<SignedSnapshotEdit>
}

fn hash_from_parts(
    author: &str,
    message: &str,
    timestamp: &DateTime<Utc>,
    files: &BTreeMap<PathBuf, ObjectHash>
) -> ObjectHash
{
    let mut hasher = Sha256::new();

    hasher.update(author.as_bytes());
    
    hasher.update(message.as_bytes());

    hasher.update(timestamp.timestamp().to_le_bytes());

    for hash in files.values() {
        hasher.update(hash.as_bytes());
    }

    let raw_hash: RawObjectHash = hasher.finalize().into();

    raw_hash.into()
}

impl Snapshot {
    /// Create a new [`Snapshot`].
    pub fn new(
        author: String,
        message: String,
        timestamp: DateTime<Utc>,
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
            files,
            edits: vec![]
        }
    }

    /// Get the original author of the [`Snapshot`]
    /// before it was edited (if ever).
    pub fn original_author(&self) -> &str {
        &self.author
    }

    /// Get the author set on the [`Snapshot`].
    pub fn author(&self) -> &str {
        self.edits
            .iter()
            .rev()
            .flat_map(|signed_edit| signed_edit.edits.iter().rev())
            .find_map(|edit| {
                if let SnapshotEdit::ReplaceAuthor(new_author, _) = edit {
                    Some(new_author)
                }
                else {
                    None
                }
            })
            .unwrap_or(&self.author)
    }

    /// Get the original message of the [`Snapshot`]
    /// before it was edited (if ever).
    pub fn original_message(&self) -> &str {
        &self.message
    }

    /// Get the message set on the [`Snapshot`].
    pub fn message(&self) -> &str {
        self.edits
            .iter()
            .rev()
            .flat_map(|signed_edit| signed_edit.edits.iter().rev())
            .find_map(|edit| {
                if let SnapshotEdit::ReplaceMessage(new_message) = edit {
                    Some(new_message)
                }
                else {
                    None
                }
            })
            .unwrap_or(&self.message)
    }

    /// Get the original timestamp of the [`Snapshot`]
    /// before it was edited (if ever).
    pub fn original_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Get the timestamp set on the [`Snapshot`].
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.edits
            .iter()
            .rev()
            .flat_map(|signed_edit| signed_edit.edits.iter().rev())
            .find_map(|edit| {
                if let SnapshotEdit::ReplaceTimestamp(new_timestamp) = edit {
                    Some(*new_timestamp)
                }
                else {
                    None
                }
            })
            .unwrap_or(self.timestamp)
    }

    /// Check if the hash on the [`Snapshot`] matches
    /// one constructed from its contents.
    pub fn validate_hash(&self) -> bool {
        self.hash == hash_from_parts(
            &self.author,
            &self.message,
            &self.timestamp,
            &self.files
        )
    }

    /// Sign this [`Snapshot`] with the given [`PrivateKey`],
    /// creating a [`SignedSnapshot`].
    /// 
    /// Ensure that the [`PrivateKey`] here belongs to the snapshot's author.
    pub fn sign(self, private_key: PrivateKey) -> SignedSnapshot {
        SignedSnapshot::new(self, private_key)
    }
}

/// Represents a signed snapshot used in transport for authenticity.
/// 
/// This takes a [`Snapshot`] and includes a [`Signature`] created from the given [`PrivateKey`].
/// 
/// This struct operates under the assumption that its [`Snapshot`] is signed by its author.
#[derive(Debug, Deserialize, Serialize)]
pub struct SignedSnapshot {
    snapshot: Snapshot,
    signature: Signature
}

impl SignedSnapshot {
    /// Create a new [`SignedSnapshot`].
    /// 
    /// Ensure that the [`PrivateKey`] here belongs to the snapshot's author.
    pub fn new(snapshot: Snapshot, mut private_key: PrivateKey) -> SignedSnapshot {
        let signature = private_key.sign(snapshot.hash.as_bytes());

        SignedSnapshot {
            snapshot,
            signature
        }
    }

    /// The name of the person who signed the snapshot (who is also the snapshot's author).
    pub fn signer(&self) -> &str {
        &self.snapshot.author
    }

    /// The hash of the inner snapshot.
    pub fn hash(&self) -> ObjectHash {
        self.snapshot.hash
    }

    /// Check if the signed snapshot is authentic.
    /// 
    /// This will return `false` if an error unrelated to verifying the signature arises.
    pub fn is_valid(&self) -> bool {
        self.signature.verify(self.snapshot.hash.as_bytes())
    }

    /// Verify the [`Signature`] on the [`SignedSnapshot`], returning
    /// the original [`Snapshot`] if the signature is valid and any
    /// errors from the signature verification process.
    pub fn verify(self) -> Result<Snapshot> {
        unwrap!(
            self.signature.check(self.snapshot.hash.as_bytes()),
            "failed to verify signature of snapshot {:?} with key {:?}",
            self.snapshot.hash, self.signature.key()
        );

        Ok(self.snapshot)
    }
}
