use std::{
    collections::{HashMap, HashSet, LinkedList},
    env,
    fs::{self},
    io::Error,
    path::{Path, PathBuf},
};

use crate::data::{self, DataType, DateErr, RefValue, Ugit};

pub struct Commit {
    pub tree: Option<String>,
    pub parents: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum Node {
    Dir(HashMap<String, Node>),
    File(String),
}

impl Ugit {
    pub fn init_repo(&self) {
        self.init();
        self.update_ref(data::HEAD, RefValue::symbolic("refs/heads/master"), true)
    }

    pub fn get_oid<T: AsRef<str>>(&self, name: T) -> String {
        let name = name.as_ref();

        //简单粗暴，直接遍历
        let refs_to_try: [&str; 4] = [
            &name,
            &format!("refs/{name}"),
            &format!("refs/tags/{name}"),
            &format!("refs/heads/{name}"),
        ];

        for name in refs_to_try {
            if let Some(val) = self
                .get_ref(name, false)
                .filter(|ref_val| !ref_val.value.is_empty())
            {
                return val.value;
            }
        }

        name.to_string()
    }

    /// 递归式读取整个仓库
    pub fn get_tree(&self, oid: &str, base_path: &Path) -> Option<HashMap<PathBuf, String>> {
        let root = match self.get_object(oid, DataType::Tree) {
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
                    if let Some(map) = self.get_tree(oid, &path) {
                        res.extend(map);
                    }
                }
                _ => eprintln!("Unknow tree entry: {}", ty),
            };
        }

