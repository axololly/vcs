use std::{io::{stdin, Read}, path::{Path, PathBuf}};

use clap::Args as A;
use eyre::Result;

use libasc::{repository::Repository, resolve_wildcard_path};

#[derive(A)]
pub struct Args {
    /// The files to add for the next snapshot. Wilcards will be expanded.
    paths: Vec<PathBuf>,
    
    /// Do not prompt when trying to add ignored files.
    #[arg(short, long)]
    force: bool,

    /// Replace the staged files with those in the snapshot the head is referencing.
    #[arg(long)]
    reset: bool
}

enum PromptResult {
    Yes,
    No,
    All,
    Reset
}

fn prompt_for_path(path: &Path) -> Result<PromptResult> {
    let mut stdin = stdin().lock();

    let mut buf = String::new();

    let result = loop {
        print!("The path '{}' is marked as ignored. Do you still want to add it? ([y]es, [n]o, [a]ll, [r]eset) ", path.display());

        buf.clear();

        stdin.read_to_string(&mut buf)?;

        break match buf.as_str() {
            "y" => PromptResult::Yes,
            "n" => PromptResult::No,
            "a" => PromptResult::All,
            "r" => PromptResult::Reset,

            _ => {
                println!("\nInvalid input: {:?}", buf);
                continue;
            }
        }
    };

    Ok(result)
}

pub fn parse(args: Args) -> Result<()> {
    let mut repo = Repository::load()?;
    
    if args.reset {
        let latest_snapshot = repo.fetch_current_snapshot()?;

        repo.staged_files = latest_snapshot.files.into_keys().collect();
    }

    let initial_length = repo.staged_files.len();

    let resolved_paths: Vec<PathBuf> = args.paths
        .iter()
        .flat_map(resolve_wildcard_path)
        .flatten()
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