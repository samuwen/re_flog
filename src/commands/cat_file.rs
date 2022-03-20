use crate::structures::{read_from_disk, Sha};
use std::io;

pub fn cat_file_pretty_print(sha: &str) -> Result<(), io::Error> {
    let sha = Sha::new_from_str(sha);
    let object = read_from_disk(sha)?;
    object.pretty_print();
    Ok(())
}

pub fn cat_file_print_type(sha: &str) -> Result<(), io::Error> {
    let sha = Sha::new_from_str(sha);
    let object = read_from_disk(sha)?;
    object.print_type();
    Ok(())
}
