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
                    Ok(_) => data::hash(&buffers, data::DataType::Blob),
                    Err(e) => eprintln!("read {} file err:{}", file, e),
                }
            }
            Err(e) => eprintln!("open {} file err:{}", file, e),
        },
        Commands::CatFile { object } => match data::get_object(&object, data::DataType::None) {
            Ok(str) => println!("{}", str),
            Err(e) => eprintln!("get object:{:?}, err:{:?}", object, e),
        },
    }
}
