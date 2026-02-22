use eyre::{Result, eyre};

use libasc::{action::Action, graph::Graph, hash::ObjectHash, repository::Repository, trash::{Entry, TrashStatus}, unwrap};

#[derive(clap::Subcommand)]
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
    #[command(visible_alias = "ls")]
    List {
        /// The specific version to list implicitly trashed snapshots of.
        version: Option<String>,

        /// The number of entries to list, or all if unspecified.
        limit: Option<usize>
    }
}

fn count_subnodes(graph: &Graph, node: ObjectHash) -> usize {
    graph
        .get_parents(node)
        .unwrap()
        .iter()
        .map(|&node| count_subnodes(graph, node))
        .sum()
}

fn list_all_subnodes(graph: &Graph, hash: ObjectHash) -> Vec<ObjectHash> {
    let mut v = vec![];
    
    for &child in graph.get_parents(hash).unwrap() {
        v.extend(&list_all_subnodes(graph, child));
    }

    v
}

pub fn parse(subcommand: Subcommands) -> Result<()> {
    let mut repo = Repository::load()?;

    let inverse_links = repo.history.invert();

    use Subcommands::*;

    match subcommand {
        Add { version } => {
            let hash = repo.normalise_hash(&version)?;

            let parents = unwrap!(
                repo.history.get_parents(hash),
                "failed to get parents of hash {hash:?}"
            );

            if parents.is_empty() {
                eprintln!("Cannot trash a root snapshot.");

                return Ok(());
            }

            if let Some(status) = repo.trash_contains(hash) {
                match status {
                    TrashStatus::Direct => {
                        eprintln!("The snapshot {hash} is already in the trash (direct).");
                    }

                    TrashStatus::Indirect(parent) => {
                        eprintln!("The snapshot {hash} is already in the trash (indirect: {parent}).")
                    }
                }

                return Ok(());
            }

            let branches_to_remove: Vec<&str> = repo.branches
                .iter()
                .filter_map(|(name, &branch_hash)| {
                    repo.history
                        .is_descendant(branch_hash, hash)
                        .unwrap()
                        .then_some(name.as_str())
                })
                .collect();

            if !branches_to_remove.is_empty() {
                eprintln!("Trashing this snapshot and its children involves trashing snapshots that are branch tips. To resolve this, run `asc tag delete {}` to delete the offending branches.", branches_to_remove.join(" "));

                return Ok(());
            }

            let tags_to_remove: Vec<&str> = repo.tags
                .iter()
                .filter_map(|(name, &tag_hash)| {
                    repo.history
                        .is_descendant(tag_hash, hash)
                        .unwrap()
                        .then_some(name.as_str())
                })
                .collect();

            if !tags_to_remove.is_empty() {
                eprintln!("Trashing this snapshot and its children involves trashing snapshots that have been tagged. To resolve this, run `asc tag delete {}` to delete the offending tags.", tags_to_remove.join(" "));

                return Ok(());
            }

            repo.trash.add(hash);

            // TODO: make it so this makes new branches for all the parent hashes
            let parents_of_hash: Vec<ObjectHash> = repo.history
                .get_parents(hash)
                .unwrap()
                .iter()
                .cloned()
                .collect();

            if repo.history.is_descendant(repo.current_hash, hash)? {
                if repo.has_unsaved_changes()? {
                    let pretty_offending = repo.branches
                        .get_name_for(hash)
                        .map(String::from)
                        .unwrap_or(hash.to_string());

                    let pretty_current = repo
                        .current_branch()
                        .map(String::from)
                        .unwrap_or(format!("{}", repo.current_hash));
                    
                    eprintln!("By trashing {pretty_offending}, the HEAD at {pretty_current} would also be trashed. Normally, this would move the HEAD back to one of the parents of {pretty_offending} to move the HEAD out of the trash. However, there are unsaved changes which would be lost. To save these, stash them or introduce a new commit to the repository.");

                    return Ok(());
                }

                let new_hash = parents_of_hash[0];

                let new_snapshot = repo.fetch_snapshot(new_hash)?;

                println!("Changing snapshots: {} -> {new_hash}", repo.current_hash);

                repo.replace_cwd_with_snapshot(&new_snapshot)?;

                repo.current_hash = new_hash; // TODO: add this to log?
            }

            println!("Moved snapshot {hash} to the trash!");

            let others_removed = count_subnodes(&inverse_links, hash);

            if others_removed > 0 {
                println!("(Moved {others_removed} other snapshots to the trash too)");
            }

            repo.action_history.push(
                Action::TrashAdd { hash }
            );
        }

        Recover { version } => {
            let hash = repo.normalise_hash(&version)?;

            match repo.trash_contains(hash) {
                Some(TrashStatus::Direct) => {
                    repo.trash.remove(hash);
                }

                Some(TrashStatus::Indirect(to_remove)) => {
                    eprintln!("Snapshot {hash} cannot be removed from the trash until {to_remove} is removed.");

                    return Ok(());
                }

                None => {
                    eprintln!("Snapshot {hash} does not exist in the trash.");

                    return Ok(());
                }
            }

            println!("Recovered {hash} from the trash!");

            let others_recovered = count_subnodes(&inverse_links, hash);

            if others_recovered > 0 {
                println!("(Recovered {others_recovered} other snapshots from the trash too)");
            }

            repo.action_history.push(
                Action::TrashRecover { hash }
            );
        }

        List { version: None, limit } => {
            let limit = limit.unwrap_or(usize::MAX);

            if repo.trash.is_empty() {
                eprintln!("The trash is empty. Add new snapshots to the trash with `asc trash add`.");

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
                let mut s = format!(" * {hash} [{when}]");
                
                let count = count_subnodes(&repo.history, *hash);

                if count > 0 {
                    s = format!("{s} (+ {count})");
                }

                println!("{s}");
            }

            let remaining = repo.trash.size() - capped_entries.len();

            if remaining > 0 {
                println!("(+ {remaining} more entries)");
            }
        }

        List { version: Some(raw_version), limit } => {
            let limit = limit.unwrap_or(usize::MAX);

            if repo.trash.is_empty() {
                eprintln!("The trash is empty. Add new snapshots to the trash with `asc trash add`.");

                return Ok(());
            }

            let hash = repo.normalise_hash(&raw_version)?;

            repo
                .trash
                .entries()
                .iter()
                .find(|e| e.hash == hash)
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
