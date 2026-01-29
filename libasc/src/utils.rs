use crate::{hash::ObjectHash, hash::RawObjectHash, unwrap};

use std::{fs::{self, File}, io::Write, path::{Path, PathBuf}, process::Command};

use eyre::{Context, Result, bail, eyre};
use glob::glob;
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Expand a path with wildcards into all possible matches.
/// 
/// This wraps the [`glob::glob`] function to make it more ergonomic.
pub fn resolve_wildcard_path(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    
    let start = unwrap!(root.to_str(), "invalid utf8 in path: {}", root.display());

    let paths = unwrap!(glob(start), "failed to glob path: {}", root.display());

    let mut result = Vec::new();

    for path in paths {
        result.push(path?);
    }

    Ok(result)
}

/// Compress data using [`miniz_oxide::deflate::compress_to_vec`].
pub fn compress_data(input: impl AsRef<[u8]>) -> Vec<u8> {
    compress_to_vec(input.as_ref(), 6)
}

/// Decompress data using [`miniz_oxide::inflate::decompress_to_vec`].
pub fn decompress_data(input: impl AsRef<[u8]>) -> Result<Vec<u8>> {
    let buf = decompress_to_vec(input.as_ref())
        .map_err(|e| eyre!("failed to decompress data: {e}"))?;

    Ok(buf)
}

/// Compute a SHA-1 hash from the given bytes.
pub fn hash_raw_bytes(input: impl AsRef<[u8]>) -> ObjectHash {
    let mut hasher = Sha256::new();

    hasher.update(input);

    let raw_hash: RawObjectHash = hasher.finalize().into();

    raw_hash.into()
}

/// Remove a path, and also recursively remove any empty directories.
/// 
/// ### Example
/// 
/// In the case of `a/b/c.txt`, calling `remove_path` would remove `c.txt`,
/// but then `a/b/` would be empty, so `b/` gets removed, making `a/` empty,
/// which also gets removed. Once the root is reached (typically `.`), the
/// process stops.
pub fn remove_path(path: impl AsRef<Path>, root: impl AsRef<Path>) -> Result<()> {
    fs::remove_file(&path)?;

    loop {
        let path = path.as_ref().parent().unwrap();

        if path == root.as_ref() || path == "" {
            break Ok(());
        }
        
        let mut dir_contents = unwrap!(
            fs::read_dir(path),
            "failed to read contents of directory: {}",
            path.display()
        );

        // Read directory and see if it has no children.
        // If it's empty, we'll delete it. If not, stop here.
        if dir_contents.next().is_some() {
            break Ok(());
        }

        unwrap!(
            fs::remove_dir(path),
            "failed to remove directory: {}",
            path.display()
        );
    }
}

/// Open a file on disk.
/// 
/// This wraps [`File::open`] to also include the path that was opened
/// in the case of an error.
pub fn open_file(path: impl AsRef<Path>) -> Result<File> {
    File::open(&path)
        .wrap_err_with(|| format!(
            "failed to open path {}",
            path.as_ref().display()
        )
    )
}

/// Open a file on disk.
/// 
/// This wraps [`File::create`] to also include the path that was opened
/// in the case of an error.
pub fn create_file(path: impl AsRef<Path>) -> Result<File> {
    File::create(&path)
        .wrap_err_with(|| format!(
            "failed to create path {}",
            path.as_ref().display()
        )
    )
}

/// Open an interactive editor, wait for the process to end, then return
/// the content of the file after.
/// 
/// To spawn a process on Windows, this uses `cmd`, while on Unix, it uses `bash`.
/// Anything else and you get a special error message :)
pub fn get_content_from_editor(editor: &str, snapshot_message_path: &Path, template_message: &str) -> Result<String> {
    unwrap!(
        fs::write(snapshot_message_path, template_message),
        "failed to write snapshot template {template_message:?} to path {}",
        snapshot_message_path.display()
    );

    let mut editor_cmd = if cfg!(windows) {
        let mut cmd = Command::new("cmd");

        cmd
            .arg("/c")
            .arg(editor)
            .arg(snapshot_message_path.display().to_string());

        cmd
    }
    else if cfg!(unix) {
        let mut cmd = Command::new("bash");
        
        cmd
            .arg("-c")
            .arg(format!("{editor} {}", snapshot_message_path.display()));

        cmd
    }
    else {
        bail!("what the fuck are you running bro 😭");
    };

    let mut child = editor_cmd.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        let message = match status.code() {
            Some(code) => format!("exited with non-zero exit code: {code}"),
            None => "process terminated by signal".to_string()
        };

        bail!("editor process did not exit successfully: {}.", message);
    }

    let content = unwrap!(
        fs::read_to_string(snapshot_message_path),
        "cannot read content of: {}", snapshot_message_path.display()
    );

    let cleaned: String = content
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect();

    Ok(cleaned)
}

pub fn save_as_msgpack<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<()> {
    let mut fp = create_file(path)?;

    let bytes = rmp_serde::to_vec(data)?;

    fp.write_all(&bytes)?;
    
    Ok(())
}

// struct _Display<T>(pub T)
// where
//     T: fmt::Display;

// pub struct DisplaySeq<'a, T>(pub &'a [T])
// where
//     T: fmt::Display;

// impl<T: fmt::Display> fmt::Debug for _Display<T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         fmt::Display::fmt(&self.0, f)
//     }
// }

// impl<T: fmt::Display> fmt::Debug for DisplaySeq<'_, T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_list().entries(self.0.iter().map(_Display)).finish()
//     }
// }
