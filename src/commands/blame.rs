use std::{collections::VecDeque, path::PathBuf, rc::Rc};

use chrono::{DateTime, Local};
use clap::Args as A;
use eyre::{Result, bail};
use unicode_width::UnicodeWidthStr;

use crate::backend::{hash::ObjectHash, repository::Repository};

use blame_rs::{BlameRevision, blame};

#[derive(A)]
pub struct Args {
    /// The path to perform the blame on.
    path: PathBuf
}

#[derive(Debug)]
struct CommitInfo<'a> {
    hash: ObjectHash,
    author: &'a str,
    timestamp: DateTime<Local>,
}

struct SnapshotData {
    hash: ObjectHash,
    author: String,
    timestamp: DateTime<Local>,
    content: String
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let path = &args.path;

    if !repo.staged_files.contains(path) {
        bail!("path {} is not staged in the repository", path.display());
    }

    let mut queue: VecDeque<ObjectHash> = VecDeque::new();

    queue.push_back(repo.current_hash);

    let mut snapshots: Vec<SnapshotData> = vec![];

    while let Some(next) = queue.pop_front() {
        if next.is_root() {
            break;
        }

        let snapshot = repo.fetch_snapshot(next)?;

        let Some(&content_hash) = snapshot.files.get(path) else { continue };

        snapshots.push(SnapshotData {
            hash: snapshot.hash,
            author: snapshot.author,
            timestamp: snapshot.timestamp,
            content: repo.fetch_string_content(content_hash)?.resolve(&repo)?
        });

        let parents = repo.history
            .get_parents(next)
            .unwrap()
            .iter()
            .cloned();

        queue.extend(parents);
    }

    let mut revisions: Vec<BlameRevision<CommitInfo>> = vec![];

    for data in &snapshots {
        revisions.push(BlameRevision {
            content: &data.content,
            metadata: Rc::new(CommitInfo {
                hash: data.hash,
                author: &data.author,
                timestamp: data.timestamp
            })
        });
    }

    let result = blame(&revisions)?;

    let max_author_width = result
        .lines()
        .iter()
        .fold(0, |total, line| {
            total + line.revision_metadata.author.width()
        });

    for line in result.lines() {
        let data = line.revision_metadata.clone();

        let mut author = data.author.to_string();

        for _ in 0 .. (max_author_width - author.width()) {
            author.push(' ');
        }

        println!("{}    {}    {author}    {}", data.hash, data.timestamp, line.content);
    }
    
    Ok(())
}