#![allow(unused)]

use std::{collections::HashMap, fs::{self, create_dir, remove_dir_all}, io, path::{Path, PathBuf}, pin::pin, process::{Command as StdCommand, Stdio}, sync::Arc};

use async_ssh2_tokio::{AuthMethod, Client as SshClient, ServerCheckMethod};
use async_trait::async_trait;
use chrono::Utc;
use eyre::{Result, eyre};
use libasc::{graph::Graph, key::PrivateKey, repository::Repository, snapshot::Snapshot, sync::{client::Client,clone::handle_clone_as_server, entry::handle_server, pull::{BranchPullResult, PullResult, TagPullResult, handle_pull_as_client, handle_pull_as_server}, push::{BranchPushResult, PushResult, TagPushResult, handle_push_as_client, handle_push_as_server}, stream::{SshStream, StdinStdout, Stream}}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, simplex}, process::{ChildStdin, ChildStdout, Command}, sync::{Mutex, mpsc::channel}};

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

    let mut cmd = StdCommand::new("nu");

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

fn setup_push() -> Result<()> {
    let mut local = init_repo("/tmp/test-local-repo")?;

    ensure_empty("/tmp/test-remote-repo")?;

    let mut cmd = StdCommand::new("nu");

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

fn setup_clone() -> Result<PrivateKey> {
    let mut repo = init_repo("/tmp/test-remote-repo")?;

    let creator = repo.users
        .iter()
        .next()
        .unwrap()
        .private_key
        .clone()
        .unwrap();

    let content1 = repo.save_content(
        "print('hello world!')",
        None
    )?;

    println!("saving content 1 ({content1})");

    let commit1 = Snapshot::new(
        creator.clone(),
        "commit 1".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content1)].into(),
        [repo.current_hash].into()
    );

    println!("saving commit 1 ({})", commit1.hash);

    repo.tags.create("v0.1.0".to_string(), commit1.hash);
    
    repo.append_snapshot(commit1)?;

    let content2 = repo.save_content(
        "print('goodbye world!')",
        Some(content1)
    )?;
    
    println!("saving content 2 ({content2})");

    let commit2 = Snapshot::new(
        creator.clone(),
        "commit 2".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2)].into(),
        [repo.current_hash].into()
    );
    
    println!("saving commit 2 ({})", commit2.hash);

    repo.tags.create("v0.2.0".to_string(), commit2.hash);
    
    repo.append_snapshot(commit2)?;

    let content2_1 = repo.save_content(
        "print('goodbye world 2.1!')",
        Some(content2)
    )?;

    println!("saving content 2.1 ({content2_1})");

    let commit2_1 = Snapshot::new(
        creator.clone(),
        "commit 2.1".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2_1)].into(),
        [repo.current_hash].into()
    );

    println!("saving commit 2.1 ({})", commit2_1.hash);

    repo.tags.create("v0.2.1".to_string(), commit2_1.hash);

    repo.append_snapshot(commit2_1);

    let content2_2 = repo.save_content(
        "print('goodbye world 2.2!')",
        Some(content2)
    )?;

    println!("saving content 2.2 ({content2_2})");

    let commit2_2 = Snapshot::new(
        creator.clone(),
        "commit 2.2".to_string(),
        Utc::now(),
        [(PathBuf::from("main.py"), content2_2)].into(),
        [repo.current_hash].into()
    );

    println!("saving commit 2.2 ({})", commit2_2.hash);

    repo.tags.create("v0.2.2".to_string(), commit2_2.hash);

    repo.append_snapshot(commit2_2);

    repo.save()?;
    
    Ok(creator)
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

async fn make_pull() -> Result<()> {
    let repo = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-local-repo")?
    ));

    let mut client = Client::connect("localhost").await?;

    let results = client.make_pull(repo.clone()).await?;

    let local_repo = repo.lock().await;

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

async fn make_push() -> Result<()> {
    let repo = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-local-repo")?
    ));

    let mut client = Client::connect("localhost").await?;

    let results = client.make_push(repo.clone()).await?;

    let local_repo = repo.lock().await;

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

async fn make_clone() -> Result<()> {
    let login_key = setup_clone()?;

    let path = Path::new("/tmp/test-local-repo");

    ensure_empty(path)?;

    let mut client = Client::connect("localhost").await?;

    client.clone_repo(path, login_key).await
}

async fn act_as_server() -> Result<()> {
    let repo = Arc::new(Mutex::new(
        Repository::load_from("/tmp/test-remote-repo")?
    ));

    let mut stream = StdinStdout::new();

    handle_server(&mut stream, repo).await
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // if let Err(e) = act_as_server().await {
    //     fs::write("/home/axo/dev/rust/vcs/server-output.txt", format!("{e:?}"));
    // }

    // test_pull().await?;

    // test_push().await?;

    // make_clone().await?;

    Ok(())
}

