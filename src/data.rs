use std::collections::LinkedList;
use std::fs::{create_dir, File};
use std::io::{Error, Read, Write};
use std::mem::{self};
use std::path::{Path, PathBuf};
use std::{env, fs, vec};

use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub const HEAD: &str = "HEAD";
pub const MERGE_HEAD: &str = "MERGE_HEAD";
pub const REF_PREFIX: &str = "ref: ";
pub const DELIMITER: u8 = b'\x00';

pub struct Ugit {
    git_dir: String,
}

impl Default for Ugit {
    fn default() -> Self {
        pub const GIT_DIR: &str = "./.rgit";

        Self {
            git_dir: GIT_DIR.to_string(),
        }
    }
}

impl Ugit {
    pub fn init(&self) {
        let current_dir = env::current_dir().expect("failed to obtain current dir");
        let current_dir: PathBuf = current_dir.join(&self.git_dir);
        let objects_dir: PathBuf = current_dir.join("objects");
        match create_dir(&current_dir) {
            Ok(_) => {
                println!("Initialized empty rgit repository in {:?}", current_dir)
            }
            Err(r) => eprintln!("Initi rgit repository err:{:?}", r),
        }

        if let Err(err) = create_dir(&objects_dir) {
            eprintln!(
                "Initi rgit object repository err:{:?}, path:{:?}",
                err, objects_dir
            );
        }
    }

    pub fn hash(&self, bytes: &[u8], ty: DataType) -> Result<String, DateErr> {
        let mut haser = Sha1::new();
        haser.input(bytes);
        let hex = haser.result_str();

        let current_dir = env::current_dir().expect("failed to obtain current dir");
        let current_dir: PathBuf = current_dir.join(&self.git_dir).join("objects").join(&hex);
        match File::create(&current_dir) {
            Ok(mut f) => {
                let mut datas: Vec<u8> = vec![];
                let str: String = (&ty).into();
                for u in str.as_bytes() {
                    datas.push(*u);
                }
                datas.push(DELIMITER);
                for u in bytes {
                    datas.push(*u);
                }
                if let Err(e) = f.write_all(&datas) {
                    eprintln!("write to {:?} err:{:?}", current_dir, e);
                }

                Ok(hex)
            }
            Err(r) => {
                eprintln!("create file:{:?} err:{:?}", current_dir, r);
                Err(r.into())
            }
        }
    }

