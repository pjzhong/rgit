use std::env;
use std::fs::{create_dir, File};
use std::io::{Error, Read, Write};
use std::path::PathBuf;

use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub const GIT_DIR: &str = ".rgit";

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

pub fn hash(bytes: &[u8]) {
    let mut haser = Sha1::new();
    haser.input(bytes);
    let hex = haser.result_str();

    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR).join(hex);
    match File::create(&current_dir) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(bytes) {
                eprintln!("write to {:?} err:{:?}", current_dir, e);
            }
        }
        Err(r) => eprintln!("open file:{:?} err:{:?}", current_dir, r),
    }
}

pub fn get_object(oid: &str) -> Result<String, Error> {
    let current_dir = env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR).join(oid);

    match File::open(current_dir) {
        Ok(mut f) => {
            let mut buffer = String::new();
            match f.read_to_string(&mut buffer) {
                Ok(_) => Ok(buffer),
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}
