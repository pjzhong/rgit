use std::env;
use std::fs::File;
use std::path::PathBuf;

pub const GIT_DIR: &str = ".rgit";

pub fn init() {
    let current_dir =  env::current_dir().expect("failed to obtain current dir");
    let current_dir: PathBuf = current_dir.join(GIT_DIR);
    match File::create(&current_dir) {
        Ok(_) => {
            println!("Initialized empty rgit repository in {:?}", current_dir)
        }
        Err(r) => eprintln!("Initi rgit repository err:{:?}", r),
    }
}
