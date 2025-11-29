use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::backend::hash::ObjectHash;

#[derive(Deserialize, Serialize)]
pub struct Graph {
    pub links: HashMap<ObjectHash, HashSet<ObjectHash>>
}

impl Graph {
    pub fn empty() -> Graph {
        Graph { links: HashMap::new() }
    }

    pub fn hashes(&self) -> impl Iterator<Item = ObjectHash> {
        self.links.keys().cloned()
    }

    fn get_mut(&mut self, hash: ObjectHash) -> &mut HashSet<ObjectHash> {
        self.links.entry(hash).or_default()
    }

    pub fn insert(&mut self, hash: ObjectHash, parent: ObjectHash) {
        let parents = self.get_mut(hash);

        parents.insert(parent);
    }

    pub fn insert_orphan(&mut self, hash: ObjectHash) {
        self.links.insert(hash, HashSet::new());
    }

    pub fn remove(&mut self, hash: ObjectHash) {
        if let Some(parents) = self.get_parents(hash).cloned() {
            for parent in parents {
                self.get_mut(parent).remove(&hash);
            }
        }

        self.links.remove(&hash);
    }

    pub fn get_parents(&self, hash: ObjectHash) -> Option<&HashSet<ObjectHash>> {
        self.links.get(&hash)
    }
}