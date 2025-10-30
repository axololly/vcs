use std::path::Path;

use eyre::{eyre, Result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("root directory `.acs` missing")]
    RootDirMissing
}

fn ensure_repo_exists() -> Result<()> {
    let path = Path::new(".acs");

    if !path.exists() {
        return Err(eyre!("Missing path: `.acs`"));
    }
    
    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    ensure_repo_exists()?;

    println!("Directory exists!");

    Ok(())
}