use crate::structures::IndexFile;
use std::io;

pub fn ls_files_staging() -> Result<(), io::Error> {
    let file = IndexFile::from_disk()?;
    for entry in file.index_entries().iter() {
        let stage_number = 0; // will be useful later
        println!(
            "{:o} {} {}\t{}",
            entry.mode(),
            entry.get_readable_sha(),
            stage_number,
            entry.get_readable_file_name()
        );
    }
    Ok(())
}
