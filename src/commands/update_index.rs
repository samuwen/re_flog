use crate::structures::IndexFile;
use std::io;
use std::path::PathBuf;

pub fn update_index_add(path_strings: &Vec<PathBuf>) -> Result<(), io::Error> {
    let mut index_file = IndexFile::new(path_strings.len() as u32);
    index_file.add_files(path_strings);
    index_file.write()?;
    Ok(())
}

pub fn update_index_remove(path_strings: &Vec<PathBuf>) -> Result<(), io::Error> {
    let mut index_file = IndexFile::new(path_strings.len() as u32);
    index_file.remove_files(path_strings);
    index_file.write()?;
    Ok(())
}
