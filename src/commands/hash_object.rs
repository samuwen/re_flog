use crate::structures::{Blob, GitObject};
use log::*;
use std::ffi::OsStr;
use std::io;
use std::path::Path;

pub fn hash_object<P: AsRef<OsStr>>(file_path: P, print_output: bool) -> Result<Blob, io::Error> {
    info!("Hashing object");
    let path = Path::new(&file_path);
    let blob = Blob::new_from_raw_file(path);
    if print_output {
        println!("{}", blob.sha());
    }
    Ok(blob)
}

pub fn hash_and_write_to_db<P: AsRef<OsStr>>(file_path: P) -> Result<(), io::Error> {
    info!("Writing hash to database");
    let path = Path::new(&file_path);
    let mut blob = Blob::new_from_raw_file(path);
    blob.write_to_disk()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_obj_and_write() {
        hash_and_write_to_db("/home/samuwen/Documents/repos/entirely_fake_repo/boop/README.md")
            .unwrap();
        assert!(
            Path::new(".re_flogged/objects/80/2992c4220de19a90767f3000a79a31b98d0df7").exists()
        );
    }
}
