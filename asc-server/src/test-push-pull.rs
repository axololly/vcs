#![allow(unused)]

use std::{collections::HashMap, fs::{create_dir, remove_dir_all}, path::{Path, PathBuf}, pin::pin, process::Command, sync::Arc};

use chrono::Utc;
use eyre::{Result, eyre};
use libasc::{graph::Graph, repository::Repository, snapshot::Snapshot, sync::{pull::{handle_pull_as_client, handle_pull_as_server, BranchPullResult, PullResult, TagPullResult}, push::{handle_push_as_client, handle_push_as_server, BranchPushResult, PushResult, TagPushResult}, stream::{local_duplex, Stream}}};
use tokio::{io::simplex, sync::Mutex};

fn ensure_empty(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if path.exists() {
        remove_dir_all(path)?;
    }

    create_dir(path)?;

    Ok(())
}

fn init_repo(path: impl AsRef<Path>) -> Result<Repository> {
    ensure_empty(&path)?;

    Repository::create_new(
        path,
        "axo".to_string(),
        "blabla".to_string()
    )
}

fn setup_pull() -> Result<()> {
    let mut local = init_repo("/tmp/test-local-repo")?;

    let creator = local.users
        .iter()
        .next()
        .unwrap()
        .private_key
        .clone()
        .unwrap();

    let content1 = local.save_content(
        "print('hello world!')",
        None
    )?;

    println!("saving content 1 ({content1})");

    let commit1 = Snapshot::new(
        creator.clone(),
        "commit 1".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content1)].into(),
        [local.current_hash].into()
    );

    println!("saving commit 1 ({})", commit1.hash);

    local.tags.create("v0.1.0".to_string(), commit1.hash);
    
    local.append_snapshot(commit1)?;

    let content2 = local.save_content(
        "print('goodbye world!')",
        Some(content1)
    )?;
    
    println!("saving content 2 ({content2})");

    let commit2 = Snapshot::new(
        creator.clone(),
        "commit 2".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2)].into(),
        [local.current_hash].into()
    );
    
    println!("saving commit 2 ({})", commit2.hash);

    local.tags.create("v0.2.0".to_string(), commit2.hash);
    
    local.append_snapshot(commit2)?;

    local.save()?;

    ensure_empty("/tmp/test-remote-repo")?;

    let mut cmd = Command::new("nu");

    cmd.args(["-c", "cp -r /tmp/test-local-repo/* /tmp/test-remote-repo"]);

    let mut child = cmd.spawn()?;
    
    child.wait()?;

    let mut remote = Repository::load_from("/tmp/test-remote-repo")?;

    let content2_1 = local.save_content(
        "print('goodbye world 2.1!')",
        Some(content2)
    )?;

    println!("saving content 2.1 ({content2_1})");

    let commit2_1 = Snapshot::new(
        creator.clone(),
        "commit 2.1".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2_1)].into(),
        [local.current_hash].into()
    );

    println!("saving commit 2.1 ({})", commit2_1.hash);

    local.tags.create("v0.2.1".to_string(), commit2_1.hash);

    local.append_snapshot(commit2_1);

    let content2_2 = remote.save_content(
        "print('goodbye world 2.2!')",
        Some(content2)
    )?;

    println!("saving content 2.2 ({content2_2})");

    let commit2_2 = Snapshot::new(
        creator.clone(),
        "commit 2.2".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2_2)].into(),
        [remote.current_hash].into()
    );

    println!("saving commit 2.2 ({})", commit2_2.hash);

    remote.tags.create("v0.2.2".to_string(), commit2_2.hash);

    remote.append_snapshot(commit2_2);

    local.save()?;
    remote.save()?;
    
    Ok(())
}

async fn make_pull() -> Result<()> {
    let local = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-local-repo")?
    ));
    let remote = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-remote-repo")?
    ));
    
    let (client, server) = local_duplex();

    let client: &'static mut _ = Box::leak(Box::new(client));
    let server: &'static mut _ = Box::leak(Box::new(server));

    let results = {
        let mut client_fut = pin!(handle_pull_as_client(client, local.clone()));
        let server_fut = pin!(handle_pull_as_server(server, remote.clone()));
        
        tokio::select! {
            client_res = &mut client_fut => client_res,
            
            server_res = server_fut => {
                server_res?;
                
                client_fut.await
            }
        }?
    };

    let local_repo = local.lock().await;

    local_repo.save()?;

    for result in results {
        let name = match &result {
            PullResult::Branch(name, _) => name.to_string(),
            PullResult::Tag(name, _) => name.to_string() /* format!("tag:{name}") */
        };

        let status = match result {
            PullResult::Branch(name, branch_result) => match branch_result {
                BranchPullResult::NotOnRemote => "branch not on remote".to_string(),
                BranchPullResult::UpToDate => "up-to-date".to_string(),
                BranchPullResult::FastForward(_, tip) => format!("{} -> {tip} (ffw)", local_repo.branches.get(&name).unwrap()),
                BranchPullResult::Conflict(_, local, remote) => format!("{local} vs {remote} (split)"),
            },

            PullResult::Tag(name, tag_result) => match tag_result {
                TagPullResult::Conflict(local, remote) => format!("{local} vs {remote} (tag: {name})"),
                TagPullResult::New(hash) => format!("new tag ({hash})")
            }
        };

        println!("{name}: {status}");
    }

    Ok(())
}

