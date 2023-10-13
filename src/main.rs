use clap::Parser;
use rgit::{
    cli::{Cli, Commands},
    data,
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init =>  data::init(),
    }
}
