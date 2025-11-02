// use diff::{lines as diff_lines, Result};

// fn main() {
//     let new = "foo\nbaz";
//     let old = "foo\nbar\nbaz\nquux";

//     for diff in diff_lines(old, new) {
//         match diff {
//             Result::Left(l) => println!("- {l}"),
//             Result::Both(l, _) => println!("  {l}"),
//             Result::Right(r) => println!("+ {r}"),
//         }
//     }
// }

use asc::commands::main;

// fn main() -> eyre::Result<()> {
//     from_cli()
// }