use std::sync::Arc;

use eyre::Result;
use libasc::{repository::Repository, sync::{client::Client, pull::{BranchPullResult, PullResult, TagPullResult}}};
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
                    
                    BranchPullResult::FastForward(_, old_tip, new_tip) => {
                        format!(" * Fast-forwarded {name} ({old_tip} -> {new_tip})")
                    },

                    BranchPullResult::Conflict(..) => {
                        format!(" ! Branch {name} diverges with remote - local version is renamed to `local/{name}`")
                    }
                },

                PullResult::Tag(name, result) => match result {
                    TagPullResult::New(hash) => format!(" * Tag {name:?} ({hash}) received from remote"),

                    TagPullResult::Conflict(..) => {
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
