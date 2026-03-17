use crate::{hash::ObjectHash, hash::RawObjectHash, unwrap};

use std::{fs::{self, File}, io::Write, path::{Path, PathBuf}, process::Command};

use eyre::{Context, Result, bail, eyre};
use glob::glob;
use glob_match::glob_match;
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use relative_path::{PathExt, RelativePath, RelativePathBuf};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

/// Expand a path with wildcards into all possible matches by querying the filesystem.
/// 
/// This wraps the [`glob::glob`] function to make it more ergonomic.
pub fn resolve_wildcard_path(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    
    let start = unwrap!(root.to_str(), "invalid utf8 in path: {}", root.display());

    let paths = unwrap!(glob(start), "failed to glob path: {}", root.display());

    let mut result = Vec::new();

    for path in paths {
        let path = path?;

        if !path.is_dir() {
            result.push(path);
            
            continue;
        }

        let pattern = path
            .join("**/*")
            .display()
            .to_string();

        let contents = glob(&pattern)?;

        for path in contents {
            result.push(path?);
        }
    }

    Ok(result)
}

/// Filter a list of strings using a list of glob patterns.
pub fn filter_with_glob<G, I>(
    globs: Vec<G>,
    input: &[I]
) -> Vec<&I>
    where G: AsRef<str>,
          I: AsRef<str>
{
    let mut valid = vec![];

    for path in input {
        for pat in &globs {
            if glob_match(pat.as_ref(), path.as_ref()) {
                valid.push(path);
            }
        }
    }

    valid
}

/// Filter a list of strings using a list of glob patterns.
pub fn filter_with_glob_indexes<G, I>(
    globs: Vec<G>,
    input: &[I]
) -> Vec<(usize, &I)>
    where G: AsRef<str>,
          I: AsRef<str>
{
    let mut valid = vec![];

    for path in input.iter().enumerate() {
        for pat in &globs {
            if glob_match(pat.as_ref(), path.1.as_ref()) {
                valid.push(path);
            }
        }
    }

    valid
}

/// Normalise a relative path contextually by appending it
/// to some root directory and then building a relative path from it.
/// 
/// This is to support functionality like `ls ../testing` where
/// `/tmp/testing/../testing` would be normalised to `/tmp/testing`,
/// but because [`RelativePath::normalize`] works independently of any
/// root path, `../testing` would become `""`
pub fn normalise_with_root(
    path: impl AsRef<RelativePath>,
    root: impl AsRef<Path>
) -> RelativePathBuf
{
    path
        .as_ref()
        .to_logical_path(&root)
        .relative_to(&root)
        .unwrap_or_else(|e| panic!(
            "failed to normalise {:?} with reference to {:?} (error: {e})",
            path.as_ref(),
            root.as_ref()
        ))
}

fn match_path_by_glob(
    glob: impl AsRef<RelativePath>,
    path: impl AsRef<RelativePath>,
    root: impl AsRef<Path>
) -> Option<bool>
{
    let mut current = path.as_ref();
    
    let glob = normalise_with_root(glob, root);

    if glob.starts_with("..") {
        return None;
    }
    
    loop {
        if glob == "" || glob_match(glob.as_str(), current.as_str()) {
            return Some(true);
        }

        match current.parent() {
            Some(next) => current = next,
            None => break
        }
    }

    Some(false)
}

/// Filter a list of paths with a glob pattern.
/// 
/// This will match globs starting with `..`. Use
/// [`filter_paths_with_glob_strict`] to disallow this.
pub fn filter_paths_with_glob<'a, P: AsRef<RelativePath>>(
    globs: &[impl AsRef<RelativePath>],
    paths: &'a [P],
    root: impl AsRef<Path>
) -> Vec<&'a P>
{
    paths
        .iter()
        .filter(|p| globs.iter().any(|glob| {
            match_path_by_glob(glob, *p, &root) == Some(true)
        }))
        .collect()
}

/// Filter a list of paths with a glob pattern.
/// 
/// This will disallow any globs starting with `..` that would
/// search outside of the tree.
pub fn filter_paths_with_glob_strict<'glob, 'path, G, P>(
    globs: &'glob [G],
    paths: &'path [P],
    root: impl AsRef<Path>
) -> Result<Vec<&'path P>, &'glob G>
    where G: AsRef<RelativePath>,
          P: AsRef<RelativePath>
{
    let mut matches = vec![];

    for path in paths {
        for glob in globs {
            match match_path_by_glob(glob, path, &root) {
                Some(true) => matches.push(path),
                Some(false) => {},
                
                None => return Err(glob)
            }
        }
    }

    Ok(matches)
}

/// Filter a list of paths with a glob pattern and include
/// their indexes.
/// 
/// This will match globs starting with `..`. Use
/// [`filter_paths_with_glob_strict`] to disallow this.
pub fn filter_paths_with_glob_indexes<'a, P: AsRef<RelativePath>>(
    globs: &[impl AsRef<RelativePath>],
    paths: &'a [P],
    root: impl AsRef<Path>
) -> Vec<(usize, &'a P)>
{
    paths
        .iter()
        .enumerate()
        .filter(|(_, p)| globs.iter().any(|glob| {
            match_path_by_glob(glob, *p, &root) == Some(true)
        }))
        .collect()
}

/// Filter a list of paths with a glob pattern.
/// 
/// This will disallow any globs starting with `..` that would
/// search outside of the tree.
pub fn filter_paths_with_glob_indexes_strict<'glob, 'path, G, P>(
    globs: &'glob [G],
    paths: &'path [P],
    root: impl AsRef<Path>
) -> Result<Vec<(usize, &'path P)>, &'glob G>
    where G: AsRef<RelativePath>,
          P: AsRef<RelativePath>
{
    let mut matches = vec![];

    for (index, path) in paths.iter().enumerate() {
        for glob in globs {
            println!("{:?} -> {:?}", path.as_ref(), match_path_by_glob(glob, path, &root));

            match match_path_by_glob(glob, path, &root) {
                Some(true) => matches.push((index, path)),
                Some(false) => {},
                
                None => return Err(glob)
            }
        }
    }

    Ok(matches)
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
    let mut path = path.as_ref();
    let root = root.as_ref();

    fs::remove_file(path)?;

    loop {
        path = path.parent().unwrap();

        if path == root || path == "" {
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

/// Write data to a file, compressing it with messagepack.
pub fn save_as_msgpack<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    let mut fp = create_file(path)?;

    let bytes = unwrap!(
        rmp_serde::to_vec(data),
        "failed to build msgpack bytes from type {} (loaded from {})",
        std::any::type_name::<T>(),
        path.display()
    );

    fp.write_all(&bytes)?;
    
    Ok(())
}

/// Load data from a file that was compressed with messagepack.
pub fn load_as_msgpack<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();

    let fp = open_file(path)?;

    let data = unwrap!(
        rmp_serde::from_read(fp),
        "failed to parse data from {} into {}",
        path.display(),
        std::any::type_name::<T>()
    );
    
    Ok(data)
}

pub trait IsGlob {
    fn is_glob(&self) -> bool;
}

impl<T: AsRef<RelativePath>> IsGlob for T {
    fn is_glob(&self) -> bool {
        self.as_ref().as_str().contains(['*', '?', '['])
    }
}
