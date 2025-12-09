use std::path::PathBuf;

use derive_more::Display;

#[derive(Display)]
pub enum FileChange {
    #[display("ADDED       {}", _0.display())]
    Added(PathBuf),

    #[display("REMOVED     {}", _0.display())]
    Removed(PathBuf),
    
    #[display("EDITED      {}", _0.display())]
    Edited(PathBuf),

    #[display("UNCHANGED   {}", _0.display())]
    Unchanged(PathBuf),

    #[display("MISSING     {}", _0.display())]
    Missing(PathBuf)
}