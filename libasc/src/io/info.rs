use std::{collections::HashMap, io::Write, path::Path};

use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::{core::{hash::ObjectHash, stash::Stash}, utils::{create_file, open_file}};

#[derive(Deserialize, Serialize)]
pub struct ProjectInfo {
    pub project_name: String,
    pub project_code: ObjectHash,
    pub current_user: String,
    pub branches: HashMap<String, ObjectHash>,
    pub current_hash: ObjectHash,
    pub stashes: Vec<Stash>
}

impl ProjectInfo {
    pub fn from_file(path: impl AsRef<Path>) -> Result<ProjectInfo> {
        let fp = open_file(path)?;

        let info = rmp_serde::from_read(fp)?;

        Ok(info)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> eyre::Result<()> {
        let bytes = rmp_serde::to_vec(self)?;
        
        let mut fp = create_file(path)?;

        fp.write_all(&bytes)?;

        Ok(())
    }
}