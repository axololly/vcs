use std::{collections::{HashMap, HashSet, VecDeque}, path::PathBuf};

use chrono::Local;
use clap::Args as A;

use eyre::{Result, bail};
use sha1::{Digest, Sha1};
use threeway_merge::{merge_strings, MergeOptions};

use crate::{backend::{action::Action, graph::Graph, hash::ObjectHash, repository::Repository, snapshot::Snapshot}, unwrap, utils::get_content_from_editor};

fn nodes_to_root(graph: &Graph, node: ObjectHash) -> HashMap<ObjectHash, usize> {
    let mut queue: VecDeque<(ObjectHash, usize)> = VecDeque::new();
    let mut distances = HashMap::new();

    queue.push_back((node, 0));

    while let Some((next, distance)) = queue.pop_front() {
        let parents = graph.get_parents(next).unwrap();

        for parent in parents {
            let new_distance = if let Some(&existing_distance) = distances.get(parent) {
                distance.max(existing_distance) + 1
            }
            else {
                distance + 1
            };

            queue.push_back((*parent, new_distance));

            distances.insert(*parent, new_distance);
        }
    }

    distances
}

#[derive(Debug)]
#[allow(dead_code)]
enum Ancestry {
    Inclusive(ObjectHash),
    Exclusive(ObjectHash)
}

fn find_closest_common_ancestor(graph: &Graph, u: ObjectHash, v: ObjectHash) -> Option<Ancestry> {
    let parents_u = nodes_to_root(graph, u);

    // v is a parent of u
    if parents_u.contains_key(&v) {
        return Some(Ancestry::Inclusive(v));
    }

    let parents_v = nodes_to_root(graph, v);

    // u is a parent of v
    if parents_v.contains_key(&u) {
        return Some(Ancestry::Inclusive(u));
    }

    parents_u
        .iter()
        .filter_map(|(item, count)| {
            parents_v
                .get(item)
                .map(|count2| (item, count.max(count2)))
        })
        .min_by(|(_, u_depth), (_, v_depth)| u_depth.cmp(v_depth))
        .map(|(&k, _)| Ancestry::Exclusive(k))
}

pub fn prettify_hash(repo: &Repository, hash: ObjectHash) -> String {
    if let Some(branch_name) = repo.branch_from_hash(hash) {
        branch_name.to_string()
    }
    else {
        hash.to_string()
    }
}

enum ContentType {
    Get(String),
    Fetch(ObjectHash)
}

enum MergeType {
    Clean(ContentType),
    Dirty(String)
}

#[derive(A)]
pub struct Args {
    /// The version to merge onto the current snapshot.
    version: String,

    /// The message to go with the merge commit.
    #[arg(short, long)]
    message: Option<String>,

    /// The interactive editor to use to get a message if
    /// none was given as an argument.
    #[arg(short, long)]
    editor: Option<String>,

    /// The version to use as the base for the merge
    /// instead of the closest common ancestor.
    #[arg(long)]
    baseline: Option<String>,

    /// Do not automatically make a commit after completing the merge.
    #[arg(long)]
    no_commit: bool
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;

    if repo.has_unsaved_changes()? {
        bail!("cannot merge with unsaved changes");
    }

    let target = repo.normalise_version(&args.version)?;

    let ancestor = if let Some(version) = args.baseline {
        repo.normalise_version(&version)?
    }
    else {
        let u = repo.current_hash;
        let v = target;

        let ancestry = unwrap!(
            find_closest_common_ancestor(&repo.history, u, v),
            "could not identify a common ancestor for snapshots {u} and {v}"
        );

        match ancestry {
            // Fast-forward, but we're already at the child, so no changes made
            Ancestry::Inclusive(parent) if parent != repo.current_hash => {
                println!("already on the child of the fast-forward, therefore no changes have been made.");
                
                return Ok(());
            }

            // Fast-forward, but we're at the parent, so make changes
            Ancestry::Inclusive(_) => {
                let snapshot = repo.fetch_snapshot(target)?;

                repo.replace_cwd_with_snapshot(&snapshot)?;

                repo.current_hash = target;

                println!("Fast-forwaded to {target}");

                repo.save()?;

                return Ok(());
            }

            Ancestry::Exclusive(parent) => parent
        }
    };

    let base_files = repo.fetch_snapshot(ancestor)?.files;

