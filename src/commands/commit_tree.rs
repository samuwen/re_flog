use std::io;

use crate::structures::{Commit, GitObject, Sha};

pub fn commit_tree(
    sha: &Sha,
    message: &Option<Vec<String>>,
    parent: &Option<Vec<Sha>>,
) -> Result<(), io::Error> {
    let mut commit = Commit::new_from_tree_sha(sha, message, parent)?;
    commit.write_to_disk()?;
    Ok(())
}
