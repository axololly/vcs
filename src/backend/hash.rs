use std::{error::Error, fmt::{Debug, Display, Formatter}, ops::{Deref, DerefMut}, str::FromStr};

pub type RawCommitHash = [u8; 20];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitHash(RawCommitHash);

impl CommitHash {
    pub fn root() -> CommitHash {
        CommitHash::from(ROOT_HASH_STR)
    }
}

impl From<RawCommitHash> for CommitHash {
    fn from(value: RawCommitHash) -> Self {
        Self(value)
    }
}

impl From<&str> for CommitHash {
    fn from(value: &str) -> Self {
        Self::from_str(value).unwrap()
    }
}

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::with_capacity(40);

        fn to_char(x: u8) -> char {
            if x >= 10 {
                (b'a' + x - 10) as char
            }
            else {
                (b'0' + x) as char
            }
        }
        
        for byte in self.0 {
            // Upper bits
            result.push(to_char(byte >> 4));
            
            // Lower bits
            result.push(to_char(byte & 0xf));
        }

        write!(f, "{result}")
    }
}

impl Debug for CommitHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Deref for CommitHash {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CommitHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct CommitHashDecodeError(String);

impl FromStr for CommitHash {
    type Err = CommitHashDecodeError;
    
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.to_string();

        if value.len() != 40 {
            return Err(CommitHashDecodeError(value));
        }
        
        let mut commit_hash = [0u8; 20];

        for i in 0 .. 20 {
            let chunk = &value[i * 2 .. (i + 1) * 2];

            let Ok(hex) = u8::from_str_radix(chunk, 16) else {
                return Err(CommitHashDecodeError(value));
            };

            commit_hash[i] = hex;
        }

        Ok(CommitHash(commit_hash))
    }
}

impl Display for CommitHashDecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommitHashDecodeError: failed to decode hash: {:?}", self.0)
    }
}

impl Error for CommitHashDecodeError {}

// Sha1 hash for the string "root". Used for every root commit.
pub static ROOT_HASH_STR: &str = "dc76e9f0c0006e8f919e0c515c66dbba3982f785";