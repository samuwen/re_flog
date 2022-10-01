use std::io;

use crate::structures::{RefFile, Sha};

pub fn update_ref_basic(reff: &String, new_value: &Sha) -> Result<(), io::Error> {
    let ref_file = RefFile::new(reff, new_value);
    ref_file.write()?;
    Ok(())
}
