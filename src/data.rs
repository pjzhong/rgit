use std::env;
use std::fs::{create_dir, File};
use std::io::{Error, Read, Write};
use std::path::PathBuf;

use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub const GIT_DIR: &str = ".rgit";

#[derive(Debug)]
pub enum DateErr {
    ContentMisMatch(String),
    Io(Error),
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
}

impl From<&DataType> for String {
    fn from(value: &DataType) -> Self {
        match value {
            DataType::None => String::from("None"),
            DataType::Blob => String::from("Blob"),
        }
    }
}

pub fn hash(bytes: &[u8], ty: DataType) {
    let mut haser = Sha1::new();
    haser.input(bytes);
    let hex = haser.result_str();

    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR).join(hex);
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
        }
        Err(r) => eprintln!("open file:{:?} err:{:?}", current_dir, r),
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
