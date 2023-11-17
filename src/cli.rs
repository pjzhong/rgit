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
}
