use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::PathBuf,
};

use clap::Parser;
use rgit::{
    base::{self},
    cli::{Cli, Commands},
    data::{self, iter_branch_names},
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => base::init(),
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
        Commands::CheckOut { commit } => base::checkout(commit),
        Commands::Tag { name, oid } => {
            let oid = if let Some(oid) = oid {
                Some(base::get_oid(oid))
            } else {
                data::get_ref_recursive(data::HEAD).map(|val| val.value)
            };

            match oid {
                Some(oid) => base::create_tag(&oid, &name),
                None => eprintln!("No head to tag"),
            }
        }
        Commands::K => k(),
        Commands::Branch { name, oid } => branch(name, oid),
        Commands::Status => status(),
    }
}

fn log(oid: Option<String>) {
    let head = if let Some(oid) = oid {
        base::get_oid(oid)
    } else {
        data::get_ref_recursive(data::HEAD)
            .map(|head| head.value)
            .unwrap_or_default()
    };

    let mut refs: HashMap<String, HashSet<String>> = HashMap::new();
    for ref_name in data::iter_refs() {
        if let Some(oid) = data::get_ref_recursive(&ref_name) {
            refs.entry(oid.value).or_default().insert(ref_name);
        }
    }

    for oid in base::iter_commits_and_parents(vec![head]) {
        if let Some(commit) = base::get_commit(&oid) {
            let refs_str = if let Some(refs) = refs.get(&oid) {
                let mut str = String::from(" ");
                for ref_name in refs {
                    str.push_str(&format!(",{ref_name}"));
                }

                str
            } else {
                String::new()
            };
            println!("commit {}{}", oid, refs_str);
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
        if let Some(ref_val) = data::get_ref_recursive(&reference) {
            dot.push_str(&format!("\"{reference}\" [ship=note]\n"));
            dot.push_str(&format!("\"{reference}\" -> \"{}\"\n", ref_val.value));
            if !ref_val.symbolic {
                oids.insert(ref_val.value);
            }
        }
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

fn branch(name: Option<String>, oid: Option<String>) {
    if let Some(name) = name {
        let oid = if let Some(oid) = oid {
            base::get_oid(oid)
        } else {
            match data::get_ref_recursive(data::HEAD) {
                Some(head) => head.value,
                None => {
                    eprintln!("No commit yet");
                    return;
                }
            }
        };

        base::create_branch(&name, &oid);
        println!("Branch {name} created at {oid}");
    } else {
        let current = base::get_branch_name().unwrap_or_default();
        for name in iter_branch_names() {
            let prefix = if name == current { "*" } else { " " };
            println!("{prefix} {name}")
        }
    }
}

fn status() {
    let oid = base::get_oid(data::HEAD);
    let branch = base::get_branch_name();
    if let Some(branch) = branch {
        println!("On branch {branch}")
    } else {
        println!("HEAD detached at {oid:10}")
    }
}
