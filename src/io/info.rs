use std::{fs::File, io::{self, Write}, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// To be saved in `<root>/info`.
#[derive(Deserialize, Serialize)]
pub struct ProjectInfo {
    pub project_name: String,
    pub current_user: String,

    // Vec<(name, hash)>
    pub branches: Vec<(String, String)>,

    // Hash as string
    pub current_hash: String
}

#[derive(Debug, Error)]
pub enum ProjectInfoError {
    #[error("failed interaction with disk")]
    IO(#[from] io::Error),

    #[error("failed to serialise information")]
    Serialise(#[from] rmp_serde::encode::Error),

    #[error("failed to deserialise information")]
    Deserialise(#[from] rmp_serde::decode::Error)
}

pub type Result<T> = std::result::Result<T, ProjectInfoError>;

impl ProjectInfo {
    pub fn from_file(path: &Path) -> Result<ProjectInfo > {
        let fp = File::open(path)?;

        let info = rmp_serde::from_read(fp)?;

        Ok(info)
    }

    pub fn to_file(&self, path: &Path) -> Result<()> {
        let bytes = rmp_serde::to_vec(self)?;
        
        let mut fp = File::create(path)?;

        fp.write_all(&bytes)?;

        Ok(())
    }
}