use std::io;

use crate::{
    commands::{commit_tree, write_tree},
    structures::{load_commit_from_sha, IndexFile, RefFile},
    utils::get_current_branch,
};

use super::update_ref_basic;

pub fn commit(messages: &Option<Vec<String>>) -> Result<(), io::Error> {
    let current_branch = get_current_branch().expect("Failed to get current branch");
    let mut index_file = IndexFile::from_disk().expect("failed to create index file");

    let sha = write_tree(false).expect("Failed to write tree");
    match RefFile::new_from_branch(&current_branch) {
        Ok(ref_file) => {
            if let Ok(head) = load_commit_from_sha(ref_file.sha()) {
                commit_tree(&sha, messages, head.parent()).expect("Failed to commit tree");
            } else {
                panic!("idk");
            }
        }
        Err(_) => {
            commit_tree(&sha, messages, &None).expect("failed to commit tree");
        }
    }
    update_ref_basic(&current_branch, &sha).expect("Failed to update ref");
    index_file.clear_all_entries();
    Ok(())
}
