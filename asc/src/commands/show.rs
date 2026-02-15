use eyre::Result;
use libasc::repository::Repository;

#[derive(clap::Args)]
pub struct Args {
    /// The version to display
    version: String
}

pub fn parse(args: Args) -> Result<()> {
    let repo = Repository::load()?;

    let version = repo.normalise_version(&args.version)?;

    let snapshot = repo.fetch_snapshot(version)?;

    let parents: Vec<_> = snapshot.parents.iter().map(|hash| format!("{hash:?}")).collect();

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

        for path in snapshot.files.keys() {
            println!(" * {}", path.display());
        }
    }
    else {
        println!("Files: none");
    }
    
    Ok(())
}