    let options = MergeOptions {
        base_label: Some("original".to_string()),
        ours_label: Some(prettify_hash(&repo, repo.current_hash)),
        theirs_label: Some(prettify_hash(&repo, target)),

        .. MergeOptions::default()
    };
    
    let our_files = repo.fetch_current_snapshot()?.files;
    let our_paths: HashSet<&PathBuf> = HashSet::from_iter(our_files.keys());

    let their_files = repo.fetch_snapshot(target)?.files;
    let their_paths = HashSet::from_iter(their_files.keys());

    let mut merged_files: HashMap<PathBuf, MergeType> = HashMap::new();

    // Any files that are in either our version or their version but not in both
    // can go in the final version perfectly fine.
    for path in our_paths.symmetric_difference(&their_paths) {
        let &hash = our_files.get(path.as_path())
            .or(their_files.get(path.as_path()))
            .unwrap();

        merged_files.insert(path.to_path_buf(), MergeType::Clean(ContentType::Fetch(hash)));
    }

    // Files that exist in both will have merge conflicts that need resolving.
    for path in our_paths.intersection(&their_paths) {
        let ours = repo.fetch_string_content(our_files[path.as_path()])?;
        let theirs = repo.fetch_string_content(their_files[path.as_path()])?;

        let base = base_files
            .get(path.as_path())
            .map(|&content_hash| repo.fetch_string_content(content_hash))
            .unwrap_or(Ok(String::new()))?;

        let merge_result = merge_strings(&base, &ours, &theirs, &options)?;

        let merge_type = if merge_result.is_clean_merge() {
            MergeType::Clean(ContentType::Get(merge_result.content))
        }
        else {
            MergeType::Dirty(merge_result.content)
        };

        merged_files.insert(path.to_path_buf(), merge_type);
    }

    let mut hasher = Sha1::new();
    
    let author = repo.current_user.clone();

    hasher.update(&author);

    let message = if let Some(msg) = args.message {
        msg
    }
    else {
        let editor = args.editor.unwrap_or(
            unwrap!(
                std::env::var("EDITOR"),
                "environment variable 'EDITOR' is not set."
            )
        );

        let snapshot_message_path = &repo.main_dir().join("SNAPSHOT_MESSAGE");

        get_content_from_editor(&editor, snapshot_message_path)?
    };

    hasher.update(&message);

    let mut files = HashMap::new();

    let mut dirty_files: Vec<PathBuf> = vec![];

    let mut is_clean_merge = true;

    for (path, merge) in merged_files {
        let content = match merge {
            MergeType::Clean(v) => v,

            MergeType::Dirty(s) => {
                dirty_files.push(path.clone());

                is_clean_merge = false;

                ContentType::Get(s)
            }
        };

        let hash = match content {
            ContentType::Get(string) => {
                hasher.update(&string);

                repo.save_string_content(&string)?
            }

            ContentType::Fetch(hash) => hash
        };

        files.insert(path, hash);
    }

    let now = Local::now();

    hasher.update(now.timestamp().to_be_bytes());

    let raw_hash: [u8; 20] = hasher.finalize().into();

    let hash = ObjectHash::from(raw_hash);

    let snapshot = Snapshot {
        hash,
        author,
        message,
        files,
        timestamp: now
    };

    repo.replace_cwd_with_snapshot(&snapshot)?;

    if !is_clean_merge {
        let new_staged_files: Vec<PathBuf> = repo.staged_files
            .iter()
            .filter_map(|p| dirty_files.contains(p).then(|| p.clone()))
            .collect();

        repo.staged_files = new_staged_files;
    }

    if args.no_commit {
        println!("Finished merge but snapshot must be committed manually.");
        
        return Ok(());
    }

    repo.save_snapshot(&snapshot)?;

    repo.history.remove(snapshot.hash);

    repo.history.insert(snapshot.hash, repo.current_hash);
    repo.history.insert(snapshot.hash, target);

    if let Some(name) = repo.current_branch() {
        repo.branches.insert(name.to_string(), snapshot.hash);
    }

    repo.action_history.push(
        Action::CreateSnapshot {
            hash,
            parents: vec![repo.current_hash, target]
        }
    );

    repo.action_history.push(
        Action::SwitchVersion {
            before: repo.current_hash,
            after: snapshot.hash
        }
    );

    let previous = repo.current_hash;

    repo.current_hash = snapshot.hash;

    repo.save()?;

    println!("Merged {previous} and {target}.");
    
    println!("New commit: {}", snapshot.hash);
    
    Ok(())
}