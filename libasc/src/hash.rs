use std::{fmt::{Debug, Display, Formatter}, str::FromStr};

use eyre::bail;
use serde::{Deserialize, Serialize};

pub type RawObjectHash = [u8; 32];

/// A SHA-256 wrapper type used to uniquely identify content in the repository.
#[derive(Clone, Copy, Default, Deserialize, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub struct ObjectHash(#[serde(with = "serde_bytes")] RawObjectHash);

impl ObjectHash {
    /// Return the root hash, which is all zeroes.
    pub fn root() -> ObjectHash {
        ObjectHash([0u8; 32])
    }

    /// Check if this hash points to the root snapshot's hash.
    pub fn is_root(&self) -> bool {
        self == &Self::root()
    }

    /// Get the full hash as a hex string.
    /// 
    /// By default, in this type's implementation of [`Display`],
    /// the hash will be shrunk to 10 characters. This instead
    /// returns the full hash.
    pub fn full(&self) -> String {
        format!("{self:?}")
    }

    /// Get the individual bytes that make up this `ObjectHash`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<RawObjectHash> for ObjectHash {
    fn from(value: RawObjectHash) -> Self {
        Self(value)
    }
}

impl Display for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.full()[..10])
    }
}

impl Debug for ObjectHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for ObjectHash {
    type Err = eyre::Report;
    
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(value)?;

        if bytes.len() != 32 {
            bail!("invalid length of string hash: {} (expected 32)", bytes.len());
        }

        Ok(ObjectHash(bytes[..].try_into()?))
    }
}