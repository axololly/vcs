use std::collections::{HashMap, HashSet, VecDeque};

use clap::Subcommand;
use eyre::{Result, bail, eyre};

use crate::backend::{hash::ObjectHash, repository::Repository, trash::{Entry, Trash}};

#[derive(Subcommand)]
pub enum Subcommands {
    /// Move a snapshot to the trash.
    Add {
        /// The version to trash.
        version: String
    },

    /// Remove a snapshot from the trash.
    Recover {
        /// The version to recover.
        version: String
    },

    /// List snapshots that were added in the trash.
    List {
        /// The specific version to list implicitly trashed snapshots of.
        version: Option<String>,

        /// The number of entries to list, or all if unspecified.
        limit: Option<usize>
    }
}

type Graph = HashMap<ObjectHash, HashSet<ObjectHash>>;

fn count_subnodes(graph: &Graph, node: ObjectHash) -> usize {
    graph[&node]
        .iter()
        .map(|&node| count_subnodes(graph, node))
        .sum()
}

fn list_all_subnodes(graph: &Graph, hash: ObjectHash) -> Vec<ObjectHash> {
    let mut v = vec![hash];
    
    for &child in &graph[&hash] {
        v.extend(&list_all_subnodes(graph, child));
    }

    v
}

fn is_descendant(graph: &Graph, a: ObjectHash, b: ObjectHash) -> bool {
    let mut queue = VecDeque::new();

    queue.push_back(a);

    while let Some(next) = queue.pop_front() {
        if next == b {
            return true;
        }

        queue.extend(graph[&next].iter());
    }

    false
}

enum TrashStatus {
    Direct,
    Indirect(ObjectHash)
}

fn hash_in_trash(hash: ObjectHash, graph: &Graph, trash: &Trash) -> Option<TrashStatus> {
    if trash.contains(hash) {
        return Some(TrashStatus::Direct);
    }

    for Entry { hash: trash_hash, .. } in &trash.entries {
        if is_descendant(graph, hash, *trash_hash) {
            return Some(TrashStatus::Indirect(*trash_hash));
        }
    }

    None
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    let inverse_links = {
        let mut map: HashMap<ObjectHash, HashSet<ObjectHash>> = HashMap::new();

        for (&hash, parents) in repo.history.links.iter() {
            for &parent in parents {
                map.entry(parent).or_default().insert(hash);
            }
        }

        map
    };

    use Subcommands::*;

    match subcommand {
        Add { version } => {
            let hash = repo.normalise_hash(&version)?;

            repo.trash.add(hash);

            println!("Moved snapshot {hash} to the trash!");

            let others_removed = count_subnodes(&inverse_links, hash);

            if others_removed > 0 {
                println!("(Moved {others_removed} other snapshots to the trash too)");
            }
        }

        Recover { version } => {
            let hash = repo.normalise_hash(&version)?;

            match hash_in_trash(hash, &inverse_links, &repo.trash) {
                Some(TrashStatus::Direct) => {
                    repo.trash.remove(hash);
                }

                Some(TrashStatus::Indirect(to_remove)) => {
                    repo.trash.remove(to_remove);
                }

                None => {
                    bail!("snapshot {hash} does not exist in the trash.")
                }
            }

            println!("Recovered {hash} from the trash!");

            let others_recovered = count_subnodes(&inverse_links, hash);

            if others_recovered > 0 {
                println!("(Recovered {others_recovered} other snapshots from the trash too)");
            }
        }

        List { version: None, limit } => {
            let limit = limit.unwrap_or(usize::MAX);

            if repo.trash.is_empty() {
                println!("The trash is empty. Add new snapshots to the trash with `asc trash add`.");

                return Ok(());
            }

            println!("Trash:");

            let capped_entries = repo
                .trash
                .entries
                .chunks(limit)
                .next()
                .unwrap();

            for Entry { hash, when } in capped_entries {
                let mut s = format!(" * {hash}");
                
                let count = count_subnodes(&repo.history.links, *hash);

                if count > 0 {
                    s = format!("{s} [{when}] (+ {count})");
                }

                println!("{s}");
            }

            let remaining = repo.trash.entries.len() - capped_entries.len();

            if remaining > 0 {
                println!("(+ {remaining} more entries)");
            }
        }

        List { version: Some(raw_hash), limit } => {
            let limit = limit.unwrap_or(usize::MAX);

            if repo.trash.is_empty() {
                println!("The trash is empty. Add new snapshots to the trash with `asc trash add`.");

                return Ok(());
            }

            let hash = repo.normalise_hash(&raw_hash)?;

            repo
                .trash
                .entries
                .iter()
                .filter(|e| e.hash == hash)
                .next()
                .ok_or(eyre!("snapshot {hash} does not exist in the trash."))?;

            println!("Trash - implicitly trashed nodes of {hash}:");

            let subnodes = list_all_subnodes(&repo.history.links, hash);
            
            let capped_subnodes = subnodes
                .chunks(limit)
                .next()
                .unwrap();

            for subnode in capped_subnodes {
                println!(" * {subnode}");
            }

            let remaining = subnodes.len() - capped_subnodes.len();

            if remaining > 0 {
                println!("(+ {remaining} remaining subnodes)");
            }
        }
    }

    Ok(())
}