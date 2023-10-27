use std::fs::{create_dir, File};
use std::io::{Error, Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub const GIT_DIR: &str = ".rgit";

#[derive(Debug)]
pub enum DateErr {
    ContentMisMatch(String),
    Io(Error),
    Err(String),
}

impl From<Error> for DateErr {
    fn from(value: Error) -> Self {
        DateErr::Io(value)
    }
}

pub fn init() {
    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR);
    match create_dir(&current_dir) {
        Ok(_) => {
            println!("Initialized empty rgit repository in {:?}", current_dir)
        }
        Err(r) => eprintln!("Initi rgit repository err:{:?}", r),
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

pub fn hash_object(path: &PathBuf) -> Result<String, DateErr> {
    match File::open(path) {
        Ok(mut f) => {
            let mut buffers = Vec::new();
            match f.read_to_end(&mut buffers) {
                Ok(_) => match hash(&buffers, DataType::Blob) {
                    Ok(hex) => Ok(hex),
                    Err(err) => Err(err),
                },
                Err(e) => Err(e.into()),
            }
        }
        Err(e) => Err(e.into()),
    }
}

pub fn hash(bytes: &[u8], ty: DataType) -> Result<String, DateErr> {
    let mut haser = Sha1::new();
    haser.input(bytes);
    let hex = haser.result_str();

    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR).join(&hex);
    match File::create(&current_dir) {
        Ok(mut f) => {
            let mut datas: Vec<u8> = vec![];
            let str: String = (&ty).into();
            for u in str.as_bytes() {
                datas.push(*u);
            }
            datas.push(b'\x00');
            for u in bytes {
                datas.push(*u);
            }
            if let Err(e) = f.write_all(&datas) {
                eprintln!("write to {:?} err:{:?}", current_dir, e);
            }

            Ok(hex)
        }
        Err(r) => {
            eprintln!("open file:{:?} err:{:?}", current_dir, r);
            Err(r.into())
        }
    }
}

pub fn get_object(oid: &str, expected: DataType) -> Result<String, DateErr> {
    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR).join(oid);

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
        let bytes = obj.splitn(2, |v| v == &b'\x00').collect::<Vec<_>>();
        (
            String::from_utf8_lossy(bytes[0]).to_string(),
            String::from_utf8_lossy(bytes[1]).to_string(),
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

pub fn set_head(oid: &str) {
    let path = PathBuf::from(GIT_DIR).join("HEAD");

    match File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
    {
        Ok(mut f) => {
            if let Err(err) = f.write_all(oid.as_bytes()) {
                eprintln!("set head err:{:?}", err);
            }
        }
        Err(e) => eprintln!("set_head error, err:{:?}", e),
    }
}

pub fn get_head() -> Option<String> {
    let path = PathBuf::from(GIT_DIR).join("HEAD");
    match File::open(path) {
        Ok(mut f) => {
            let mut str = String::new();
            match f.read_to_string(&mut str) {
                Ok(_) => Some(str),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}
