use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use libasc::{content::{Content, Delta}, hash_raw_bytes, repository::Repository, snapshot::Snapshot};
use similar::TextDiff;
use size::{Base, Size};

#[derive(clap::Args)]
pub struct Args {
    /// The version to display
    version: String
}

fn display_snapshot(snapshot: Snapshot, repo: &Repository) {
    let line = format!("Hash: {:?}", snapshot.hash);
    
    if snapshot.hash == repo.current_hash {
        println!("{}", line.bright_green().bold());
    }
    else {
        println!("{line}");
    }

    let parents: Vec<_> = snapshot.parents
        .iter()
        .map(|hash| format!("{hash:?}"))
        .collect();

    println!("Parents: {}", parents.join(", "));

    // .format("%d/%m/%Y %H:%M:%S")

    let branches_here = repo.branches.get_names_for(snapshot.hash);

    if branches_here.len() == 1 {
        println!("Branch: {}", branches_here[0]);
    }
    else if branches_here.len() > 1 {
        println!("Branches: {}", branches_here.join(", "));
    }

    let tags_here = repo.tags.get_names_for(snapshot.hash);

    if !tags_here.is_empty() {
        println!("Tags: {}", tags_here.join(", "));
    }

    let mut tags_on_snapshot: Vec<&str> = repo.tags
        .iter()
        .filter_map(|(name, &hash)| (hash == snapshot.hash).then_some(name.as_str()))
        .collect();

    if !tags_on_snapshot.is_empty() {
        tags_on_snapshot.sort();

        println!("Tags: {}", tags_on_snapshot.join(", "));
    }

    let author = repo.users
        .get_user(&snapshot.author)
        .map(|user| user.name.clone())
        .unwrap_or(format!("unknown ({})", snapshot.author));

    println!("Author: {author}");
    println!("Message: {}", snapshot.message);
    println!("Timestamp: {}", snapshot.timestamp.format("%d/%m/%Y %H:%M:%S"));

    if !snapshot.files.is_empty() {
        println!("Files:");

        for (path, hash) in snapshot.files {
            println!(" * {path} ({hash})");
        }
    }
    else {
        println!("Files: none");
    }
}

fn format_size(n: usize) -> String {
    let size = Size::from_bytes(n);

    size.format()
        .with_base(Base::Base10)
        .to_string()
}

fn display_content(content: Content, repo: &Repository) -> Result<()> {
    let text = content.resolve(repo)?;

    let kind = match &content {
        Content::Literal(data) => {
            format!("Literal, size compressed: {}", format_size(data.len()))
        }
        
        Content::Delta(Delta { original, edit }) => {
            let basis = repo.fetch_string_content(*original)?;

            let diff = TextDiff::from_lines(&basis, &text);

            let similarity = diff.ratio();

            format!(
                "Delta based on {original}, edit size: {}, similarity: {similarity}%",
                format_size(edit.len())
            )
        }
    };

    println!("---");
    println!("Hash: {:?}", hash_raw_bytes(&text));
    println!("{kind}");
    println!("Size: {}", format_size(text.len()));
    println!("---");
    println!("{text}");

    Ok(())
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let version = repo.normalise_version(&args.version)?;

    if repo.history.contains(version) {
        let snapshot = repo.fetch_snapshot(version)?;

        display_snapshot(snapshot, &repo);
    }
    else {
        let content = repo.fetch_content_object(version)?;

        display_content(content, &repo)?;
    }
    
    Ok(())
}
