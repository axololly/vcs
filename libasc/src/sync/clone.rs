use std::{collections::{HashMap, HashSet, VecDeque}, path::Path};

use eyre::{Result, eyre};
use serde_bytes::ByteBuf;

use crate::{compress_data, content::Content, decompress_data, hash::ObjectHash, key::{PrivateKey, Signature}, repository::Repository, sync::{stream::Stream, utils::{Object, Repo, ServerSecret, get_server_secret}}};

pub async fn handle_clone_as_client(
    stream: &mut impl Stream,
    repo_path: &Path,
    mut user_key: PrivateKey
) -> Result<()>
{
    let secret: ServerSecret = stream.receive().await?;

    let signature = user_key.sign(&secret);

    stream.send(&signature).await?;

    let result: Result<(), String> = stream.receive().await?;

    result.map_err(|message| eyre!("server error: {message}"))?;

    let mut repo = Repository::create_new(
        repo_path,
        "axo".to_string(),
        "name".to_string()
    )?;

    repo.project_name = stream.receive().await?;
    repo.project_code = stream.receive().await?;

    repo.branches = stream.receive().await?;
    repo.tags = stream.receive().await?;
    
    repo.current_hash = stream.receive().await?;

    repo.users = stream.receive().await?;

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

    let mut objects: HashMap<ObjectHash, Object> = HashMap::new();

    let mut content_seen: HashSet<ObjectHash> = HashSet::new();
    let mut queue: VecDeque<ObjectHash> = VecDeque::new();

    queue.extend(repo.branches.hashes());

    while let Some(next) = queue.pop_front() {
        if repo.history.contains(next) {
            let snapshot = repo.fetch_snapshot(next)?;

            queue.extend(snapshot.files.values().cloned());

            queue.extend(snapshot.parents.iter().cloned());

            objects.insert(next, Object::Commit(Box::new(snapshot)));
        }
        else if !content_seen.contains(&next) {
            content_seen.insert(next);

            let content = repo.fetch_content_object(next)?;

            if let Content::Delta(delta) = &content {
                queue.push_back(delta.original);
            }

            objects.insert(next, Object::Content(content));
        }
    }

    let serialised = rmp_serde::to_vec(&objects)?;

    let compressed = serde_bytes::ByteBuf::from(compress_data(serialised));

    stream.send(&compressed).await?;
    
    Ok(())
}
