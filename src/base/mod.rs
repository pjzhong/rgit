use std::{path::PathBuf, fs};

pub fn write_tree(directory: PathBuf) {
    let mut dirs = vec![fs::read_dir(directory)];

    while !dirs.is_empty() {
        let dir =  match dirs.pop() {
            Some(Ok(dir)) => dir,
            _ => continue,
        };

        for path in dir.filter_map(Result::ok) {
            let path = path.path();
            if path.is_file() {
                println!("{:?}", path);
            } else {
                dirs.push(fs::read_dir(path));
            }
        }
    }
    
}