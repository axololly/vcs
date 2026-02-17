use std::sync::Arc;

use eyre::Result;
use libasc::{repository::Repository, sync::{client::Client, push::{BranchPushResult, PushResult, TagPushResult}}};
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

        println!("Pushing to: {name}");

        let mut client = Client::connect(remote).await?;

        let results = client.make_push(repo_arc.clone()).await?;

        println!("Results: ");

        for result in results {
            let line = match result {
                PushResult::Branch(name, result) => match result {
                    BranchPushResult::CreatedOnRemote => format!(" * Branch {name:?} created on remote"),
                    
                    BranchPushResult::UpToDate => format!(" * Branch {name:?} is up-to-date"),
                    
                    BranchPushResult::FastForward(old_tip, new_tip) => {
                        format!(" * Fast-forwarded {name} ({old_tip} -> {new_tip})")
                    },

                    BranchPushResult::SplitHistory => format!(" ! Branch {name:?} diverges from remote - pull to see more")
                },

                PushResult::Tag(name, result) => match result {
                    TagPushResult::CreatedOnRemote => format!(" * Tag {name:?} created on remote"),

                    TagPushResult::Conflict => format!(" ! Tag {name:?} diverges from remote - pull to see more")
                }
            };

            println!("{line}");
        }

        println!();
    }

    repo_arc.lock().await.save()?;

    Ok(())
}