        Some(res)
    }

    pub fn get_tree_in_base(&self, oid: &str) -> Option<HashMap<PathBuf, String>> {
        let path = PathBuf::from(".");
        self.get_tree(oid, &path)
    }

    pub fn read_tree(&self, oid: &str, update_working: bool) {
        match self.get_tree(oid, &PathBuf::from("./")) {
            Some(map) => {
                match self.get_index() {
                    Ok(mut index) => {
                        index.clear();
                        for (path, oid) in map {
                            index.insert(path.to_string_lossy().to_string(), oid);
                        }

                        if let Err(err) = self.write_index(&index) {
                            eprintln!("read_tree, write_index error:{:?}", err);
                        }

                        if update_working {
                            if let Err(err) = self.checkout_index(&index) {
                                eprintln!("read_tree, checkout_index error:{:?}", err);
                            }
                        }
                    }
                    Err(err) => eprintln!("read_tree get_index error:{:?}", err),
                };
            }
            None => eprintln!("tree oid:{}, didn't exit", oid),
        };
    }

    pub fn commit(&self, message: &str) -> Result<String, DateErr> {
        let oid = self.write_tree()?;

        let mut commit = format!("tree {oid}\n");
        if let Some(head) = self
            .get_ref_recursive(data::HEAD)
            .filter(|head| !head.value.is_empty())
        {
            commit.push_str(&format!("parent {}\n", head.value));
        } else {
            commit.push('\n');
        }

        if let Some(head) = self
            .get_ref_recursive(data::MERGE_HEAD)
            .filter(|head| !head.value.is_empty())
        {
            commit.push_str(&format!("parent {}\n", head.value));
            if let Err(err) = self.delete_ref(data::MERGE_HEAD, true) {
                println!("commit, delete merge head error, err:{:?}", err);
            }
        } else {
            commit.push('\n');
        }
        commit.push_str(&format!("\n{message}\n"));

        match self.hash(commit.as_bytes(), DataType::Commit) {
            Ok(oid) => {
                self.update_ref(data::HEAD, RefValue::direct(oid.clone()), true);
                Ok(oid)
            }
            err @ Err(_) => err,
        }
    }

    pub fn get_commit<T: AsRef<str>>(&self, oid: T) -> Option<Commit> {
        let oid = oid.as_ref();
        match self.get_object(oid, DataType::Commit) {
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

    pub fn checkout<T: AsRef<str>>(&self, name: T) {
        let name = name.as_ref();
        let oid = self.get_oid(name);
        match self.get_commit(&oid) {
            Some(Commit {
                tree: Some(tree_id),
                ..
            }) => {
                self.read_tree(&tree_id, true);

                let ref_value = if self.is_branch(name) {
                    RefValue {
                        symbolic: true,
                        value: format!("refs/heads/{name}"),
                    }
                } else {
                    RefValue::direct(oid)
                };

                self.update_ref(data::HEAD, ref_value, false)
            }
            _ => eprintln!("checkout not exists commit, oid:{}", name),
        }
    }

    pub fn create_tag(&self, oid: &str, tag: &str) {
        self.update_ref(
            format!("refs/tags{tag}"),
            RefValue::direct(oid.to_string()),
            true,
        )
    }

    fn is_branch(&self, branch: &str) -> bool {
        self.get_ref(&format!("refs/heads/{branch}"), true)
            .is_some()
    }

    pub fn iter_commits_and_parents(&self, oids: Vec<String>) -> Vec<String> {
        self.iter_commits_and_parents_with_fectch(oids, &|_| {})
    }

    pub fn iter_commits_and_parents_with_fectch(
        &self,
        oids: Vec<String>,
        fetch: &impl Fn(&str),
    ) -> Vec<String> {
        let mut oids = oids.into_iter().collect::<LinkedList<_>>();
        let mut visited = HashSet::new();

        let mut commits = vec![];
        while let Some(oid) = oids.pop_front() {
            if visited.contains(&oid) {
                continue;
            }

            fetch(&oid);

            if let Some(parents) = self.get_commit(&oid).map(|c| c.parents) {
                let mut parents = parents.into_iter();
                if let Some(first_parent) = parents.next() {
                    oids.push_front(first_parent.to_string());
                }

                for parent in parents {
                    oids.push_back(parent)
                }
            }

            commits.push(oid.clone());
            visited.insert(oid);
        }

        commits
    }

    pub fn create_branch<T: AsRef<str>>(&self, name: T, oid: T) {
        self.update_ref(
            format!("refs/heads/{}", name.as_ref()),
            RefValue {
                symbolic: false,
                value: oid.as_ref().to_string(),
            },
            true,
        );
    }

    pub fn get_branch_name(&self) -> Option<String> {
        let ref_value = match self.get_ref(data::HEAD, false) {
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

    pub fn reset(&self, oid: String) {
        self.update_ref(
            data::HEAD,
            RefValue {
                symbolic: false,
                value: oid,
            },
            true,
        );
    }

    pub fn get_working_tree(&self) -> HashMap<PathBuf, String> {
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
                    match self.hash_object(&path) {
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

    fn read_tree_merged(
        &self,
        t_base: Option<String>,
        t_head: &str,
        t_other: &str,
        update_working: bool,
    ) -> Result<(), DateErr> {
        let t_base_tree = match t_base.and_then(|t_base| self.get_tree_in_base(&t_base)) {
            Some(tree) => tree,
            None => HashMap::new(),
        };

        //TODO check is there un commit changes?
        let t_head_tree = match self.get_tree_in_base(t_head) {
            Some(tree) => tree,
            None => return Err(DateErr::TreeNotExists(String::from(t_head))),
        };

        let t_other_tree = match self.get_tree_in_base(t_other) {
            Some(tree) => tree,
            None => return Err(DateErr::TreeNotExists(String::from(t_other))),
        };

        let merge_tress = match self.merge_tress(&t_base_tree, &t_head_tree, &t_other_tree) {
            Ok(merged_tree) => merged_tree,
            Err(err) => return Err(err),
        };

        let mut index = self.get_index()?;

        index.clear();
        for (path, oid) in merge_tress {
            index.insert(path.to_string_lossy().to_string(), oid);
        }

        self.write_index(&index)?;

        if update_working {
            self.checkout_index(&index)?;
        }

        Ok(())
    }

    pub fn merge(&self, other: &str) {
        let head = match self
            .get_ref_recursive(data::HEAD)
            .filter(|refvalue| !refvalue.value.is_empty())
            .map(|refvalue| refvalue.value)
        {
            Some(head) => head,
            None => {
                eprintln!("merge failed, head commit not exists");
                return;
            }
        };

        let other = self.get_oid(other);
        let merge_base = self.get_merge_base(&head, &other);

        let c_other = match self.get_commit(&other).and_then(|commit| commit.tree) {
            Some(c_other) => c_other,
            None => {
                eprintln!("merge failed, commit not exists:{:?}", other);
                return;
            }
        };

        if merge_base
            .as_ref()
            .filter(|merge_base| *merge_base == &head)
            .is_some()
        {
            self.read_tree(&c_other, true);
            self.update_ref(data::HEAD, RefValue::direct(other.to_string()), true);
            println!("Fast-forward merge, no need to commit");
            return;
        }

        let c_head = match self.get_commit(&head).and_then(|commit| commit.tree) {
            Some(c_head) => c_head,
            None => {
                eprintln!("merge failed, commit not exists:{:?}", data::HEAD);
                return;
            }
        };

        let merge_base = merge_base
            .and_then(|str| self.get_commit(str))
            .and_then(|base_commit| base_commit.tree);

        self.update_ref(data::MERGE_HEAD, RefValue::direct(other.to_string()), true);

        if let Err(err) = self.read_tree_merged(merge_base, &c_head, &c_other, true) {
            eprintln!("merge failed err:{:?}", err);
        } else {
            self.update_ref(
                data::MERGE_HEAD,
                RefValue {
                    symbolic: false,
                    value: other.to_string(),
                },
                true,
            );
            println!("Merged in working tree\nPlease commit");
        }
    }

    pub fn get_merge_base(&self, oid1: &str, oid2: &str) -> Option<String> {
        let parents1: HashSet<String> =
            HashSet::from_iter(self.iter_commits_and_parents(vec![oid1.to_string()]));

        self.iter_commits_and_parents(vec![oid2.to_string()])
            .into_iter()
            .find(|oid| parents1.contains(oid))
    }

    fn build_index_tree_recursive(&self) -> Result<HashMap<String, Node>, Error> {
        let mut index_as_tree = Node::Dir(HashMap::new());
        let index = self.get_index()?;

        for (path, oid) in index {
            let path = PathBuf::from(path);
            let file_name = match path.file_name() {
                Some(file_name) => file_name.to_string_lossy(),
                None => {
                    eprintln!("write_tree error, required a filename:{:?}", path);
                    continue;
                }
            };

            let mut current_path = &mut index_as_tree;

            if let Some(components) = path.parent().map(|parent| parent.components()) {
                for component in components {
                    match component {
                        std::path::Component::Normal(ostr) => {
                            let ostr = ostr.to_string_lossy().to_string();
                            if let Node::Dir(map) = current_path {
                                current_path =
                                    map.entry(ostr).or_insert_with(|| Node::Dir(HashMap::new()));
                            } else {
                                eprintln!("required dir component:{:?}", ostr)
                            }
                        }
                        _ => {
                            eprintln!("unknow handle component:{:?}", component)
                        }
                    }
                }
            }

            if let Node::Dir(map) = current_path {
                map.insert(file_name.to_string(), Node::File(oid));
            }
        }

        match index_as_tree {
            Node::Dir(map) => Ok(map),
            _ => unreachable!(),
        }
    }

    fn write_tree_recursive(&self, tree_dict: &HashMap<String, Node>) -> Result<String, DateErr> {
        //（类型，OID,名字）
        let mut entires: Vec<(DataType, String, String)> = vec![];

        for (name, node) in tree_dict {
            let (oid, data_type) = match node {
                Node::Dir(map) => {
                    let oid = self.write_tree_recursive(map)?;
                    (oid, DataType::Tree)
                }
                Node::File(oid) => (oid.to_string(), DataType::Blob),
            };

            entires.push((data_type, oid, name.to_string()));
        }

        let mut bytes: Vec<u8> = vec![];
        for (ty, oid, name) in entires {
            for u in format!("{} {} {}\n", String::from(&ty), oid, name).as_bytes() {
                bytes.push(*u);
            }
        }

        self.hash(&bytes, DataType::Tree)
    }

    pub fn write_tree(&self) -> Result<String, DateErr> {
        let tree_dict = match self.build_index_tree_recursive() {
            Ok(tree_dict) => tree_dict,
            Err(err) => return Err(DateErr::Io(err)),
        };
        self.write_tree_recursive(&tree_dict)
    }

    pub fn iter_objects_in_commits(&self, oids: Vec<String>) -> HashSet<String> {
        self.iter_objects_in_commits_fetch(oids, &|_| {})
    }

    pub fn iter_objects_in_commits_fetch(
        &self,
        oids: Vec<String>,
        fetch: &impl Fn(&str),
    ) -> HashSet<String> {
        let mut visited = HashSet::new();

        for oid in self.iter_commits_and_parents_with_fectch(oids, fetch) {
            fetch(&oid);
            if let Some(tree) = self.get_commit(&oid).and_then(|commit| commit.tree) {
                if !visited.contains(tree.as_str()) {
                    self.iter_objects_in_tree_with_fetch(&tree, &mut visited, fetch);
                }
            }

            visited.insert(oid);
        }

        visited
    }

    fn iter_objects_in_tree_with_fetch(
        &self,
        oid: &str,
        visited: &mut HashSet<String>,
        fetch: &impl Fn(&str),
    ) {
        visited.insert(oid.to_string());

        fetch(oid);

        match self.iter_tree_entires(oid) {
            Ok(entries) => {
                for (data_type, oid, _) in entries {
                    if visited.contains(&oid) {
                        continue;
                    }

                    match data_type {
                        DataType::Tree => {
                            self.iter_objects_in_tree_with_fetch(oid.as_str(), visited, fetch);
                        }
                        _ => {
                            visited.insert(oid.clone());
                            fetch(&oid);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("iter tree entries err:{:?}, oid:{:?}", err, oid);
            }
        }
    }

    fn iter_tree_entires<T: AsRef<str>>(
        &self,
        oid: T,
    ) -> Result<Vec<(DataType, String, String)>, DateErr> {
        let oid = oid.as_ref();
        match self.get_object(oid, DataType::Tree) {
            Ok(content) => {
                let mut result = vec![];
                for line in content.lines() {
                    let splits = line.splitn(3, ' ').collect::<Vec<_>>();

                    let t = (
                        splits
                            .first()
                            .map(|dt| DataType::from(*dt))
                            .unwrap_or(DataType::None),
                        splits.get(1).unwrap_or(&"").to_string(),
                        splits.get(2).unwrap_or(&"").to_string(),
                    );

                    result.push(t);
                }

                Ok(result)
            }
            Err(err) => Err(err),
        }
    }

    pub fn is_ancestor_of(&self, commit: &str, maybe_ancesotr: &str) -> bool {
        self.iter_commits_and_parents(vec![commit.to_string()])
            .contains(&maybe_ancesotr.to_string())
    }

    fn add_directory(&self, dir: &str, map: &mut HashMap<String, String>) {
        let read_dir: fs::ReadDir = match PathBuf::from(dir).read_dir() {
            Ok(read_dir) => read_dir,
            Err(err) => {
                eprintln!("add_directory:{:?} read_dir_err:{:?}", dir, err);
                return;
            }
        };

        let mut dirs = LinkedList::new();
        dirs.push_back(read_dir);
        while let Some(read_dir) = dirs.pop_front() {
            for path in read_dir.filter_map(Result::ok) {
                let path = path.path();
                if is_ignored(&path) {
                    continue;
                }

                if path.is_file() {
                    self.add_file(&path.to_string_lossy(), map);
                } else if let Ok(dir) = path.read_dir() {
                    dirs.push_back(dir);
                }
            }
        }
    }

    fn add_file(&self, filename: &str, map: &mut HashMap<String, String>) {
        let cur_dir = match env::current_dir() {
            Ok(cur_dir) => cur_dir,
            Err(err) => {
                eprintln!("add file, get cur_dir error:{:?}", err);
                return;
            }
        };

        let filename = match PathBuf::from(filename).canonicalize() {
            Ok(path) => match path.strip_prefix(cur_dir) {
                Ok(filename) => filename.to_path_buf(),
                Err(err) => {
                    eprintln!("get relative path error:{:?}", err);
                    return;
                }
            },
            Err(err) => {
                eprintln!("add_file, canonicalize error:{:?}", err);
                return;
            }
        };

        match self.hash_object(&filename) {
            Ok(oid) => {
                let filename = filename.to_string_lossy();
                map.insert(filename.to_string(), oid);
            }
            Err(data_type) => {
                eprintln!("add file:{:?} error:{:?}", filename, data_type);
            }
        }
    }

    pub fn add(&self, filenames: &[String]) {
        let mut index = match self.get_index() {
            Ok(index) => index,
            Err(err) => {
                eprintln!("get_index error:{:?}", err);
                return;
            }
        };

        for filename in filenames {
            let path = PathBuf::from(filename);
            if path.is_file() {
                self.add_file(filename, &mut index);
            } else {
                self.add_directory(filename, &mut index);
            }
        }

        if let Err(err) = self.write_index(&index) {
            eprintln!("add file, update index error:{:?}", err);
        }
    }

    fn checkout_index(&self, index: &HashMap<String, String>) -> Result<(), DateErr> {
        for (path, oid) in index {
            let pathbuf = PathBuf::from(path);
            if let Some(pathbuf) = pathbuf.parent() {
                fs::create_dir_all(pathbuf)?;
            }

            let object = self.get_object(oid, DataType::Blob)?;
            fs::write(path, object)?;
        }

        Ok(())
    }
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
