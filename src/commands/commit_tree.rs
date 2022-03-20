use std::io;

use crate::structures::{Commit, GitObject, Sha};

pub fn commit_tree(sha: &String, parent: &Option<String>) -> Result<(), io::Error> {
    let sha = Sha::new_from_str(sha);
    let mut commit = Commit::new_from_tree_sha(sha, parent)?;
    commit.write_to_disk()?;
    Ok(())
}
