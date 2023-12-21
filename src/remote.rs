use crate::data::Ugit;

impl Ugit {
    pub fn fetch(&mut self, remote_path: String) {
        println!("Will fetch the following refs:");
        for ref_name in self.get_remote_refs(remote_path, "refs/heads") {
            if self.get_ref_if_not_empty(&ref_name).is_some() {
                println!("- {ref_name}");
            }
        }
    }

    fn get_remote_refs(&mut self, remote_path: String, prefix: &str) -> Vec<String> {
        let old_dir = self.change_git_dir(remote_path);
        let iter = self.iter_refs_prefix(prefix);
        self.change_git_dir(old_dir);

        iter
    }
}
