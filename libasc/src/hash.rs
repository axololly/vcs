use std::{fmt::{Debug, Display, Formatter}, hash::{DefaultHasher, Hasher}, str::FromStr};

use eyre::bail;
use rateless_tables::Symbol;
use serde::{Deserialize, Serialize};

pub type RawObjectHash = [u8; 32];

/// A SHA-256 wrapper type used to uniquely identify content in the repository.
#[derive(Clone, Copy, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
pub struct ObjectHash(#[serde(with = "serde_bytes")] RawObjectHash);

impl ObjectHash {
    /// Get the full hash as a hex string.
    /// 
    /// By default, in this type's implementation of [`Display`],
    /// the hash will be shrunk to 10 characters. This instead
    /// returns the full hash.
    pub fn full(&self) -> String {
        format!("{self:?}")
    }

    /// Get the individual bytes that make up this `ObjectHash`.
    pub fn as_bytes(&self) -> &RawObjectHash {
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

impl Symbol for ObjectHash {
    fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        hasher.write(self.as_bytes());

        hasher.finish()
    }
    
    fn xor(&self, other: &Self) -> Self {
        let mut result = *self.as_bytes();

        for (i, &v) in other.as_bytes().iter().enumerate() {
            result[i] ^= v;
        }

        result.into()
    }
}
