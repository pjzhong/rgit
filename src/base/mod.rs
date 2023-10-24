use std::{path::PathBuf, fs};

pub fn write_tree(directory: PathBuf) {
    let mut dirs = vec![directory.read_dir()];

    while !dirs.is_empty() {
        let dir =  match dirs.pop() {
            Some(Ok(dir)) => dir,
            _ => continue,
        };

        for path in dir.filter_map(Result::ok) {
            let path = path.path();
            if is_ignored(&path) {
                continue;
            }

            if path.is_file() {
                println!("{:?}", path);
            } else {
                dirs.push(path.read_dir());
            }
        }
    }
}

fn is_ignored(path: &PathBuf) -> bool {
    for component in path.iter() {
        if component == ".ugit" {
            return true;
        }
    }

    return false;
}