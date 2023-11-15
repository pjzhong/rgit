use std::{collections::HashSet, fs::File, io::Read, path::PathBuf};

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
            log(oid);
        }
        Commands::CheckOut { oid } => base::checkout(base::get_oid(oid)),
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
        Commands::K => k(),
        Commands::Branch { name, oid } => branch(&name, oid),
    }
}

fn log(oid: Option<String>) {
    let head = if let Some(oid) = oid {
        base::get_oid(oid)
    } else {
        data::get_ref(data::HEAD).unwrap_or_default()
    };

    for oid in base::iter_commits_and_parents(vec![head]) {
        if let Some(commit) = base::get_commit(&oid) {
            println!("commit {}", oid);
            println!("       {}", commit.message.unwrap_or_default());
        }
    }
}

///Render graph, you need to know well about graphviz tool first
/// I choose skip now
fn k() {
    let mut dot = String::from("digraph commits {\n");

    let mut oids = HashSet::new();
    for reference in data::iter_refs() {
        let oid = base::get_oid(&reference);
        dot.push_str(&format!("\"{reference}\" [ship=note]\n"));
        dot.push_str(&format!("\"{reference}\" -> \"{oid}\"\n"));
        oids.insert(oid);
    }

    for oid in base::iter_commits_and_parents(Vec::from_iter(oids)) {
        if let Some(commit) = base::get_commit(&oid) {
            dot.push_str(&format!(
                "\"{oid}\" [shape=box style=filled label=\"{oid}\"]\n"
            ));
            if let Some(parent) = commit.parent {
                dot.push_str(&format!("\"{oid}\" -> \"{parent}\"\n"));
            }
        }
    }

    dot.push('}');
    println!("{dot}");
}

fn branch(name: &str, oid: Option<String>) {
    let oid = if let Some(oid) = oid {
        base::get_oid(oid)
    } else {
        match data::get_ref(data::HEAD) {
            Some(head) => head,
            None => {
                eprintln!("No commit yet");
                return;
            }
        }
    };

    base::create_branch(name, &oid);
    println!("Branch {name} created at {oid}");
}
