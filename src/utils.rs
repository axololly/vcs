use crate::{backend::hash::ObjectHash, unwrap};

use std::{fmt, fs::{self, File}, path::{Path, PathBuf}, process::Command};

use eyre::{bail, eyre, Context};
use glob::glob;
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use sha1::{Digest, Sha1};

pub fn resolve_wildcard_path(root: impl AsRef<Path>) -> eyre::Result<Vec<PathBuf>> {
    let root = root.as_ref();
    
    let start = unwrap!(root.to_str(), "invalid utf8 in path: {}", root.display());

    let paths = unwrap!(glob(start), "failed to glob path: {}", root.display());

    let mut result = Vec::new();

    for path in paths {
        result.push(path?);
    }

    Ok(result)
}

pub fn compress_data(input: impl AsRef<[u8]>) -> Vec<u8> {
    compress_to_vec(input.as_ref(), 6)
}

pub fn decompress_data(input: impl AsRef<[u8]>) -> eyre::Result<Vec<u8>> {
    let buf = decompress_to_vec(input.as_ref())
        .map_err(|e| eyre!("failed to decompress data: {e}"))?;

    Ok(buf)
}

pub fn hash_raw_bytes(input: impl AsRef<[u8]>) -> ObjectHash {
    let mut hasher = Sha1::new();

    hasher.update(input);

    let raw_hash: [u8; 20] = hasher.finalize().into();

    raw_hash.into()
}

pub fn remove_path(path: impl AsRef<Path>, root: impl AsRef<Path>) -> eyre::Result<()> {
    fs::remove_file(&path)?;

    loop {
        let path = path.as_ref().parent().unwrap();

        if path == root.as_ref() {
            break Ok(());
        }

        // Read directory and see if it has no children.
        // If it's empty, we'll delete it. If not, stop here.
        if fs::read_dir(path)?.next().is_some() {
            break Ok(());
        }

        fs::remove_dir(path)?;
    }
}

pub fn open_file(path: impl AsRef<Path>) -> eyre::Result<File> {
    File::open(&path)
        .wrap_err_with(|| format!(
            "failed to open path {}",
            path.as_ref().display()
        )
    )
}

pub fn create_file(path: impl AsRef<Path>) -> eyre::Result<File> {
    File::create(&path)
        .wrap_err_with(|| format!(
            "failed to open path {}",
            path.as_ref().display()
        )
    )
}

pub fn get_content_from_editor(editor: &str, snapshot_message_path: &Path) -> eyre::Result<String> {
    // TODO: Fill it with a template like Git and Fossil have
    
    File::create(snapshot_message_path)?;

    let mut editor_cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        
        cmd.arg(format!("/c {editor} '{}'", snapshot_message_path.display()));

        cmd
    }
    else if cfg!(unix) {
        let mut cmd = Command::new("bash");
        
        cmd.arg(format!("-c {editor} '{}'", snapshot_message_path.display()));

        cmd
    }
    else {
        bail!("what the fuck are you running bro 😭");
    };

    let mut child = editor_cmd.spawn()?;

    if !child.wait()?.success() {
        bail!("editor process exited with a non-zero exit code.");
    }

    let content = unwrap!(
        fs::read_to_string(snapshot_message_path),
        "cannot read content of: {}", snapshot_message_path.display()
    );

    Ok(content)
}

struct _Display<T>(pub T)
where
    T: fmt::Display;

pub struct DisplaySeq<'a, T>(pub &'a [T])
where
    T: fmt::Display;

impl<T: fmt::Display> fmt::Debug for _Display<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<T: fmt::Display> fmt::Debug for DisplaySeq<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.0.iter().map(_Display)).finish()
    }
}