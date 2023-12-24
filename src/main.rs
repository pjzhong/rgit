use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{self, PathBuf},
};

use clap::Parser;
use rgit::{
    base::Commit,
    cli::{Cli, Commands},
    data::{self, Ugit},
    diff,
};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => Ugit::default().init_repo(),
        Commands::HashObject { file } => match File::open(&file) {
            Ok(mut f) => {
                let ugit = Ugit::default();
                let mut buffers = Vec::new();
                match f.read_to_end(&mut buffers) {
                    Ok(_) => match ugit.hash(&buffers, data::DataType::Blob) {
                        Ok(hex) => println!("file_hex:{}", hex),
                        Err(err) => eprintln!("hash {} file err:{:?}", file, err),
                    },
                    Err(e) => eprintln!("read {} file err:{}", file, e),
                }
            }
            Err(e) => eprintln!("open {} file err:{}", file, e),
        },
        Commands::CatFile { oid } => {
            let ugit = Ugit::default();
            match ugit.get_object(&ugit.get_oid(&oid), data::DataType::None) {
                Ok(str) => println!("{}", str),
                Err(e) => eprintln!("get object:{:?}, err:{:?}", oid, e),
            }
        }
        Commands::WriteTree { dir: _ } => {
            let ugit = Ugit::default();
            println!("{:?}", ugit.write_tree());
        }
        Commands::ReadTree { oid } => {
            let ugit = Ugit::default();
            ugit.read_tree(&ugit.get_oid(oid), false);
        }
        Commands::Commit { message } => {
            let ugit = Ugit::default();
            println!("{:?}", ugit.commit(&message))
        }
        Commands::Log { oid } => {
            log(oid);
        }
        Commands::CheckOut { commit } => Ugit::default().checkout(commit),
        Commands::Tag { name, oid } => {
            let ugit = Ugit::default();
            let oid = if let Some(oid) = oid {
                Some(ugit.get_oid(oid))
            } else {
                ugit.get_ref_recursive(data::HEAD).map(|val| val.value)
            };

            match oid {
                Some(oid) => ugit.create_tag(&oid, &name),
                None => eprintln!("No head to tag"),
            }
        }
        Commands::K => k(),
        Commands::Branch { name, oid } => branch(name, oid),
        Commands::Status => status(),
        Commands::Reset { oid } => reset(oid),
        Commands::Show { oid } => show(oid),
        Commands::Diff { oid } => diff(&oid),
        Commands::Merge { commit } => merge(commit),
        Commands::MergeBase { commit1, commit2 } => merge_base(commit1, commit2),
        Commands::Fetch { remote } => {
            let mut ugit = Ugit::default();
            ugit.fetch(remote);
        }
        Commands::Push { remote, branch } => {
            let mut ugit = Ugit::default();
            ugit.push(
                &remote,
                &format!(
                    "refs{}heads{}{}",
                    path::MAIN_SEPARATOR_STR,
                    path::MAIN_SEPARATOR_STR,
                    branch
                ),
            );
        }
        Commands::Add { files } => {
            let ugit = Ugit::default();
            ugit.add(&files);
        }
    }
}

fn log(oid: Option<String>) {
    let ugit = Ugit::default();
    let head = if let Some(oid) = oid {
        ugit.get_oid(oid)
    } else {
        match ugit.get_ref_if_not_empty(data::HEAD).map(|head| head.value) {
            Some(head) => head,
            None => {
                eprintln!("No Commits");
                return;
            }
        }
    };

    let mut refs: HashMap<String, Vec<String>> = HashMap::new();
    for ref_name in ugit.iter_refs() {
        if let Some(oid) = ugit.get_ref_if_not_empty(&ref_name) {
            let refs = refs.entry(oid.value).or_default();
            refs.push(ref_name);
        }
    }

    for oid in ugit.iter_commits_and_parents(vec![head]) {
        if let Some(commit) = ugit.get_commit(&oid) {
            match refs.get(&oid) {
                Some(refs) => print_commit(&oid, &commit, refs),
                None => print_commit(&oid, &commit, &[]),
            };
        }
    }
}

