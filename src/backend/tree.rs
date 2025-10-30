use std::collections::BTreeMap;

use crate::backend::hash::CommitHash;

pub struct Link {
    pub parents: Vec<CommitHash>,
    pub children: Vec<CommitHash>
}

impl Link {
    pub fn empty() -> Link {
        Link {
            parents: vec![],
            children: vec![]
        }
    }
}

pub struct Tree {
    pub(crate) links: BTreeMap<CommitHash, Link>
}

impl Tree {
    pub fn empty() -> Tree {
        Tree { links: BTreeMap::new() }
    }

    fn get_mut(&mut self, hash: CommitHash) -> &mut Link {
        self.links.entry(hash).or_insert_with(Link::empty)
    }

    pub fn insert(&mut self, hash: CommitHash, parent: CommitHash) {
        let child = self.get_mut(hash);

        child.parents.push(parent);

        let parent = self.get_mut(parent);

        parent.children.push(hash);
    }

    pub fn insert_orphan(&mut self, hash: CommitHash) {
        self.links.insert(hash, Link::empty());
    }

    pub fn get_parents(&self, hash: CommitHash) -> Option<&Vec<CommitHash>> {
        self.links.get(&hash).map(|link| &link.parents)
    }

    pub fn get_children(&self, hash: CommitHash) -> Option<&Vec<CommitHash>> {
        self.links.get(&hash).map(|link| &link.children)
    }
}