use std::{io::{stdin, Read}, path::{Path, PathBuf}};

use clap::Args as A;

use crate::{backend::repository::Repository, utils::resolve_wildcard_path};

#[derive(A)]
pub struct Args {
    /// The files to add for the next commit. Wilcards will be expanded.
    paths: Vec<PathBuf>,
    
    /// Do not prompt when trying to add ignored files.
    #[arg(short, long)]
    force: bool,

    /// Replace the staged files with those in the commit the head is referencing.
    #[arg(long)]
    reset: bool
}

enum PromptResult {
    Yes,
    No,
    All,
    Reset
}

fn prompt_for_path(path: &Path) -> eyre::Result<PromptResult> {
    let mut stdin = stdin().lock();

    let result = loop {
        print!("The path '{}' is marked as ignored. Do you still want to add it? ([y]es, [n]o, [a]ll, [r]eset) ", path.display());

        let mut buf = [0u8];

        stdin.read_exact(&mut buf)?;

        break match buf[0] {
            b'y' => PromptResult::Yes,
            b'n' => PromptResult::No,
            b'a' => PromptResult::All,
            b'r' => PromptResult::Reset,

            b => {
                println!("\nInvalid input: {:?}", b as char);
                continue;
            }
        }
    };

    Ok(result)
}

pub fn parse(args: Args) -> eyre::Result<()> {
    let mut repo = Repository::load()?;
    
    if args.reset {
        let latest_commit = repo.fetch_current_commit()?;

        repo.staged_files = latest_commit.files.into_keys().collect();
    }

    let initial_length = repo.staged_files.len();

    let resolved_paths: Vec<PathBuf> = args.paths
        .iter()
        .flat_map(|p| resolve_wildcard_path(p))
        .collect();

    let mut should_prompt_on_ignored = true;

    for path in resolved_paths {
        println!("{}", path.display());
        
        if !repo.is_ignored_path(&path) || args.force || !should_prompt_on_ignored {
            repo.staged_files.push(path);
            continue;
        }
        
        match prompt_for_path(&path)? {
            PromptResult::Yes => {
                repo.staged_files.push(path);
            }
            
            PromptResult::No => {
                println!("Skipping path...");
            }
            
            PromptResult::All => {
                repo.staged_files.push(path);

                should_prompt_on_ignored = false;

                println!("Skipping prompting for all future ignored paths...");
            }

            PromptResult::Reset => {
                println!("Temporary index reset. No files have been added.");

                return Ok(());
            }
        }
    }

    repo.save()?;
    
    let new_files_added = repo.staged_files.len() - initial_length;

    println!("Added {new_files_added} new file{}!", if new_files_added != 1 { "s" } else { "" });

    Ok(())
}