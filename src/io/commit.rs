use std::{collections::BTreeMap, fs::File, io::{Read, Write}, path::{Path, PathBuf}, str::FromStr};

use chrono::DateTime;
use eyre::eyre;
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::{Deserialize, Serialize};

use crate::backend::{commit::{Commit, CommitHeader}, hash::CommitHash};

#[derive(Deserialize, Serialize)]
pub struct RawCommitHeader {
    hash: String,
    author: String,
    message: String,
    timestamp: u64
}

fn read_exact(fp: &mut File, len: usize) -> eyre::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];

    fp.read_exact(&mut buf)?;

    Ok(buf)
}

fn compress_data(input: &[u8]) -> Vec<u8> {
    compress_to_vec(input, 6)
}

fn decompress_data(input: &[u8]) -> eyre::Result<Vec<u8>> {
    let buf = decompress_to_vec(input)
        .map_err(|e| eyre!("failed to decompress data: {e}"))?;

    Ok(buf)
}

fn _intermediate_read(path: &Path) -> eyre::Result<(CommitHeader, File)> {
    let fp = File::open(path)?;

    let raw_header: RawCommitHeader = rmp_serde::from_read(&fp)?;

    let timestamp = DateTime::from_timestamp_secs(raw_header.timestamp as i64).unwrap().into();

    let header = CommitHeader {
        hash: CommitHash::from_str(&raw_header.hash)?,
        author: raw_header.author,
        message: raw_header.message,
        timestamp
    };

    Ok((header, fp))
}

impl CommitHeader {
    pub fn from_file(path: &Path) -> eyre::Result<CommitHeader> {
        let (header, _) = _intermediate_read(path)?;

        Ok(header)
    }
}

impl Commit {
    pub fn from_file(path: &Path) -> eyre::Result<Commit> {
        let (header, mut fp) = _intermediate_read(path)?;

        let file_info: Vec<(String, usize)> = rmp_serde::from_read(&fp)?;

        let mut files = BTreeMap::new();

        for (file, length) in file_info {
            let compressed = read_exact(&mut fp, length)?;

            let content = decompress_data(&compressed)?;

            files.insert(PathBuf::from(file), String::from_utf8(content)?);
        }

        Ok(Commit { header, files })
    }

    pub fn to_file(&self, path: &Path) -> eyre::Result<()> {
        let raw_header = RawCommitHeader {
            author: self.header.author.clone(),
            message: self.header.message.clone(),
            hash: self.header.hash.to_string(),
            timestamp: self.header.timestamp.timestamp() as u64
        };

        let raw_header_bytes = rmp_serde::to_vec(&raw_header)?;

        let mut file_info: Vec<(String, usize)> = vec![];

        let mut content_blob: Vec<u8> = Vec::with_capacity(4096);

        for (path, content) in &self.files {
            let compressed = compress_data(content.as_bytes());

            content_blob.extend_from_slice(&compressed);

            file_info.push((path.to_string_lossy().to_string(), compressed.len()));
        }

        let mut fp = File::create(path)?;
        
        fp.write_all(&raw_header_bytes)?;

        let file_info_bytes = rmp_serde::to_vec(&file_info)?;

        fp.write_all(&file_info_bytes)?;

        fp.write_all(&content_blob)?;

        Ok(())
    }
}