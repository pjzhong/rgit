use std::path;

use crate::data::{RefValue, Ugit};

const REMOTE_REF_BASE: &str = "refs/heads";
const LOCAL_REFS_BASE: &str = "refs/remote";

impl Ugit {
    pub fn push(&mut self, remote_path: &str, ref_name: &str) {
        let known_remote_refs = self
            .get_remote_refs(remote_path, "")
            .into_iter()
            .filter(|refvalue| self.objects_exists(&refvalue.1))
            .map(|refvalue| refvalue.1)
            .collect::<Vec<_>>();
        let ref_val = match self.get_ref_if_not_empty(ref_name) {
            Some(ref_val) => ref_val.value,
            None => {
                eprintln!("ref not exists, ref:{:?}", ref_name);
                return;
            }
        };

        let remote_objects = self.iter_objects_in_commits(known_remote_refs);
        let local_objects = self.iter_objects_in_commits(vec![ref_val.clone()]);
        let objects_to_push = local_objects.difference(&remote_objects);

        for oid in objects_to_push {
            println!("{:?}", oid);
            if let Err(err) = self.push_object(&oid, remote_path) {
                eprintln!("push object to {:?} err:{:?}", remote_path, err);
            }
        }

        let old_dir =
            self.change_git_dir(format!("{}{}.rgit", remote_path, path::MAIN_SEPARATOR_STR));
        self.update_ref(ref_name, RefValue::direct(ref_val.to_string()), true);
        self.change_git_dir(old_dir);
    }

    pub fn fetch(&mut self, remote_path: String) {
        println!("Will fetch the following refs:");
        let refs = self.get_remote_refs(&remote_path, REMOTE_REF_BASE);

        let oids = refs.iter().map(|refs| refs.1.clone()).collect::<Vec<_>>();
        self.iter_objects_in_commits_fetch(oids, &|str| {
            if let Err(err) = self.fetch_object_if_missing(str, &remote_path) {
                eprintln!(
                    "fetch remote object err, remote_path:{:?}, oid:{:?}, err:{:?}",
                    remote_path, str, err
                );
            }
        });

        for (ref_name, val) in refs {
            if let Some(ref_name) = ref_name.strip_prefix(REMOTE_REF_BASE) {
                println!("- {ref_name}");
                self.update_ref(
                    format!("{LOCAL_REFS_BASE}{ref_name}"),
                    RefValue::direct(val),
                    true,
                );
            }
        }
    }

    fn get_remote_refs(&mut self, remote_path: &str, prefix: &str) -> Vec<(String, String)> {
        let old_dir =
            self.change_git_dir(format!("{}{}.rgit", remote_path, path::MAIN_SEPARATOR_STR));
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
