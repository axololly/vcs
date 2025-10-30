use std::{fs::File, io::Write as _, path::Path};

use crate::backend::{hash::RawCommitHash, tree::Tree};

impl Tree {
    pub fn from_file(path: &Path) -> eyre::Result<Tree> {
        let fp = File::open(path)?;

        let raw_links: Vec<(RawCommitHash, Vec<RawCommitHash>)> = rmp_serde::from_read(fp).unwrap();

        let mut tree = Tree::empty();

        for (current_hash, parent_hashes) in raw_links {
            for parent in parent_hashes {
                tree.insert(current_hash.into(), parent.into());
            }
        }

        Ok(tree)
    }

    pub fn to_file(&self, path: &Path) -> eyre::Result<()> {
        let mut fp = File::open(path)?;

        let mut raw_links: Vec<(RawCommitHash, Vec<RawCommitHash>)> = vec![];

        for (&current_hash, link) in &self.links {
            let raw_parents = link.parents
                .iter()
                .map(|&h| *h)
                .collect();

            raw_links.push((*current_hash, raw_parents));
        }

        let data = rmp_serde::to_vec(&raw_links).unwrap();

        fp.write_all(&data)?;

        Ok(())
    }
}