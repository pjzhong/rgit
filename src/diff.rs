use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::PathBuf,
    process::Command,
};

use tempfile::NamedTempFile;

use crate::data::{self, DateErr};

//比较两个目录，同一个key指向不同内容，则发生了变化
pub fn diff_tree(t_from: &HashMap<PathBuf, String>, t_to: &HashMap<PathBuf, String>) -> String {
    let keys = merge_key(t_from, t_to);

    let mut output = String::new();
    for k in keys {
        let (from, to) = (t_from.get(k), t_to.get(k));
        if from != to {
            output.push_str(&format!("changed:{:?}\n", k))
        }
    }

    output
}

pub fn iter_changed_files(
    t_from: &HashMap<PathBuf, String>,
    t_to: &HashMap<PathBuf, String>,
) -> HashMap<PathBuf, String> {
    let keys = merge_key(t_from, t_to);

    let mut map = HashMap::new();
    for k in keys {
        let action = match (t_from.get(k), t_to.get(k)) {
            (None, Some(_)) => "new file",
            (Some(_), None) => "deleted",
            (Some(_), Some(_)) => "modified",
            (None, None) => "Unknow files",
        };
        map.insert(k.clone(), String::from(action));
    }

    map
}

fn merge_key<'a>(
    t_from: &'a HashMap<PathBuf, String>,
    t_to: &'a HashMap<PathBuf, String>,
) -> HashSet<&'a PathBuf> {
    let mut keys = HashSet::new();
    for k in t_from.keys() {
        keys.insert(k);
    }

    for k in t_to.keys() {
        keys.insert(k);
    }

    keys
}

pub fn merge_tress(
    t_from: &HashMap<PathBuf, String>,
    t_to: &HashMap<PathBuf, String>,
) -> Result<HashMap<PathBuf, String>, DateErr> {
    let keys = merge_key(t_from, t_to);

    let mut tree = HashMap::new();
    for k in keys {
        let (from, other) = (t_from.get(k), t_to.get(k));
        match merge_blobs(from.map(String::as_str), other.map(String::as_str)) {
            Ok(content) => {
                tree.insert(k.clone(), content);
            }
            Err(err) => return Err(err),
        }
    }

    Ok(tree)
}

pub fn merge_blobs(o_head: Option<&str>, o_other: Option<&str>) -> Result<String, DateErr> {
    let mut f_head = match NamedTempFile::new() {
        Ok(f_head) => f_head,
        Err(err) => return Err(DateErr::Io(err)),
    };
    let mut f_other = match NamedTempFile::new() {
        Ok(f_ohter) => f_ohter,
        Err(err) => return Err(DateErr::Io(err)),
    };

    for (oid, f) in [(o_head, &mut f_head), (o_other, &mut f_other)] {
        let oid = match oid {
            Some(oid) => oid,
            None => continue,
        };

        match data::get_object(oid, data::DataType::None) {
            Ok(content) => {
                if let Err(err) = f.as_file_mut().write_all(content.as_bytes()) {
                    eprintln!(
                        "merge_blogs, write temp file failed, oid:{:?}, err:{:?}",
                        oid, err
                    );
                }
            }
            Err(err) => {
                eprintln!(
                    "merge_blogs, get_object failed, oid:{:?}, err:{:?}",
                    oid, err
                );
            }
        }
    }

    let (f_head_path_str, f_other_path_str) = (
        f_head.path().to_string_lossy(),
        f_other.path().to_string_lossy(),
    );

    match Command::new("diff")
        .args(["-DHEAD", &f_head_path_str, &f_other_path_str])
        .output()
    {
        Ok(output) => Ok(String::from_utf8_lossy(output.stdout.as_slice()).to_string()),
        Err(err) => Err(DateErr::Io(err)),
    }
}
