use std::path::Path;

use derive_more::Display;

#[derive(Display)]
pub enum FileChange<'path> {
    #[display("ADDED       {}", "0.display()")]
    Added(&'path Path),

    #[display("REMOVED     {}", "0.display()")]
    Removed(&'path Path),
    
    #[display("EDITED      {}", "0.display()")]
    Edited(&'path Path),

    #[display("UNCHANGED   {}", "0.display()")]
    Unchanged(&'path Path),

    #[display("MISSING     {}", "0.display()")]
    Missing(&'path Path)
}