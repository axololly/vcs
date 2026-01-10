use eyre::Result;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::{hash::ObjectHash, repository::Repository, unwrap, utils::{decompress_data, hash_raw_bytes}};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub enum Content {
    Literal(#[serde(with = "serde_bytes")] Vec<u8>),
    Delta(Delta)
}

impl Content {
    /// Obtain a `String` from [`Content`] by potentially resolving deltas.
    pub fn resolve(self, repo: &Repository) -> Result<String> {
        Ok(match self {
            Self::Literal(compressed) => {
                let decompressed = decompress_data(compressed)?;
                
                String::from_utf8(decompressed)?
            },

            Self::Delta(delta) => {
                let original = repo.fetch_string_content(delta.original)?;

                let source = original.resolve(repo)?;

                let resolved_bytes = unwrap!(
                    xdelta3::decode(&delta.edit, source.as_bytes()),
                    "failed to decode delta: {delta:?}"
                );

                String::from_utf8(resolved_bytes)?
            }
        })
    }
}