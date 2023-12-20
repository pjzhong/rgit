use crate::data::Ugit;

impl Ugit {
    pub fn fetch(&mut self, remote_path: String) {
        println!("Will fetch the following refs:");
        let old_dir = self.change_git_dir(remote_path);
        for ref_name in self.iter_refs_prefix("refs/heads") {
            if self.get_ref_if_not_empty(&ref_name).is_some() {
                println!("- {ref_name}");
            }
        }
        self.change_git_dir(old_dir);
    }
}
