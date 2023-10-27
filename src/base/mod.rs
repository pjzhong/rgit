use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::data::{self, get_head, set_head, DataType, DateErr};

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
    //TODO ignore
    for component in path.iter() {
        if component == ".rgit" || component == ".git" || component == "target" {
            return true;
        }
    }

    false
}

/// 递归式读取整个仓库
pub fn get_tree(oid: &str, base_path: &Path) -> Option<HashMap<PathBuf, String>> {
    let root = match data::get_object(oid, DataType::Tree) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("get_tree err, path:{:?}, err:{:?}", base_path, e);
            return None;
        }
    };

    let mut res = HashMap::new();
    for line in root.lines() {
        let parts = line.splitn(3, ' ').collect::<Vec<_>>();
        let (ty, oid, name) = (parts[0], parts[1], parts[2]);
        let path = base_path.join(name);
        match ty {
            "Blob" => {
                res.insert(path, oid.to_string());
            }
            "Tree" => {
                if let Some(map) = get_tree(oid, &path) {
                    res.extend(map);
                }
            }
            _ => eprintln!("Unknow tree entry: {}", ty),
        };
    }

    Some(res)
}

pub fn read_tree(oid: &str) {
    match get_tree(oid, &PathBuf::from("./")) {
        Some(map) => {
            for (path, oid) in map {
                if let Some(parent) = path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        eprintln!("create dirs error:{:?}", e);
                    }
                }

                match File::options().write(true).create(true).open(&path) {
                    Ok(mut f) => match data::get_object(&oid, DataType::None) {
                        Ok(content) => {
                            if let Err(e) = f.write_all(content.as_bytes()) {
                                eprintln!("read_tree write err file:{:?}, oid:{:?}", path, e);
                            }
                        }
                        Err(e) => eprintln!("read_tree err file:{:?}, oid:{:?}", path, e),
                    },
                    Err(e) => eprintln!("open file error:{:?}", e),
                }
            }
        }
        None => eprintln!("tree oid:{}, didn't exit", oid),
    };
}

pub fn commit(message: &str) -> Result<String, DateErr> {
    let oid = match write_tree(&PathBuf::from("./")) {
        Some(oid) => oid,
        None => {
            return Err(DateErr::Err(
                "unknow reason, can't not write current director".to_string(),
            ))
        }
    };

    let mut commit = format!("tree {oid}\n");
    if let Some(head) = get_head() {
        commit.push_str(&format!("parnt {head}\n"));
    }
    commit.push_str(&format!("\n{message}\n"));

    match data::hash(commit.as_bytes(), DataType::Commit) {
        Ok(oid) => {
            set_head(&oid);
            return Ok(oid);
        }
        err @ Err(_) => err,
    }
}