    pub fn hash_object(&self, path: &PathBuf) -> Result<String, DateErr> {
        match File::open(path) {
            Ok(mut f) => {
                let mut buffers = Vec::new();
                match f.read_to_end(&mut buffers) {
                    Ok(_) => match self.hash(&buffers, DataType::Blob) {
                        Ok(hex) => Ok(hex),
                        Err(err) => Err(err),
                    },
                    Err(e) => Err(e.into()),
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_object(&self, oid: &str, expected: DataType) -> Result<String, DateErr> {
        let current_dir = env::current_dir().expect("failed to obtain current dir");
        let current_dir: PathBuf = current_dir.join(&self.git_dir).join("objects").join(oid);

        let obj = match File::open(current_dir) {
            Ok(mut f) => {
                let mut buffer = vec![];
                match f.read_to_end(&mut buffer) {
                    Ok(_) => buffer,
                    Err(e) => return Err(DateErr::Io(e)),
                }
            }
            Err(e) => return Err(DateErr::Io(e)),
        };

        let (ty, content) = {
            let bytes = obj.splitn(2, |v| v == &DELIMITER).collect::<Vec<_>>();
            let ty_bytes = bytes.get(0);
            let content_bytes = bytes.get(1);
            (
                ty_bytes
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_else(String::new),
                content_bytes
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_else(String::new),
            )
        };

        let expect_str: String = (&expected).into();
        match expected {
            DataType::None => Ok(content),
            _ => {
                if expect_str != ty {
                    Err(DateErr::ContentMisMatch(format!(
                        "found:{}, expected:{}",
                        ty, expect_str
                    )))
                } else {
                    Ok(content)
                }
            }
        }
    }

    pub fn delete_ref(&self, ref_str: &str, deref: bool) -> Result<(), DateErr> {
        let (ref_str, _) = self.get_ref_internal(ref_str, deref);
        let path = PathBuf::from(&self.git_dir).join(ref_str);
        if path.is_file() {
            if let Err(err) = fs::remove_file(path) {
                return Err(DateErr::Io(err));
            }
        }
        Ok(())
    }

    pub fn update_ref<T: AsRef<str>>(&self, ref_str: T, value: RefValue, deref: bool) {
        let ref_str = ref_str.as_ref();
        let ref_str = self.get_ref_internal(ref_str, deref).0;

        let path = PathBuf::from(&self.git_dir).join(ref_str);

        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("update_ref, create dirs error:{:?}", e);
            }
        }

        match File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
        {
            Ok(mut f) => {
                let value = if value.symbolic {
                    format!("{REF_PREFIX}{}", value.value)
                } else {
                    value.value
                };
                if let Err(err) = f.write_all(value.as_bytes()) {
                    println!("update_ref  ref:{:?} err:{:?}", value, err);
                }
            }
            Err(e) => eprintln!("update_ref error, err:{:?}", e),
        }
    }

    /// ['ref_str']: /ref/heads/branch or /refs/tags/test
    fn get_ref_internal(&self, ref_str: &str, deref: bool) -> (String, RefValue) {
        let value = {
            let path = PathBuf::from(&self.git_dir).join(ref_str);
            match File::open(path) {
                Ok(mut f) => {
                    let mut str = String::new();
                    match f.read_to_string(&mut str) {
                        Ok(_) => str,
                        Err(_) => String::default(),
                    }
                }
                Err(_) => String::default(),
            }
        };

        let (symbolic, value) = if let Some(symblock) = value.strip_prefix(REF_PREFIX) {
            (true, symblock.to_string())
        } else {
            (false, value)
        };

        if symbolic && deref {
            self.get_ref_internal(&value, deref)
        } else {
            (ref_str.to_string(), RefValue { symbolic, value })
        }
    }

    pub fn get_ref(&self, ref_str: &str, deref: bool) -> Option<RefValue> {
        Some(self.get_ref_internal(ref_str, deref).1)
    }

    pub fn get_ref_recursive(&self, ref_str: &str) -> Option<RefValue> {
        Some(self.get_ref_internal(ref_str, true).1)
    }

    pub fn get_ref_if_not_empty(&self, ref_str: &str) -> Option<RefValue> {
        Some(self.get_ref_internal(ref_str, true).1).filter(|ref_val| !ref_val.value.is_empty())
    }

    pub fn iter_refs_prefix(&self, prefix: &str) -> Vec<String> {
        let mut refs = vec![String::from(HEAD), String::from(MERGE_HEAD)];

        let refs_path = PathBuf::from(&self.git_dir).join("refs");
        let mut dirs = LinkedList::new();
        dirs.push_back(refs_path);

        while let Some(dir) = dirs.pop_front() {
            let read_dir = match dir.read_dir() {
                Ok(read_dir) => read_dir,
                Err(err) => {
                    println!("err:{:?}", err);
                    continue;
                }
            };

            for dir in read_dir.filter_map(Result::ok) {
                let file_type = match dir.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    if let Some(path) = dir
                        .path()
                        .strip_prefix(&self.git_dir)
                        .ok()
                        .and_then(Path::to_str)
                        .filter(|str| str.starts_with(prefix))
                    {
                        refs.push(String::from(path));
                    }
                } else {
                    dirs.push_back(dir.path());
                }
            }
        }

        refs.into_iter()
            .filter(|ref_name| ref_name.starts_with(prefix))
            .collect::<Vec<_>>()
    }

    pub fn iter_refs(&self) -> Vec<String> {
        self.iter_refs_prefix("")
    }

    pub fn iter_branch_names(&self) -> Vec<String> {
        let mut branchs = vec![];
        let binding = PathBuf::from("refs").join("heads").join("");
        let prefix = binding.to_str().unwrap_or_default();
        for ref_name in self
            .iter_refs_prefix(prefix)
            .iter()
            .filter_map(|str| str.strip_prefix(prefix))
        {
            branchs.push(ref_name.to_string());
        }

        branchs
    }

    pub fn change_git_dir(&mut self, new_dir: String) -> String {
        mem::replace(&mut self.git_dir, new_dir)
    }

    pub fn objects_exists(&self, oid: &str) -> bool {
        PathBuf::from(&self.git_dir)
            .join("objects")
            .join(oid)
            .is_file()
    }

    pub fn fetch_object_if_missing(&self, oid: &str, remote_git_dir: &str) -> Result<(), Error> {
        if self.objects_exists(oid) {
            return Ok(());
        }

        println!("fetching oid:{:?}", oid);
        match fs::read_to_string(
            PathBuf::from(remote_git_dir)
                .join(".rgit")
                .join("objects")
                .join(oid),
        ) {
            Ok(str) => {
                match File::options()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(PathBuf::from(&self.git_dir).join("objects").join(oid))
                {
                    Ok(mut f) => match f.write_all(str.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(err),
                    },
                    Err(err) => Err(err),
                }
            }
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug)]
pub enum DateErr {
    ContentMisMatch(String),
    TreeNotExists(String),
    Io(Error),
    Err(String),
}

pub struct RefValue {
    pub symbolic: bool,
    pub value: String,
}

impl RefValue {
    pub fn direct(value: String) -> Self {
        Self {
            symbolic: false,
            value,
        }
    }

    pub fn symbolic(value: impl Into<String>) -> Self {
        Self {
            symbolic: true,
            value: value.into(),
        }
    }
}

impl From<Error> for DateErr {
    fn from(value: Error) -> Self {
        DateErr::Io(value)
    }
}

pub enum DataType {
    None,
    Blob,
    Tree,
    Commit,
}

impl From<&DataType> for String {
    fn from(value: &DataType) -> Self {
        match value {
            DataType::None => String::from("None"),
            DataType::Blob => String::from("Blob"),
            DataType::Tree => String::from("Tree"),
            DataType::Commit => String::from("Commit"),
        }
    }
}

impl From<&str> for DataType {
    fn from(value: &str) -> Self {
        match value {
            "Blob" => DataType::Blob,
            "Tree" => DataType::Tree,
            "Commit" => DataType::Commit,
            _ => DataType::None,
        }
    }
}
