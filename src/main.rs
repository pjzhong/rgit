use std::{fs::File, io::Read, path::PathBuf};

use clap::Parser;
use rgit::{
    base,
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
                    Ok(_) => match data::hash(&buffers, data::DataType::Blob) {
                        Ok(hex) => println!("file_hex:{}", hex),
                        Err(err) => eprintln!("hash {} file err:{:?}", file, err),
                    },
                    Err(e) => eprintln!("read {} file err:{}", file, e),
                }
            }
            Err(e) => eprintln!("open {} file err:{}", file, e),
        },
        Commands::CatFile { object } => match data::get_object(&object, data::DataType::None) {
            Ok(str) => println!("{}", str),
            Err(e) => eprintln!("get object:{:?}, err:{:?}", object, e),
        },
        Commands::WriteTree { dir } => {
            base::write_tree(&PathBuf::from(dir)).unwrap();
        }
        Commands::ReadTree { oid } => {
            base::read_tree(&oid);
        }
        Commands::Commit { message } => {
            println!("{:?}", base::commit(&message))
        }
        Commands::Log { oid } => {
            let mut head = if oid.is_some() { oid } else { data::get_head() };
            while let Some(oid) = head.take() {
                if let Some(commit) = base::get_commit(&oid) {
                    println!("commit {}", commit.tree.unwrap_or_else(String::new));
                    println!(
                        "    {}",
                        if let Some(msg) = commit.message.as_ref() {
                            msg
                        } else {
                            ""
                        }
                    );

                    head = commit.parent.clone();
                }
            }
        }
        Commands::CheckOut { oid } => base::checkout(&oid),
    }
}