fn setup_push() -> Result<()> {
    let mut local = init_repo("/tmp/test-local-repo")?;

    ensure_empty("/tmp/test-remote-repo")?;

    let mut cmd = Command::new("nu");

    cmd.args(["-c", "cp -r /tmp/test-local-repo/* /tmp/test-remote-repo"]);

    let mut child = cmd.spawn()?;
    
    child.wait()?;

    let creator = local.users
        .iter()
        .next()
        .unwrap()
        .private_key
        .clone()
        .unwrap();

    let content1 = local.save_content(
        "print('hello world!')",
        None
    )?;

    println!("saving content 1 ({content1})");

    let commit1 = Snapshot::new(
        creator.clone(),
        "commit 1".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content1)].into(),
        [local.current_hash].into()
    );

    println!("saving commit 1 ({})", commit1.hash);

    local.tags.create("v0.1.0".to_string(), commit1.hash);
    
    local.append_snapshot(commit1)?;

    let content2 = local.save_content(
        "print('goodbye world!')",
        Some(content1)
    )?;
    
    println!("saving content 2 ({content2})");

    let commit2 = Snapshot::new(
        creator.clone(),
        "commit 2".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2)].into(),
        [local.current_hash].into()
    );
    
    println!("saving commit 2 ({})", commit2.hash);

    local.tags.create("v0.2.0".to_string(), commit2.hash);
    
    local.append_snapshot(commit2)?;

    let content3 = local.save_content(
        "print('goodbye world 2!')",
        Some(content2)
    )?;

    println!("saving content 3 ({content3})");

    let commit3 = Snapshot::new(
        creator.clone(),
        "commit 3".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content3)].into(),
        [local.current_hash].into()
    );

    println!("saving commit 3 ({})", commit3.hash);

    local.tags.create("v0.3.0".to_string(), commit3.hash);

    local.append_snapshot(commit3)?;

    local.save()?;

    Ok(())
}

async fn make_push() -> Result<()> {
    let local = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-local-repo")?
    ));
    
    let remote = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-remote-repo")?
    ));
    
    let (client, server) = local_duplex();

    let client: &'static mut _ = Box::leak(Box::new(client));

    let server: &'static mut _ = Box::leak(Box::new(server));

    let results = {
        let mut client_fut = pin!(handle_push_as_client(client, local.clone()));
        let server_fut = pin!(handle_push_as_server(server, remote.clone()));
        
        tokio::select! {
            client_res = &mut client_fut => client_res,
            
            server_res = server_fut => {
                server_res?;
                
                client_fut.await
            }
        }?
    };

    let local_repo = local.lock().await;

    local_repo.save()?;

    for result in results {
        let name = match &result {
            PushResult::Branch(name, _) => name.to_string(),
            PushResult::Tag(name, _) => name.to_string() /* format!("tag:{name}") */
        };

        let status = match result {
            PushResult::Branch(name, branch_result) => match branch_result {
                BranchPushResult::CreatedOnRemote => "created on remote".to_string(),
                BranchPushResult::UpToDate => "up-to-date".to_string(),
                BranchPushResult::FastForward(_, tip) => format!("{} -> {tip} (ffw)", local_repo.branches.get(&name).unwrap()),
                BranchPushResult::SplitHistory => "split history".to_string(),
            },

            PushResult::Tag(name, tag_result) => match tag_result {
                TagPushResult::Conflict(local, remote) => format!("tag conflict - {local} vs {remote}"),
                TagPushResult::New(hash) => format!("new tag ({hash})")
            }
        };

        println!("{name}: {status}");
    }

    Ok(())
}

fn list_actions_local() -> Result<()> {
    let mut local = Repository::load_from("/tmp/test-local-repo")?;

    let (undoable, redoable) = local.action_history.as_slices();

    println!("--- Actions Locally ---");
    
    for action in std::iter::chain(undoable, redoable) {
        println!(" * {action}");
    }

    Ok(())
}

fn list_actions_remote() -> Result<()> {
    let mut remote = Repository::load_from("/tmp/test-remote-repo")?;

    let (undoable, redoable) = remote.action_history.as_slices();

    println!("--- Actions Remotely ---");
    
    for action in std::iter::chain(undoable, redoable) {
        println!(" * {action}");
    }

    Ok(())
}

async fn test_pull() -> Result<()> {
    setup_pull()?;

    println!();

    println!("--- First Pull ---");

    make_pull().await?;

    println!();

    list_actions_local()?;

    println!();

    println!("--- Second Pull ---");

    make_pull().await?;

    println!();

    list_actions_local()?;

    Ok(())
}

async fn test_push() -> Result<()> {
    setup_push()?;

    println!();

    println!("--- First Push ---");

    make_push().await?;

    println!();

    list_actions_remote()?;

    println!();

    println!("--- Second Push ---");

    make_push().await?;

    println!();

    list_actions_remote()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    println!();

    // test_pull().await

    test_push().await
}
