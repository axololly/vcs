use derive_more::Display;
use relative_path::RelativePath;

#[derive(Display, Debug)]
pub enum FileChange<P: AsRef<RelativePath>> {
    #[display("ADDED       {_0}")]
    Added(P),

    #[display("REMOVED     {_0}")]
    Removed(P),
    
    #[display("EDITED      {_0}")]
    Edited(P),

    #[display("UNCHANGED   {_0}")]
    Unchanged(P),

    #[display("MISSING     {_0}")]
    Missing(P),

    #[display("SKIP        {_0}")]
    Skip(P)
}
