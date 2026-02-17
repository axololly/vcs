use std::sync::Arc;

use eyre::Result;
use libasc::{action::Action, repository::Repository, sync::{client::Client, pull::{BranchPullResult, PullResult, TagPullResult}}};
use tokio::sync::Mutex;

#[derive(clap::Args)]
pub struct Args {
    /// The remote to push to. Defaults to all.
    remote: Option<String>,

    // The branch to push. TODO
    // branch: Option<String>
}

#[tokio::main]
pub async fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let remotes = repo.remotes.clone();
    
    let repo_arc = Arc::new(Mutex::new(repo));

    for (name, remote) in remotes.into_iter() {
        if let Some(remote_arg) = &args.remote && name != *remote_arg {
            continue;
        }

        println!("Pulling from: {name}");

        let mut client = Client::connect(remote).await?;

        let results = client.make_pull(repo_arc.clone()).await?;

        println!("Sent: {} | Received: {}", client.bytes_sent(), client.bytes_recv());

        println!();

        println!("Results: ");

        for result in results {
            let line = match result {
                PullResult::Branch(name, result) => match result {
                    BranchPullResult::NotOnRemote => format!(" * {name:?} not found on remote"),
                    
                    BranchPullResult::UpToDate => format!(" * Branch {name:?} is up-to-date"),
                    
                    BranchPullResult::FastForward(new_history, new_tip) => {
                        let mut repo = repo_arc.lock().await;

                        repo.history.extend(&new_history);

                        let old_tip = repo.branches.create(name.clone(), new_tip).unwrap();

                        repo.action_history.push(
                            Action::MoveBranch {
                                name: name.clone(),
                                old: old_tip,
                                new: new_tip
                            }
                        );

                        format!(" * Fast-forwarded {name} ({old_tip} -> {new_tip})")
                    },

                    BranchPullResult::Conflict(new_history, old_tip, new_tip) => {
                        let mut repo = repo_arc.lock().await;

                        repo.history.extend(&new_history);

                        repo.branches.create(name.clone(), new_tip);

                        repo.action_history.push(
                            Action::MoveBranch {
                                name: name.clone(),
                                old: old_tip,
                                new: new_tip
                            }
                        );

                        repo.branches.create(format!("local/{name}"), old_tip);

                        repo.action_history.push(
                            Action::CreateBranch {
                                name: format!("local/{name}"),
                                hash: old_tip
                            }
                        );

                        format!(" ! Branch {name} diverges with remote - local version is renamed to `local/{name}`")
                    }
                },

                PullResult::Tag(name, result) => match result {
                    TagPullResult::New(hash) => format!(" * Tag {name:?} ({hash}) received from remote"),

                    TagPullResult::Conflict(local, remote) => {
                        let mut repo = repo_arc.lock().await;
                        
                        repo.tags.create(name.clone(), remote);

                        repo.action_history.push(
                            Action::MoveTag {
                                name: name.clone(),
                                old: local,
                                new: remote
                            }
                        );

                        repo.tags.create(format!("local/{name}"), local);

                        repo.action_history.push(
                            Action::CreateTag {
                                name: format!("local/{name}"),
                                hash: local
                            }
                        );

                        format!(" ! Tag {name:?} diverges from remote - local version is renamed to `local/{name}`")
                    }
                }
            };

            println!("{line}");
        }

        println!();
    }

    repo_arc.lock().await.save()?;

    Ok(())
}
