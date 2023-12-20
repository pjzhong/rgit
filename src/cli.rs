use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Init a rgit repository
    Init,
    #[command(name = "hash-object")]
    /// hash the file
    HashObject { file: String },
    /// cat the file
    #[command(name = "cat-file")]
    CatFile { oid: String },
    /// storing a whole directory
    #[command(name = "write-tree")]
    WriteTree { dir: String },
    /// read a whole directory
    #[command(name = "read-tree")]
    ReadTree { oid: String },
    /// Record changes to the repository
    #[command(name = "commit")]
    Commit {
        #[arg(short, long)]
        message: String,
    },
    /// print the commit history
    #[command(name = "log")]
    Log { oid: Option<String> },
    /// Switch branches or restore working tree files
    #[command(name = "checkout")]
    CheckOut { commit: String },
    /// Create, list, delete or verify a tag object signed with GPG
    #[command(name = "tag")]
    Tag { name: String, oid: Option<String> },
    /// Print refs
    #[command(name = "k")]
    K,
    /// Create new branch
    #[command(name = "branch")]
    Branch {
        name: Option<String>,
        oid: Option<String>,
    },
    /// Show the working tree status
    #[command(name = "status")]
    Status,
    /// Reset current HEAD to the specified state
    #[command(name = "reset")]
    Reset { oid: String },
    /// Show various types of objects
    #[command(name = "show")]
    Show { oid: Option<String> },
    /// Show the changed files
    #[command(name = "diff")]
    Diff { oid: String },
    /// Join two or more development histories together
    #[command(name = "merge")]
    Merge { commit: String },
    /// Find as good common ancestors as possible for a merge
    #[command(name = "merge-base")]
    MergeBase { commit1: String, commit2: String },
    #[command(name = "fetch")]
    /// Download objects and refs from another repository
    Fetch { remote: String },
}
