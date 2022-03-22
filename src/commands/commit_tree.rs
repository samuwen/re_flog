use std::io;

use crate::structures::{Commit, GitObject, Sha};

pub fn commit_tree(
    sha: &String,
    message: &Option<Vec<String>>,
    parent: &Option<Vec<String>>,
) -> Result<(), io::Error> {
    let sha = Sha::new_from_str(sha);
    let mut commit = Commit::new_from_tree_sha(sha, message, parent)?;
    commit.write_to_disk()?;
    Ok(())
}
