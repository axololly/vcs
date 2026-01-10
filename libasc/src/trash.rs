use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::hash::ObjectHash;

// For documentation purposes.
#[allow(unused_imports)]
use crate::graph::Graph;

/// For _how_ an [`ObjectHash`] is included in the trash.
pub enum TrashStatus {
    /// The [`ObjectHash`] was found directly in the trash.
    Direct,

    /// The [`ObjectHash`] was not found directly in the trash,
    /// but it was a child of a hash directly in the trash.
    Indirect(ObjectHash)
}

#[derive(Deserialize, Serialize)]
pub struct Entry {
    pub when: DateTime<Local>,
    pub hash: ObjectHash
}

/// A rubbish bin meant exclusively for snapshot hashes.
#[derive(Default, Deserialize, Serialize)]
pub struct Trash {
    entries: Vec<Entry>
}

impl Trash {
    /// Create an empty [`Trash`].
    pub fn new() -> Trash {
        Trash { entries: vec![] }
    }
    
    /// Directly add an [`ObjectHash`] to the trash.
    /// 
    /// This does not include the children of the [`ObjectHash`],
    /// as this information is in [`Graph`].
    pub fn add(&mut self, hash: ObjectHash) {
        self.entries.push(Entry {
            when: Local::now(),
            hash
        });
    }

    /// Check if an [`ObjectHash`] is directly contained in the trash.
    /// 
    /// If an [`ObjectHash`] is **indirectly** (see [`TrashStatus::Indirect`])
    /// included in the trash, this will return `false`.
    pub fn contains(&self, hash: ObjectHash) -> bool {
        self.entries
            .iter()
            .any(|e| e.hash == hash)
    }

    /// Remove an [`ObjectHash`] from the trash.
    pub fn remove(&mut self, hash: ObjectHash) -> bool {
        let index = self.entries
            .iter()
            .enumerate()
            .find(|(_, e)| e.hash == hash)
            .map(|(i, _)| i);

        let Some(i) = index else {
            return false
        };
        
        self.entries.remove(i);
        
        true
    }

    /// Check if the trash is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check how many things are in the trash.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Get an internal reference to the entries of the trash.
    pub fn entries(&self) -> &[Entry] {
        self.entries.as_slice()
    }
}