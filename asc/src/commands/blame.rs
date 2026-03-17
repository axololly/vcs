use std::{collections::VecDeque, rc::Rc};

use chrono::{DateTime, Utc};
use eyre::Result;
use relative_path::RelativePathBuf;
use unicode_width::UnicodeWidthStr;

use libasc::{hash::ObjectHash, repository::Repository, unwrap};

// TODO: write your own
use blame_rs::{BlameRevision, blame};

#[derive(clap::Args)]
pub struct Args {
    /// The path to perform the blame on.
    path: RelativePathBuf
}

#[derive(Debug)]
struct CommitInfo<'a> {
    hash: ObjectHash,
    author: &'a str,
    timestamp: DateTime<Utc>,
}

struct SnapshotData {
    hash: ObjectHash,
    author: String,
    timestamp: DateTime<Utc>,
    content: String
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    if !repo.staged_files.contains(&args.path) {
        eprintln!("Path {} is not staged in the repository.", &args.path);
    }

    let mut queue: VecDeque<ObjectHash> = VecDeque::new();

    queue.push_back(repo.current_hash);

    let mut snapshots: Vec<SnapshotData> = vec![];

    while let Some(next) = queue.pop_front() {
        let parents = unwrap!(
            repo.history.get_parents(next),
            "could not get hash of {next:?} in repository"
        );
        
        if parents.is_empty() {
            continue;
        }

        queue.extend(parents);

        let snapshot = repo.fetch_snapshot(next)?;

        let Some(&content_hash) = snapshot.files.get(&args.path) else { continue };

        snapshots.push(SnapshotData {
            hash: snapshot.hash,
            author: snapshot.author.to_string(),
            timestamp: snapshot.timestamp,
            content: repo.fetch_string_content(content_hash)?
        });
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