///Render graph, you need to know well about graphviz tool first
/// I choose skip now
fn k() {
    let ugit = Ugit::default();
    let mut dot = String::from("digraph commits {\n");

    let mut oids = HashSet::new();
    for reference in ugit.iter_refs() {
        if let Some(ref_val) = ugit.get_ref_recursive(&reference) {
            dot.push_str(&format!("\"{reference}\" [ship=note]\n"));
            dot.push_str(&format!("\"{reference}\" -> \"{}\"\n", ref_val.value));
            if !ref_val.symbolic {
                oids.insert(ref_val.value);
            }
        }
    }

    for oid in ugit.iter_commits_and_parents(Vec::from_iter(oids)) {
        if let Some(commit) = ugit.get_commit(&oid) {
            dot.push_str(&format!(
                "\"{oid}\" [shape=box style=filled label=\"{oid}\"]\n"
            ));
            for parent in commit.parents {
                dot.push_str(&format!("\"{oid}\" -> \"{parent}\"\n"));
            }
        }
    }

    dot.push('}');
    println!("{dot}");
}

fn branch(name: Option<String>, oid: Option<String>) {
    let ugit = Ugit::default();
    if let Some(name) = name {
        let oid = if let Some(oid) = oid {
            ugit.get_oid(oid)
        } else {
            match ugit.get_ref_recursive(data::HEAD) {
                Some(head) => head.value,
                None => {
                    eprintln!("No commit yet");
                    return;
                }
            }
        };

        ugit.create_branch(&name, &oid);
        println!("Branch {name} created at {oid}");
    } else {
        let current = ugit.get_branch_name().unwrap_or_default();
        for name in ugit.iter_branch_names() {
            let prefix = if name == current { "*" } else { " " };
            println!("{prefix} {name}")
        }
    }
}

fn status() {
    let ugit = Ugit::default();
    let oid = ugit.get_oid(data::HEAD);
    let branch = ugit.get_branch_name();
    if let Some(branch) = branch {
        println!("On branch {branch}")
    } else {
        println!("HEAD detached at {oid:10}")
    }

    let tree_id = match ugit
        .get_ref_if_not_empty(&oid)
        .and_then(|r| ugit.get_commit(r.value))
        .and_then(|c| c.tree)
    {
        Some(tree_id) => tree_id,
        None => return,
    };

    if let Some(ref_value) = ugit.get_ref_if_not_empty(data::MERGE_HEAD) {
        println!("Merging with {}", ref_value.value);
    }

    let path = PathBuf::from(".");
    if let Some(tree_map) = ugit.get_tree(&tree_id, &path) {
        let actions = diff::iter_changed_files(&tree_map, &ugit.get_working_tree());
        println!("\nChanges to be committed:");
        for (path, action) in actions {
            println!("{:>12}: {:?}", action, path);
        }
    }
}

fn reset(oid: String) {
    Ugit::default().reset(oid)
}

fn show(oid: Option<String>) {
    let ugit = Ugit::default();
    let oid = if let Some(oid) = oid {
        ugit.get_oid(oid)
    } else {
        return;
    };

    match ugit.get_commit(&oid) {
        Some(commit) => print_commit(&oid, &commit, &[]),
        None => {
            eprint!("Show command can't not find commit, oid:{}", oid);
        }
    }
}

fn print_commit(oid: &str, commit: &Commit, refs: &[String]) {
    let refs_str = refs.join(",");
    println!("commit {oid} {refs_str}");
    println!(
        "       {}",
        if let Some(msg) = commit.message.as_ref() {
            &msg
        } else {
            ""
        }
    );
}

fn diff(oid: &str) {
    let ugit = Ugit::default();
    let commit = match ugit.get_commit(oid) {
        Some(commit) => commit,
        None => return,
    };

    print_commit(oid, &commit, &[]);

    if let Some(tree) = commit.tree {
        let path = PathBuf::from(".");

        if let (Some(t_from), Some(t_to)) =
            (ugit.get_tree(&tree, &path), Some(ugit.get_working_tree()))
        {
            let output = diff::diff_tree(&t_from, &t_to);
            println!("{output}");
        }
    }
}

fn merge(commit: String) {
    Ugit::default().merge(&commit);
}

fn merge_base(commit1: String, commit2: String) {
    let ugit = Ugit::default();
    println!("merge_base: {:?}", ugit.get_merge_base(&commit1, &commit2));
}
