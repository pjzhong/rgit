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
        Commands::CatFile { oid } => {
            match data::get_object(&base::get_oid(&oid), data::DataType::None) {
                Ok(str) => println!("{}", str),
                Err(e) => eprintln!("get object:{:?}, err:{:?}", oid, e),
            }
        }
        Commands::WriteTree { dir } => {
            base::write_tree(&PathBuf::from(dir)).unwrap();
        }
        Commands::ReadTree { oid } => {
            base::read_tree(&base::get_oid(oid));
        }
        Commands::Commit { message } => {
            println!("{:?}", base::commit(&message))
        }
        Commands::Log { oid } => {
            let mut head = if let Some(oid) = oid {
                Some(base::get_oid(oid))
            } else {
                data::get_ref(data::HEAD)
            };
            while let Some(oid) = head.take() {
                if let Some(commit) = base::get_commit(&oid) {
                    println!("commit {}", oid);
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
        Commands::CheckOut { oid } => base::checkout(&base::get_oid(oid)),
        Commands::Tag { name, oid } => {
            let oid = if let Some(oid) = oid {
                Some(base::get_oid(oid))
            } else {
                data::get_ref(data::HEAD)
            };

            match oid {
                Some(oid) => base::create_tag(&oid, &name),
                None => eprintln!("No head to tag"),
            }
        }
    }
}
