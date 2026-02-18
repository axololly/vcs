use eyre::Result;
use libasc::{content::{Content, Delta}, repository::Repository, snapshot::Snapshot};
use similar::TextDiff;
use size::{Base, Size};

#[derive(clap::Args)]
pub struct Args {
    /// The version to display
    version: String
}

fn display_snapshot(snapshot: Snapshot, repo: &Repository) {
    let parents: Vec<_> = snapshot.parents
        .iter()
        .map(|hash| format!("{hash:?}"))
        .collect();

    let author = repo.users
        .get_user(&snapshot.author)
        .map(|user| user.name.clone())
        .unwrap_or(format!("unknown ({})", snapshot.author));

    if snapshot.hash == repo.current_hash {
        println!("Hash: {:?} (current)", snapshot.hash);
    }
    else {
        println!("Hash: {:?}", snapshot.hash);
    }

    if let Some(name) = repo.current_branch() {
        println!("Branch: {name}");
    }

    let mut tags_on_snapshot: Vec<&str> = repo.tags
        .iter()
        .filter_map(|(name, &hash)| (hash == snapshot.hash).then_some(name.as_str()))
        .collect();

    if !tags_on_snapshot.is_empty() {
        tags_on_snapshot.sort();

        println!("Tags: {}", tags_on_snapshot.join(", "));
    }

    println!("Parents: {}", parents.join(", "));
    println!("Author: {author}");
    println!("Message: {}", snapshot.message);

    if !snapshot.files.is_empty() {
        println!("Files:");

        for (path, hash) in snapshot.files {
            println!(" * {} ({hash})", path.display());
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
