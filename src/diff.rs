use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

//比较两个目录，同一个key指向不同内容，则发生了变化
pub fn diff_tree(t_from: &HashMap<PathBuf, String>, t_to: &HashMap<PathBuf, String>) -> String {
    let keys = {
        let mut keys = HashSet::new();
        for k in t_from.keys() {
            keys.insert(k);
        }

        for k in t_to.keys() {
            keys.insert(k);
        }

        keys
    };

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
    let keys = {
        let mut keys = HashSet::new();
        for k in t_from.keys() {
            keys.insert(k);
        }

        for k in t_to.keys() {
            keys.insert(k);
        }

        keys
    };

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
