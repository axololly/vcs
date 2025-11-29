use std::{collections::BTreeSet, fs, path::{Path, PathBuf}};

use clap::Args as A;
use eyre::{Result, bail};
use similar::{udiff::UnifiedDiff, TextDiff};

use crate::{backend::{hash::ObjectHash, repository::Repository}, unwrap};

#[derive(A)]
pub struct Args {
    path: Option<PathBuf>,

    from: Option<String>,
    to: Option<String>
}

fn create_diff(path: &Path, old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);

    let mut udiff = UnifiedDiff::from_text_diff(&diff);

    let path_repr = path.display().to_string();

    udiff.header(&path_repr, &path_repr);

    udiff.to_string()
}

#[derive(Debug, Eq)]
pub enum Locator {
    WithHash(PathBuf, ObjectHash),
    FromCwd(PathBuf)
}

impl PartialEq for Locator {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl PartialOrd for Locator {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Locator {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path().cmp(other.path())
    }
}

impl Locator {
    pub fn get_content(&self, repo: &Repository) -> Result<String> {
        match self {
            Locator::WithHash(_, hash) => repo.fetch_string_content(*hash),

            Locator::FromCwd(path) => Ok(unwrap!(
                fs::read_to_string(path),
                "cannot read from file: {}", path.display()
            ))
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Locator::FromCwd(path) => path,
            Locator::WithHash(path, _) => path
        }
    }
}

pub fn get_locators(repo: &Repository, snapshot_hash: Option<ObjectHash>) -> Result<Vec<Locator>> {
    if let Some(hash) = snapshot_hash {
        Ok(repo.fetch_snapshot(hash)?
            .files
            .into_iter()
            .map(|(path, content_hash)| Locator::WithHash(path, content_hash))
            .collect())
    }
    else {
        Ok(repo.staged_files
            .iter()
            .cloned()
            .map(Locator::FromCwd)
            .collect())
    }
}

fn get_before_and_after(
    repo: &Repository,
    old_files: &[Locator],
    new_files: &[Locator],
    path: &Path
) -> Result<(Option<String>, Option<String>)>
{
    let mut old_content = None;

    for locator in old_files {
        if locator.path() == path {
            old_content = Some(locator.get_content(repo)?);
            
            break;
        }
    }

    let mut new_content = None;

    for locator in new_files {
        if locator.path() == path {
            new_content = Some(locator.get_content(repo)?);
            
            break;
        }
    }

    Ok((old_content, new_content))
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let from = if let Some(version) = args.from {
        Some(repo.normalise_version(&version)?)
    }
    else {
        None
    };

    let to = if let Some(version) = args.to {
        Some(repo.normalise_version(&version)?)
    }
    else {
        None
    };

    if from.is_none() && to.is_some() {
        bail!("the option '--to' cannot be used without '--from'.");
    }

    let old_files = get_locators(&repo, from.or(Some(repo.current_hash)))?;

    let new_files = get_locators(&repo, to)?;

    let old = BTreeSet::from_iter(&old_files);
    let new = BTreeSet::from_iter(&new_files);

    let unique_locators = old.union(&new);

    let mut diffs: Vec<String> = vec![];

    for locator in unique_locators {
        let path = locator.path();

        let diff = match get_before_and_after(&repo, &old_files, &new_files, path)? {
            (None, None) => unreachable!(),

            (None, Some(_)) => format!("ADDED     {}", path.display()),

            (Some(_), None) => {
                if to.is_some() {
                    format!("REMOVED   {}", path.display())
                }
                else {
                    format!("MISSING   {}", path.display())
                }
            }

            (Some(old), Some(new)) => create_diff(path, &old, &new)
        };

        if !diff.is_empty() {
            diffs.push(diff);
        }
    }
    
    if !diffs.is_empty() {
        println!("{}", diffs.join("\n"));
    }
    
    Ok(())
}