use std::{collections::{HashMap, HashSet, VecDeque}, io::Write, path::Path};

use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{create_file, hash::ObjectHash, open_file, unwrap};

type Parents = HashSet<ObjectHash>;

type RawGraph = HashMap<ObjectHash, Parents>;

/// Represents the DAG (directed acylic graph) used to
/// store snapshots and their relationships.
/// 
/// This is implemented with a [`HashMap`] of nodes to parents.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Graph {
    links: RawGraph
}

impl Graph {
    /// Create an empty [`Graph`].
    pub fn new() -> Graph {
        Graph::default()
    }

    /// Connect a hash to a parent, either adding it to the DAG,
    /// or updating its state within the DAG.
    /// 
    /// If the parent is not present, this will insert an orphan for the parent.
    pub fn insert(&mut self, hash: ObjectHash, parent: ObjectHash) {
        if !self.contains(parent) {
            self.insert_orphan(parent);
        }

        let parents = self.links.entry(hash).or_default();
        
        parents.insert(parent);
    }

    /// Insert a hash with no parents.
    /// 
    /// This is usually used for creating a root snapshot.
    pub fn insert_orphan(&mut self, hash: ObjectHash) {
        self.links.insert(hash, HashSet::new());
    }

    /// Remove a hash from the DAG, returning the parents of the removed hash.
    pub fn remove(&mut self, hash: ObjectHash) -> Option<Parents> {
        let node_parents = self.links.remove(&hash);

        for parents in self.links.values_mut() {
            parents.remove(&hash);
        }

        node_parents
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
            
            let parents = unwrap!(
                self.get_parents(next),
                "failed to get parents of hash {next:?}"
            );

            queue.extend(parents.iter());
        }

        Ok(false)
    }

    /// Get the number of nodes in the DAG.
    pub fn size(&self) -> usize {
        self.links.len()
    }

    /// Return a [`HashMap`] inverting all the links in this [`Graph`].
    /// 
    /// This clones the entire graph for convenience.
    pub fn invert(&self) -> HashMap<ObjectHash, HashSet<ObjectHash>> {
        let mut map: HashMap<ObjectHash, HashSet<ObjectHash>> = HashMap::new();

        for (hash, parents) in self.iter() {
            for &parent in parents {
                map.entry(parent).or_default().insert(hash);
            }
        }

        map
    }

    pub fn extend(&mut self, other: &Graph) {
        for (hash, parents) in other.iter() {
            for &parent in parents {
                self.insert(hash, parent);
            }
        }
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

impl From<RawGraph> for Graph {
    fn from(value: RawGraph) -> Self {
        Graph { links: value }
    }
}

impl From<Graph> for RawGraph {
    fn from(value: Graph) -> RawGraph {
        value.links
    }
}
