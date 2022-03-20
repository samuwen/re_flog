use std::io;

use crate::structures::RefFile;

pub fn update_ref_basic(reff: &String, new_value: &String) -> Result<(), io::Error> {
    let ref_file = RefFile::new(reff, new_value);
    ref_file.write()?;
    Ok(())
}
