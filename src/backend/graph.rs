use std::collections::{HashMap, HashSet, VecDeque};

use eyre::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::backend::hash::ObjectHash;

type Parents = HashSet<ObjectHash>;

/// Represents the DAG (directed acylic graph) used to
/// store snapshots and their relationships.
/// 
/// This is implemented with a [`HashMap`] of nodes to parents.
#[derive(Deserialize, Serialize)]
pub struct Graph {
    links: HashMap<ObjectHash, Parents>
}

impl Graph {
    /// Create a [`Graph`] with a root hash as the first node.
    pub fn new() -> Graph {
        let mut graph = Graph::empty();

        graph.insert_orphan(ObjectHash::root());

        graph
    }
    
    /// Create an empty [`Graph`].
    pub fn empty() -> Graph {
        Graph { links: HashMap::new() }
    }

    fn get_mut(&mut self, hash: ObjectHash) -> &mut Parents {
        self.links.entry(hash).or_default()
    }

    /// Connect a hash to a parent, either adding it to the DAG,
    /// or updating its state within the DAG.
    pub fn insert(&mut self, hash: ObjectHash, parent: ObjectHash) {
        self.get_mut(hash).insert(parent);
    }

    /// Insert a hash with no parents.
    /// 
    /// This is usually used for creating a root snapshot.
    pub fn insert_orphan(&mut self, hash: ObjectHash) {
        self.links.insert(hash, HashSet::new());
    }

    /// Remove a hash from the DAG but **DOES NOT** remove itself from
    /// its children for performance reasons (increasing from `O(1)` to `O(n)`).
    /// 
    /// This also returns the parents of the removed hash.
    pub fn remove(&mut self, hash: ObjectHash) -> Option<Parents> {
        self.links.remove(&hash)
    }

    /// Perform [`Graph::remove`] on the hash, then [`Graph::insert`]
    /// with the hash and the new parents.
    pub fn upsert(&mut self, hash: ObjectHash, new_parents: impl IntoIterator<Item = ObjectHash>) -> Option<Parents> {
        let removed = self.links.remove(&hash);
        
        self.links.insert(hash, new_parents.into_iter().collect());

        removed
    }

    /// Get the parents of a hash in the DAG, if the hash is present.
    pub fn get_parents(&self, hash: ObjectHash) -> Option<&Parents> {
        self.links.get(&hash)
    }

    /// Return an iterator over all the hashes and their parents in the DAG.
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectHash, &Parents)> {
        self.links.iter()
    }

    /// Return an iterator over all the hashes in the DAG.
    pub fn iter_hashes(&self) -> impl Iterator<Item = ObjectHash> {
        self.links.keys().cloned()
    }

    /// Check if a hash is contained anywhere in the DAG.
    pub fn contains(&self, hash: ObjectHash) -> bool {
        self.links.contains_key(&hash)
    }

    /// Check if `a` is a descendant of `b` in the graph.
    pub fn is_descendant(&self, a: ObjectHash, b: ObjectHash) -> bool {
        let mut queue = VecDeque::new();

        queue.push_back(a);

        while let Some(next) = queue.pop_front() {
            if next == b {
                return true;
            }

            queue.extend(self.get_parents(next).unwrap().iter());
        }

        false
    }

    /// Replace a hash in the tree with another hash,
    /// provided the hash does not already exist in the graph.
    pub fn rename(&mut self, old: ObjectHash, new: ObjectHash) -> Result<usize> {
        if self.links.contains_key(&new) {
            bail!("hash {new} already exists in the graph.");
        }
        
        let Some(parents) = self.links.remove(&old) else {
            bail!("hash {old} does not exist in the graph.")
        };

        self.links.insert(new, parents);

        let mut modified = 0;

        for parents in self.links.values_mut() {
            if parents.remove(&old) {
                modified += 1;
                
                parents.insert(new);
            }
        }

        Ok(modified)
    }
}