use std::io::{stdin, stdout, IsTerminal, Write};

use eyre::{bail, Result};
use libasc::{change::FileChange, repository::Repository, utils::resolve_wildcard_path};
use relative_path::{PathExt, RelativePath};

#[derive(clap::Args)]
pub struct Args {
    /// The files to add for the next snapshot. Wilcards will be expanded.
    paths: Vec<String>,
    
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

fn prompt_for_path(path: &RelativePath) -> Result<PromptResult> {
    let stdin = stdin();
    let mut stdout = stdout();

    if !stdin.is_terminal() {
        bail!("failed to prompt about path: stdin is not connected to a tty");
    }

    if !stdout.is_terminal() {
        bail!("failed to prompt about path: stdout is not connected to a tty");
    }

    let mut buf = String::new();

    let result = loop {
        let prompt = format!("The path {path} is marked as ignored. Do you still want to add it? ([y]es, [n]o, [a]ll, [r]eset) ");

        stdout.write_all(prompt.as_bytes())?;

        stdout.flush()?;

        buf.clear();

        let mut input = String::new();
        
        stdin.read_line(&mut input)?;

        let input = input
            .strip_suffix("\n")
            .unwrap_or(input.as_str());

        break match input {
            "y" => PromptResult::Yes,
            "n" => PromptResult::No,
            "a" => PromptResult::All,
            "r" => PromptResult::Reset,

            _ => {
                eprintln!("Invalid input: {input:?}");

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

        repo.staged_files = latest_snapshot.files
            .into_keys()
            .collect();
    }

    let initial_length = repo.staged_files.len();

    let mut resolved_paths = vec![];
    
    for glob in args.paths {
        let full = repo.root_dir.join(glob.as_str());

        let results = resolve_wildcard_path(full)?;

        for result in results {
            resolved_paths.push(result);
        }
    }

    if resolved_paths.is_empty() {
        eprintln!("Nothing to add.");

        return Ok(());
    }

    let mut should_prompt_on_ignored = true;

    for path in resolved_paths {
        let relative = path.relative_to(&repo.root_dir)?;

        let should_prompt = repo.is_ignored_path(&path) && (args.force || !should_prompt_on_ignored);

        if !should_prompt {
            if repo.staged_files.contains(&relative) {
                eprintln!("{}", FileChange::Skip(relative));
            }
            else {
                repo.staged_files.push(relative.clone());

                println!("{}", FileChange::Added(relative));
            }

            continue;
        }
        
        match prompt_for_path(&relative)? {
            PromptResult::Yes => {
                if repo.staged_files.contains(&relative) {
                    eprintln!("Skipping path {relative:?} because it is already tracked...");

                    continue;
                }

                repo.staged_files.push(relative.clone());

                println!("{}", FileChange::Added(relative));
            }
            
            PromptResult::No => {
                eprintln!("{}", FileChange::Skip(relative));
            }
            
            PromptResult::All => {
                repo.staged_files.push(relative);

                should_prompt_on_ignored = false;

                eprintln!("Skipping prompting for all future ignored paths...");
            }

            PromptResult::Reset => {
                eprintln!("Temporary index reset. No files have been added.");

                return Ok(());
            }
        }
    }

    repo.save()?;
    
    let new_files_added = repo.staged_files.len() - initial_length;

    println!("Added {new_files_added} new files.");

    Ok(())
}
