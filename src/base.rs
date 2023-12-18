use std::{
    collections::{HashMap, HashSet, LinkedList},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    data::{self, get_ref, get_ref_recursive, update_ref, DataType, DateErr, RefValue},
    diff,
};

pub struct Commit {
    pub tree: Option<String>,
    pub parents: Vec<String>,
    pub message: Option<String>,
}

pub fn init() {
    data::init();
    data::update_ref(data::HEAD, RefValue::symbolic("refs/heads/master"), true)
}

pub fn get_oid<T: AsRef<str>>(name: T) -> String {
    let name = name.as_ref();

    //简单粗暴，直接遍历
    let refs_to_try: [&str; 4] = [
        &name,
        &format!("refs/{name}"),
        &format!("refs/tags/{name}"),
        &format!("refs/heads/{name}"),
    ];

    for name in refs_to_try {
        if let Some(val) = get_ref(name, false)
            .filter(|ref_val| ref_val.symbolic)
            .filter(|ref_val| !ref_val.value.is_empty())
        {
            return val.value;
        }
    }

    name.to_string()
}

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
                Err(e) => eprintln!("write_tree_hash_object error, file:{:?} err:{:?}", path, e),
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

pub fn get_tree_in_base(oid: &str) -> Option<HashMap<PathBuf, String>> {
    let path = PathBuf::from(".");
    get_tree(oid, &path)
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

                match data::get_object(&oid, DataType::None) {
                    Ok(content) => match File::options()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&path)
                    {
                        Ok(mut f) => {
                            if let Err(e) = f.write_all(content.as_bytes()) {
                                eprintln!("read_tree write err file:{:?}, oid:{:?}", path, e);
                            }
                        }
                        Err(e) => eprintln!("read_tree open file error:{:?}", e),
                    },
                    Err(e) => eprintln!("read_tree err file:{:?}, oid:{:?}", path, e),
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
    if let Some(head) = get_ref_recursive(data::HEAD).filter(|head| !head.value.is_empty()) {
        commit.push_str(&format!("parent {}\n", head.value));
    } else {
        commit.push('\n');
    }
    commit.push_str(&format!("\n{message}\n"));

    match data::hash(commit.as_bytes(), DataType::Commit) {
        Ok(oid) => {
            update_ref(data::HEAD, RefValue::direct(oid.clone()), true);
            Ok(oid)
        }
        err @ Err(_) => err,
    }
}

pub fn get_commit<T: AsRef<str>>(oid: T) -> Option<Commit> {
    let oid = oid.as_ref();
    match data::get_object(oid, DataType::Commit) {
        Ok(content) => {
            const TREE_PREFIX: &str = "tree ";
            const PARENT_PREFIX: &str = "parent ";
            //前两行
            //tree
            //parent
            //空格
            //剩下的都是内容
            let mut lines = content.lines();
            let tree = lines
                .next()
                .and_then(|s| s.strip_prefix(TREE_PREFIX))
                .map(str::to_string);
            let mut parents = vec![];
            while let Some(s) = lines.next().and_then(|s| s.strip_prefix(PARENT_PREFIX)) {
                parents.push(s.to_string());
            }
            let message = lines.collect::<String>();

            Some(Commit {
                tree,
                parents,
                message: Some(message),
            })
        }
        Err(e) => {
            eprintln!("get_commit err, oid:{:?}, err:{:?}", oid, e);
            None
        }
    }
}

pub fn checkout<T: AsRef<str>>(name: T) {
    let name = name.as_ref();
    let oid = get_oid(name);
    match get_commit(&oid) {
        Some(Commit {
            tree: Some(tree_id),
            ..
        }) => {
            read_tree(&tree_id);

            let ref_value = if is_branch(name) {
                RefValue {
                    symbolic: true,
                    value: format!("refs/heads/{name}"),
                }
            } else {
                RefValue::direct(oid)
            };

            data::update_ref(data::HEAD, ref_value, false)
        }
        _ => eprintln!("checkout not exists commit, oid:{}", name),
    }
}

pub fn create_tag(oid: &str, tag: &str) {
    update_ref(
        format!("refs/tags{tag}"),
        RefValue::direct(oid.to_string()),
        true,
    )
}

