use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::backend::hash::ObjectHash;

#[derive(Deserialize, Serialize)]
pub struct Entry {
    pub when: DateTime<Local>,
    pub hash: ObjectHash
}

pub struct Trash {
    pub entries: Vec<Entry>
}

impl Trash {
    pub fn new() -> Trash {
        Trash { entries: vec![] }
    }
    
    pub fn add(&mut self, hash: ObjectHash) {
        self.entries.push(Entry {
            when: Local::now(),
            hash
        });
    }

    pub fn contains(&self, hash: ObjectHash) -> bool {
        self.entries
            .iter()
            .filter(|e| e.hash == hash)
            .next()
            .is_some()
    }

    pub fn remove(&mut self, hash: ObjectHash) -> bool {
        let index = self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.hash == hash)
            .next()
            .map(|(i, _)| i);

        let Some(i) = index else {
            return false
        };
        
        self.entries.remove(i);
        
        true
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}