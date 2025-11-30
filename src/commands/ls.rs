use clap::Args as A;
use eyre::Result;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::{backend::repository::Repository, unwrap};

enum Entry<'name> {
    File(&'name str),
    Directory(&'name str, Vec<Entry<'name>>)
}

fn build_file_tree<'a>(paths: &'a [&str], patterns: Option<Vec<String>>) -> Result<Entry<'a>> {
    let mut root = Entry::Directory("", vec![]);

    let ignore = match patterns {
        Some(paths) => {
            let mut builder = GitignoreBuilder::new(".");

            for path in paths {
                builder.add(path);
            }

            builder.build()?
        }
    
        None => Gitignore::empty()
    };

    for path in paths {
        // Ignore any that don't match the glob.
        if !ignore.matched(path, false).is_ignore() {
            continue;
        }

        let node = &mut root;

        for part in path.split('/') {
            match node {
                Entry::File(name) => {
                    *node = Entry::Directory(name, vec![]);
                }

                Entry::Directory(_, children) => {
                    children.push(Entry::File(part));
                }
            }
        }
    }

    Ok(root)
}

#[derive(A)]
pub struct Args {
    /// The pattern to glob against. Omitting this lists from the repository root.
    patterns: Option<Vec<String>>,

    /// Include hidden files.
    #[arg(short = 'a', long = "all")]
    include_hidden: bool,

    /// List contents from another version. Omitting this uses the current version.
    #[arg(short, long)]
    version: Option<String>
}

fn print_tree(tree: Entry<'_>) -> eyre::Result<()> {
    let files = match tree {
        Entry::File(_) => vec![tree],
        Entry::Directory(_, children) => children
    };

    for file in files {
        match file {
            Entry::File(name) => {
                println!("{name}");
            }

            Entry::Directory(name, _) => {
                println!("{name}/");
            }
        }
    }

    Ok(())
}

fn from_version(repo: &Repository, version: &str, patterns: Option<Vec<String>>, include_hidden: bool) -> eyre::Result<()> {
    let snapshot_hash = repo.normalise_version(version)?;

    let snapshot = repo.fetch_snapshot(snapshot_hash)?;

    let mut files: Vec<&str> = vec![];
    
    for raw_path in snapshot.files.keys() {
        let str_path = unwrap!(
            raw_path.to_str(),
            "invalid utf8 in path: {}", raw_path.display()
        );

        if !include_hidden && str_path.starts_with(".") {
            continue;
        }

        files.push(str_path);
    }

    let tree = build_file_tree(&files, patterns)?;

    print_tree(tree)?;

    Ok(())
}

fn from_cwd(repo: &Repository, patterns: Option<Vec<String>>, include_hidden: bool) -> eyre::Result<()> {
    let mut files: Vec<&str> = vec![];

    for raw_path in &repo.staged_files {
        let str_path = unwrap!(
            raw_path.to_str(),
            "invalid utf8 in path: {}", raw_path.display()
        );

        if !include_hidden && str_path.starts_with(".") {
            continue;
        }

        files.push(str_path);
    }

    let tree = build_file_tree(&files, patterns)?;

    print_tree(tree)?;
    
    Ok(())
}

pub fn parse(args: Args) -> eyre::Result<()> {
    let repo = Repository::load()?;

    if let Some(ref version) = args.version {
        from_version(&repo, version, args.patterns, args.include_hidden)
    }
    else {
        from_cwd(&repo, args.patterns, args.include_hidden)
    }
}