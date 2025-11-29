use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use super::hash::ObjectHash;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Snapshot {
    pub hash: ObjectHash,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub files: HashMap<PathBuf, ObjectHash>
}