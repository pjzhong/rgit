use std::path::{Path, PathBuf};

use crate::data::{self, DataType};

pub fn write_tree(path: &PathBuf) -> Option<String> {
    //（类型，OID,名字）
    let mut entires: Vec<(DataType, String, String)> = vec![];

    let read_dir = match path.read_dir() {
        Ok(read_dir) => read_dir,
        Err(_) => return None,
    };
    for path in read_dir.filter_map(Result::ok) {
        let path = path.path();
        if is_ignored(&path) {
            continue;
        }

        if path.is_file() {
            match data::hash_object(&path) {
                Ok(hex) => entires.push((DataType::Blob, hex, file_name(&path))),
                Err(e) => eprintln!("hash_object error, file:{:?} err:{:?}", path, e),
            }
        } else if let Some(hex) = write_tree(&path) {
                entires.push((DataType::Tree, hex, file_name(&path)))
        }
    }

    let mut bytes: Vec<u8> = vec![];
    for (ty, oid, name) in entires {
        for u in format!("{} {} {}\n", String::from(&ty), oid, name).as_bytes() {
            bytes.push(*u);
        }
    }

    match data::hash(&bytes, DataType::Tree) {
        Ok(hex) => Some(hex),
        Err(err) => {
            eprintln!("hash dir error, dir:{:?} err:{:?}", path, err);
            None
        }
    }
}

fn file_name(path: &Path) -> String {
    let file_name = match path.file_name() {
        Some(os_str) => os_str.to_string_lossy().to_string(),
        None => String::new(),
    };
    file_name
}

fn is_ignored(path: &Path) -> bool {
    for component in path.iter() {
        if component == ".rgit" {
            return true;
        }
    }

    false
}
