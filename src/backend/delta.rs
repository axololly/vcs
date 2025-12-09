use serde::{Deserialize, Serialize};
use xdelta3::{decode, encode};

use crate::backend::hash::ObjectHash;

#[derive(Deserialize, Serialize)]
pub struct Delta {
    original: ObjectHash,
    
    #[serde(with = "serde_bytes")]
    edit: Vec<u8>
}