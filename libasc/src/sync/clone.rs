use std::{collections::{HashMap, HashSet, VecDeque}, path::Path};

use eyre::{Result, eyre};
use serde_bytes::ByteBuf;

use crate::{compress_data, content::Content, decompress_data, hash::ObjectHash, key::{PrivateKey, Signature}, repository::Repository, sync::{remote::Remote, stream::Stream, utils::{get_server_secret, Object, Repo, ServerSecret}}};

pub fn fetch_repo_objecs(repo: &Repository) -> Result<HashMap<ObjectHash, Object>> {
    let mut objects = HashMap::new();

    let mut queue = VecDeque::new();
    let mut hashes_seen = HashSet::new();

    for &hash in repo.tags.values() {
        let snapshot = repo.fetch_snapshot(hash)?;
        
        hashes_seen.insert(hash);

        objects.insert(hash, Object::Commit(Box::new(snapshot)));
    }

    queue.extend(repo.branches.values().cloned());

    while let Some(hash) = queue.pop_front() {
        if hashes_seen.contains(&hash) {
            continue;
        }

        hashes_seen.insert(hash);

        if repo.history.contains(hash) {
            let snapshot = repo.fetch_snapshot(hash)?;

            queue.extend(snapshot.parents.iter().cloned());

            objects.insert(hash, Object::Commit(Box::new(snapshot)));
        }
        else {
            let content = repo.fetch_content_object(hash)?;

            if let Content::Delta(delta) = &content {
                queue.push_back(delta.original);
            }

            objects.insert(hash, Object::Content(content));
        }
    }

    Ok(objects)
}

pub async fn handle_clone_as_client(
    stream: &mut impl Stream,
    remote: Remote,
    local_repo_path: &Path,
    mut user_key: PrivateKey
) -> Result<()>
{
    let secret: ServerSecret = stream.receive().await?;

    let signature = user_key.sign(&secret);

    stream.send(&signature).await?;

    let result: Result<(), String> = stream.receive().await?;

    result.map_err(|message| eyre!("server error: {message}"))?;

    let mut repo = Repository::create_new(
        local_repo_path,
        "axo".to_string(),
        "unnamed".to_string()
    )?;

    repo.project_name = stream.receive().await?;
    repo.project_code = stream.receive().await?;

    repo.branches = stream.receive().await?;
    repo.tags = stream.receive().await?;
    
    repo.current_hash = stream.receive().await?;

    repo.users = stream.receive().await?;

    repo.remotes.create("origin".to_string(), remote);

    let compressed: ByteBuf = stream.receive().await?;

    let decompressed = decompress_data(compressed)?;

    let objects: HashMap<ObjectHash, Object> = rmp_serde::from_slice(&decompressed)?;

    for (hash, object) in objects {
        match object {
            Object::Commit(snapshot) => repo.save_snapshot(*snapshot)?,
            Object::Content(content) => repo.save_content_object(content, hash)?
        }
    }

    repo.save()?;

    Ok(())
}

pub async fn handle_clone_as_server(
    stream: &mut impl Stream,
    repo: Repo
) -> Result<()>
{
    let repo = repo.lock().await;

    let secret = get_server_secret();

    stream.send(&secret).await?;

    let signature: Signature = stream.receive().await?;

    if repo.users.get_user(&signature.key()).is_some() {
        let ok: Result<(), ()> = Ok(());

        stream.send(&ok).await?;
    }
    else {
        let error: Result<(), String> = Err("user does not exist".to_string());

        stream.send(&error).await?;

        return Ok(());
    }

    stream.send(&repo.project_name).await?;
    stream.send(&repo.project_code).await?;

    stream.send(&repo.branches).await?;
    stream.send(&repo.tags).await?;

    stream.send(&repo.current_hash).await?;

    stream.send(&repo.users.without_private_keys()).await?;

    let objects = fetch_repo_objecs(&repo)?;

    let serialised = rmp_serde::to_vec(&objects)?;

    let compressed = serde_bytes::ByteBuf::from(compress_data(serialised));

    stream.send(&compressed).await?;
    
    Ok(())
}
