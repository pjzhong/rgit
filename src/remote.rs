use std::path;

use crate::data::{RefValue, Ugit};

const REMOTE_REF_BASE: &str = "refs/heads";
const LOCAL_REFS_BASE: &str = "refs/remote";

impl Ugit {
    pub fn fetch(&mut self, remote_path: String) {
        println!("Will fetch the following refs:");
        for (ref_name, val) in self.get_remote_refs(remote_path, REMOTE_REF_BASE) {
            println!("- {ref_name}");
            self.update_ref(
                format!("{LOCAL_REFS_BASE}/{ref_name}"),
                RefValue::direct(val),
                true,
            );
        }
    }

    fn get_remote_refs(&mut self, remote_path: String, prefix: &str) -> Vec<(String, String)> {
        let old_dir =
            self.change_git_dir(format!("{}{}.rgit", remote_path, path::MAIN_SEPARATOR_STR,));
        let mut vec = vec![];
        for ref_name in self.iter_refs_prefix(prefix) {
            if let Some(ref_val) = self.get_ref_if_not_empty(&ref_name) {
                vec.push((ref_name, ref_val.value));
            }
        }
        self.change_git_dir(old_dir);

        vec
    }
}
