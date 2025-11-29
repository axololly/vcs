use std::{fmt::{Debug, Display, Formatter}, ops::{Deref, DerefMut}, str::FromStr};

use serde::{Deserialize, Serialize};

pub type RawObjectHash = [u8; 20];

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ObjectHash(RawObjectHash);

impl ObjectHash {
    pub fn root() -> ObjectHash {
        ObjectHash([0u8; 20])
    }

    pub fn is_root(&self) -> bool {
        self == &Self::root()
    }

    pub fn full(&self) -> String {
        format!("{self:?}")
    }
}

impl From<RawObjectHash> for ObjectHash {
    fn from(value: RawObjectHash) -> Self {
        Self(value)
    }
}

impl From<&str> for ObjectHash {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}

impl Display for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full = format!("{self:?}");

        write!(f, "{}", &full[..10])
    }
}

impl Debug for ObjectHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Deref for ObjectHash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ObjectHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for ObjectHash {
    type Err = eyre::Report;
    
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(value)?;

        Ok(ObjectHash(bytes[..20].try_into()?))
    }
}