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
    CatFile { object: String },
}
