use std::{fs::File, io::Read};

use clap::Parser;
use rgit::{
    cli::{Cli, Commands},
    data,
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => data::init(),
        Commands::HashObject { file } => match File::open(&file) {
            Ok(mut f) => {
                let mut buffers = Vec::new();
                match f.read_to_end(&mut buffers) {
                    Ok(_) => data::hash(&buffers),
                    Err(e) => eprintln!("read {} file err:{}", file, e),
                }
            }
            Err(e) => eprintln!("open {} file err:{}", file, e),
        },
    }
}
