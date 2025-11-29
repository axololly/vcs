// use asc::commands::main;

use threeway_merge::{merge_strings, MergeOptions};

fn _main() -> eyre::Result<()> {
    let options = MergeOptions {
        base_label: Some("base".to_string()),
        ours_label: Some("ours".to_string()),
        theirs_label: Some("theirs".to_string()),

        .. MergeOptions::default()
    };

    let result = merge_strings("hello world\n", "greetings\n\nsandwich\n\n\nhello\n", "hello world\n", &options)?;

    if result.is_clean_merge() {
        println!("Merged!\n\n{}", result.content);
    }
    else {
        println!("Failed! Conflicts: {}\n\n{}", result.conflicts, result.content);
    }

    Ok(())
}

fn main() {
    let x = [1u8, 2, 3];

    println!("{:?}", x.chunks(usize::MAX).next().unwrap());
}