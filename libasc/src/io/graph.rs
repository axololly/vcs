use std::{fs::File, io::Write, path::Path};

use eyre::Result;

use crate::{core::graph::Graph, utils::create_file};

impl Graph {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Graph> {
        let fp = File::open(path)?;

        Ok(rmp_serde::from_read(fp)?)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut fp = create_file(path)?;

        let data = rmp_serde::to_vec(self)?;

        fp.write_all(&data)?;

        Ok(())
    }
}