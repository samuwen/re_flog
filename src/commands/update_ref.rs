use std::io;

use log::info;

use crate::structures::{RefFile, Sha};

pub fn update_ref_basic(reff: &String, new_value: &Sha) -> Result<(), io::Error> {
    info!(
        "Update ref basic with reff {} and new_value {}",
        reff, new_value
    );
    let ref_file = RefFile::new(reff, new_value);
    ref_file.write()?;
    Ok(())
}
