use std::{collections::{BTreeMap, HashMap}, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::hash::ObjectHash;

#[derive(Clone, Deserialize, Serialize)]
pub struct State {
    pub message: String,
    pub files: BTreeMap<PathBuf, ObjectHash>
}

/// Represents a snapshot independent of the history.
/// 
/// This also stores the snapshot from which the stash was made from.
#[derive(Clone, Deserialize, Serialize)]
pub struct Entry {
    pub state: State,
    pub basis: ObjectHash,
    pub timestamp: DateTime<Utc>
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Stash {
    entries: HashMap<usize, Entry>,
    count: usize
}

impl Stash {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_state(&mut self, state: State, basis: ObjectHash) -> usize {
        let entry = Entry {
            state,
            basis,
            timestamp: Utc::now()
        };

        self.entries.insert(self.count, entry);

        self.count += 1;

        self.count - 1
    }

    pub fn get_state(&self, id: usize) -> Option<&Entry> {
        self.entries.get(&id)
    }

    pub fn remove_state(&mut self, id: usize) -> Option<Entry> {
        self.entries.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &Entry)> {
        self.entries.iter().map(|(&k, v)| (k, v))
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn topmost_id(&self) -> Option<usize> {
        self.entries
            .keys()
            .cloned()
            .max()
    }

    pub fn topmost(&self) -> Option<&Entry> {
        let id = self.topmost_id()?;

        self.entries.get(&id)
    }
}
