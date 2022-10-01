use crate::structures::{read_from_disk, Sha};
use std::io;

pub fn cat_file_pretty_print(sha: &Sha) -> Result<(), io::Error> {
    let object = read_from_disk(sha.clone())?;
    object.pretty_print();
    Ok(())
}

pub fn cat_file_print_type(sha: &Sha) -> Result<(), io::Error> {
    let object = read_from_disk(sha.clone())?;
    object.print_type();
    Ok(())
}
