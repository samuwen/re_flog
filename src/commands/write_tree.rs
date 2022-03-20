use std::io;

use crate::structures::{GitObject, IndexFile, Tree};

pub fn write_tree(missing_ok: bool) -> Result<(), io::Error> {
    let idx_file = IndexFile::from_disk()?;
    let mut root_tree = Tree::empty();
    for entry in idx_file.index_entries() {
        let slash_count = entry
            .file_name()
            .iter()
            .filter(|&&b| b as char == '/')
            .count();
        if slash_count < 1 {
            root_tree.add_blob_from_index(entry);
        } else {
            root_tree.add_tree_from_index(entry, missing_ok);
        }
    }
    root_tree.write_to_disk()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w() {
        write_tree(false);
        assert_eq!(true, false);
    }
}
