use std::path::Path;

use derive_more::Display;

#[derive(Display)]
pub enum FileChange<'path> {
    #[display("ADDED       {}", _0.display())]
    Added(&'path Path),

    #[display("REMOVED     {}", _0.display())]
    Removed(&'path Path),
    
    #[display("EDITED      {}", _0.display())]
    Edited(&'path Path),

    #[display("UNCHANGED   {}", _0.display())]
    Unchanged(&'path Path),

    #[display("MISSING     {}", _0.display())]
    Missing(&'path Path)
}