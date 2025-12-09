use eyre::Result;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::{backend::{hash::ObjectHash, repository::Repository}, unwrap, utils::{decompress_data, hash_raw_bytes}};

#[derive(Debug, Deserialize, Serialize)]
pub struct Delta {
    pub original: ObjectHash,
    
    #[serde(with = "serde_bytes")]
    pub edit: Vec<u8>
}

impl Delta {
    pub fn new_unchecked(old: &str, new: &str) -> Delta {
        let original = hash_raw_bytes(old);

        let edit = xdelta3::encode(new.as_bytes(), old.as_bytes())
            .expect("failed to encode using xdelta3");

        Delta {
            original,
            edit
        }
    }

    pub fn new(old: &str, new: &str, min_similarity: f32) -> Option<Delta> {
        let diff = TextDiff::from_words(old, new);

        (diff.ratio() >= min_similarity).then(|| {
            Delta::new_unchecked(old, new)
        })
    }
}

#[derive(Deserialize, Serialize)]
pub enum RawContent {
    Literal(#[serde(with = "serde_bytes")] Vec<u8>),
    Delta(Delta)
}

impl RawContent {
    pub fn into_content(self) -> Result<Content> {
        Ok(match self {
            Self::Literal(bytes) => {
                let decompressed = decompress_data(bytes)?;
                
                let string = String::from_utf8(decompressed)?;
                
                Content::Literal(string)
            },

            Self::Delta(delta) => Content::Delta(delta)
        })
    }
}

#[derive(Deserialize, Serialize)]
pub enum Content {
    Literal(String),
    Delta(Delta)
}

impl Content {
    pub fn resolve(self, repo: &Repository) -> Result<String> {
        Ok(match self {
            Self::Literal(string) => string,

            Self::Delta(delta) => {
                let original = repo.fetch_string_content(delta.original)?;

                let source = original.resolve(repo)?;

                let resolved_bytes = unwrap!(
                    xdelta3::decode(&delta.edit, source.as_bytes()),
                    "failed to decode delta: {delta:?}"
                );

                let resolved = String::from_utf8(resolved_bytes)?;

                resolved
            }
        })
    }
}