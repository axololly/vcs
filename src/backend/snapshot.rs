use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

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
    pub files: HashMap<PathBuf, ObjectHash>
}