use clap::Subcommand;
use eyre::{Result, bail, eyre};

use crate::backend::{graph::Graph, hash::ObjectHash, repository::Repository, trash::{Entry, TrashStatus}};

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

// type Graph = HashMap<ObjectHash, HashSet<ObjectHash>>;

fn count_subnodes(graph: &Graph, node: ObjectHash) -> usize {
    graph
        .get_parents(node)
        .unwrap()
        .iter()
        .map(|&node| count_subnodes(graph, node))
        .sum()
}

fn list_all_subnodes(graph: &Graph, hash: ObjectHash) -> Vec<ObjectHash> {
    let mut v = vec![hash];
    
    for &child in graph.get_parents(hash).unwrap() {
        v.extend(&list_all_subnodes(graph, child));
    }

    v
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    let inverse_links = {
        let mut graph = Graph::empty();

        for (&hash, parents) in repo.history.iter() {
            for &parent in parents {
                graph.insert(hash, parent);
            }
        }

        graph
    };

    use Subcommands::*;

    match subcommand {
        Add { version } => {
            let hash = repo.normalise_hash(&version)?;

            if hash.is_root() {
                bail!("cannot trash the root snapshot.");
            }

            if let Some(status) = repo.trash_contains(hash) {
                match status {
                    TrashStatus::Direct => {
                        bail!("this snapshot is already in the trash.");
                    }

                    TrashStatus::Indirect(in_trash) => {
                        bail!("this snapshot in the trash because it is a descendant of the snapshot {in_trash}.");
                    }
                }
            }

            let tags_to_remove: Vec<(&str, ObjectHash)> = repo.tags
                .iter()
                .filter_map(|(name, &tag_hash)| {
                    repo.history
                        .is_descendant(tag_hash, hash)
                        .then_some((name.as_str(), tag_hash))
                })
                .collect();

            if !tags_to_remove.is_empty() {
                let mut tag_list = format!("Tagged snapshots ({}):\n", tags_to_remove.len());

                for (name, hash) in &tags_to_remove {
                    tag_list += &format!(" * {name} -> {hash}\n");
                }

                let tag_names: Vec<&str> = tags_to_remove
                    .iter()
                    .map(|(name, _)| *name)
                    .collect();

                bail!("trashing this snapshot and its children involves trashing snapshots that have been tagged.\n\n{tag_list}\nTo resolve this, run `asc tag delete {}` to delete the offending tags.", tag_names.join(" "));
            }

            repo.trash.add(hash);

            if repo.history.is_descendant(repo.current_hash, hash) {
                if repo.has_unsaved_changes()? {
                    bail!("by trashing {hash}, the HEAD at {} would also be trashed.\n\nNormally, this would move the HEAD back to one of the parents of {hash} to move the HEAD out of the trash. However, there are unsaved changes which would be lost.\n\nTo save these, stash them or introduce a new commit to the repository.", repo.current_hash)
                }

                let parents = repo.history.get_parents(hash).unwrap();

                let new_hash = parents
                    .iter()
                    .next()
                    .cloned()
                    .unwrap();

                let new_snapshot = repo.fetch_snapshot(new_hash)?;

                repo.replace_cwd_with_snapshot(&new_snapshot)?;

                repo.current_hash = new_hash;
            }

            println!("Moved snapshot {hash} to the trash!");

            let others_removed = count_subnodes(&inverse_links, hash);

            if others_removed > 0 {
                println!("(Moved {others_removed} other snapshots to the trash too)");
            }
        }

        Recover { version } => {
            let hash = repo.normalise_hash(&version)?;

            match repo.trash_contains(hash) {
                Some(TrashStatus::Direct) => {
                    repo.trash.remove(hash);
                }

                Some(TrashStatus::Indirect(to_remove)) => {
                    bail!("snapshot {hash} cannot be removed from the trash until {to_remove} is removed.");
                }

                None => {
                    bail!("snapshot {hash} does not exist in the trash.");
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
                .entries()
                .chunks(limit)
                .next()
                .unwrap();

            for Entry { hash, when } in capped_entries {
                let mut s = format!(" * {hash}");
                
                let count = count_subnodes(&repo.history, *hash);

                if count > 0 {
                    s = format!("{s} [{when}] (+ {count})");
                }

                println!("{s}");
            }

            let remaining = repo.trash.size() - capped_entries.len();

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
                .entries()
                .iter()
                .filter(|e| e.hash == hash)
                .next()
                .ok_or(eyre!("snapshot {hash} does not exist in the trash."))?;

            println!("Trash - implicitly trashed nodes of {hash}:");

            let subnodes = list_all_subnodes(&repo.history, hash);
            
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

    repo.save()?;

    Ok(())
}