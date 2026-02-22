use derive_more::Display;
use relative_path::RelativePathBuf;

#[derive(Display)]
pub enum FileChange {
    #[display("ADDED       {_0}")]
    Added(RelativePathBuf),

    #[display("REMOVED     {_0}")]
    Removed(RelativePathBuf),
    
    #[display("EDITED      {_0}")]
    Edited(RelativePathBuf),

    #[display("UNCHANGED   {_0}")]
    Unchanged(RelativePathBuf),

    #[display("MISSING     {_0}")]
    Missing(RelativePathBuf),

    #[display("SKIP        {_0}")]
    Skip(RelativePathBuf)
}