fn is_branch(branch: &str) -> bool {
    get_ref(&format!("refs/heads/{branch}"), true).is_some()
}

pub fn iter_commits_and_parents(oids: Vec<String>) -> Vec<String> {
    let mut oids = oids.into_iter().collect::<LinkedList<_>>();
    let mut visited = HashSet::new();

    let mut commits = vec![];
    while let Some(oid) = oids.pop_front() {
        if visited.contains(&oid) {
            continue;
        }

        if let Some(parents) = get_commit(&oid).map(|c| c.parents) {
            for parent in parents {
                oids.push_back(parent);
            }
        }

        commits.push(oid.clone());
        visited.insert(oid);
    }

    commits
}

pub fn create_branch<T: AsRef<str>>(name: T, oid: T) {
    data::update_ref(
        format!("refs/heads/{}", name.as_ref()),
        RefValue {
            symbolic: false,
            value: oid.as_ref().to_string(),
        },
        true,
    );
}

pub fn get_branch_name() -> Option<String> {
    let ref_value = match data::get_ref(data::HEAD, false) {
        Some(ref_value) => ref_value,
        None => return None,
    };

    if !ref_value.symbolic {
        return None;
    }

    ref_value
        .value
        .strip_prefix("refs/heads/")
        .map(str::to_string)
}

pub fn reset(oid: String) {
    data::update_ref(
        data::HEAD,
        RefValue {
            symbolic: false,
            value: oid,
        },
        true,
    );
}

pub fn get_working_tree() -> HashMap<PathBuf, String> {
    let read_dir = match PathBuf::from(".").read_dir() {
        Ok(read_dir) => read_dir,
        Err(_) => return HashMap::new(),
    };

    let mut dirs = LinkedList::new();
    dirs.push_back(read_dir);
    let mut entires = HashMap::new();
    while let Some(read_dir) = dirs.pop_front() {
        for path in read_dir.filter_map(Result::ok) {
            let path = path.path();
            if is_ignored(&path) {
                continue;
            }

            if path.is_file() {
                match data::hash_object(&path) {
                    Ok(hex) => {
                        entires.insert(path, hex);
                    }
                    Err(e) => {
                        eprintln!("write_tree_hash_object error, file:{:?} err:{:?}", path, e)
                    }
                }
            } else if let Ok(dir) = path.read_dir() {
                dirs.push_back(dir);
            }
        }
    }

    entires
}

fn read_tree_merged(t_head: &str, t_other: &str) -> Result<(), DateErr> {
    //TODO check is there un commit changes?
    let t_head_tree = match get_tree_in_base(t_head) {
        Some(tree) => tree,
        None => return Err(DateErr::TreeNotExists(String::from(t_head))),
    };

    let t_other_tree = match get_tree_in_base(t_other) {
        Some(tree) => tree,
        None => return Err(DateErr::TreeNotExists(String::from(t_other))),
    };

    let merge_tress = match diff::merge_tress(&t_head_tree, &t_other_tree) {
        Ok(merged_tree) => merged_tree,
        Err(err) => return Err(err),
    };

    for (path, content) in merge_tress {
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(DateErr::Io(e));
            }
        }

        match File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(content.as_bytes()) {
                    eprintln!("read_tree_merged write err file:{:?}, oid:{:?}", path, e);
                }
            }
            Err(e) => eprintln!("read_tree_merged open file error:{:?}", e),
        }
    }

    Ok(())
}

pub fn merge(commit: &str) {
    let c_head = match data::get_ref_recursive(data::HEAD)
        .filter(|refvalue| !refvalue.value.is_empty())
        .and_then(|refvalue| get_commit(refvalue.value))
        .and_then(|commit| commit.tree)
    {
        Some(c_head) => c_head,
        None => {
            eprintln!("merge failed, commit not exists:{:?}", data::HEAD);
            return;
        }
    };

    let c_other = match get_commit(commit).and_then(|commit| commit.tree) {
        Some(c_head) => c_head,
        None => {
            eprintln!("merge failed, commit not exists:{:?}", commit);
            return;
        }
    };

    if let Err(err) = read_tree_merged(&c_head, &c_other) {
        eprintln!("merge failed err:{:?}", err);
    } else {
        println!("Merged in working tree");
    }
}
