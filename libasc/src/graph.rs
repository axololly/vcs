use std::{collections::{HashMap, HashSet, VecDeque}, io::Write, path::Path};

use eyre::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{hash::ObjectHash, create_file, open_file};

type Parents = HashSet<ObjectHash>;

/// Represents the DAG (directed acylic graph) used to
/// store snapshots and their relationships.
/// 
/// This is implemented with a [`HashMap`] of nodes to parents.
#[derive(Debug, Default, Deserialize, Serialize)]
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
        Graph::default()
    }

    /// Connect a hash to a parent, either adding it to the DAG,
    /// or updating its state within the DAG.
    pub fn insert(&mut self, hash: ObjectHash, parent: ObjectHash) -> Result<()> {
        if !self.contains(parent) {
            bail!("parent hash {parent} does not exist in the graph.")
        }

        let parents = self.links.entry(hash).or_default();
        
        parents.insert(parent);

        Ok(())
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

    /// Return an iterator over all the hashes and their parents in the DAG in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (ObjectHash, &Parents)> {
        self.iter_hashes().map(|hash| (hash, self.get_parents(hash).unwrap()))
    }

    /// Return an iterator over all the hashes in the DAG in no particular order.
    pub fn iter_hashes(&self) -> impl Iterator<Item = ObjectHash> {
        self.links.keys().cloned()
    }

    /// Check if a hash is contained anywhere in the DAG.
    pub fn contains(&self, hash: ObjectHash) -> bool {
        self.links.contains_key(&hash)
    }

    /// Check if `a` is a descendant of `b` in the graph.
    pub fn is_descendant(&self, a: ObjectHash, b: ObjectHash) -> Result<bool> {
        let mut queue = VecDeque::new();

        queue.push_back(a);

        while let Some(next) = queue.pop_front() {
            if next == b {
                return Ok(true);
            }

            queue.extend(self.get_parents(next).unwrap().iter());
        }

        Ok(false)
    }

    /// Get the number of nodes in the DAG.
    pub fn size(&self) -> usize {
        self.links.len()
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Graph> {
        let fp = open_file(path)?;

        Ok(rmp_serde::from_read(fp)?)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut fp = create_file(path)?;

        let data = rmp_serde::to_vec(self)?;

        fp.write_all(&data)?;

        Ok(())
    }
}

impl From<HashMap<ObjectHash, HashSet<ObjectHash>>> for Graph {
    fn from(value: HashMap<ObjectHash, HashSet<ObjectHash>>) -> Self {
        Graph { links: value }
    }
}