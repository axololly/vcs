// use asc::commands::main;

use similar::TextDiff;

fn main() -> eyre::Result<()> {
    let old = "\
pub struct Snapshot {
    pub hash: ObjectHash,
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Local>,
    
    // A BTreeMap is used to preserve order, so that
    // reconstructing and validating the hash is easier.
    pub files: BTreeMap<PathBuf, ObjectHash>
}";
    let new = "\
pub struct Snapshot {
    pub hash: ObjectHash,
    pub author: String,
    pub message: String,
    pub files: BTreeMap<PathBuf, ObjectHash>
}";

    let diff = TextDiff::from_lines(old, new);

    if diff.ratio() < 0.75 {
        println!("too low: {}% < 75%", diff.ratio() * 100.0);

        return Ok(());
    }

    // let old = "hello world\n";
    // let new = "hello\nthis is a new line\n";

    let xd = xdelta3::encode(new.as_bytes(), old.as_bytes()).unwrap();

    println!("xd size: {}", xd.len());

    let decoded = xdelta3::decode(&xd, old.as_bytes()).unwrap();

    println!("{}", decoded == new.as_bytes());

    Ok(())
}