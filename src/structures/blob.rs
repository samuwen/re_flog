use std::{
    ffi::OsStr,
    fs::{create_dir_all, File},
    io::{self, BufRead, Read, Write},
    os::unix::prelude::MetadataExt,
    path::Path,
};

use derive_getters::Getters;
use log::{debug, info};

use crate::structures::compress;

use super::{GitObject, Sha};

#[derive(Clone, Debug, Getters, PartialEq)]
pub struct Blob {
    mode: String,
    data: Vec<u8>,
    sha: Sha,
}

impl Blob {
    /// Creates a new Blob from a path to a real file on the FS
    pub fn new_from_raw_file<P: AsRef<OsStr>>(path: P) -> Self {
        info!("New blob from raw file");
        let path = Path::new(&path);
        let mut file = File::open(path).expect("Failed to read file");
        let mut file_buffer = vec![];
        file.read_to_end(&mut file_buffer)
            .expect("failed to read file");
        let mut mode = file.metadata().unwrap().mode() as u32;
        if mode == 0o100664 {
            mode = 0o100644; // git only supports 3 modes for files
        }
        Self::new_from_bytes(file_buffer, mode)
    }

    /// Creates a new Blob from bytes already read into memory
    pub fn new_from_bytes(mut bytes: Vec<u8>, mode: u32) -> Self {
        let mut heading = Blob::create_heading(bytes.len());
        let mut all_data = vec![];
        all_data.append(&mut heading);
        all_data.append(&mut bytes);
        let sha = Sha::new_hash(&all_data);
        Self {
            data: all_data,
            sha,
            mode: format!("{:o}", mode),
        }
    }

    /// Creates a blob from a reader with a known length and known SHA
    pub fn new_from_disk<R: BufRead>(
        reader: &mut R,
        count: usize,
        sha: Sha,
        mode: u32,
    ) -> Result<Self, io::Error> {
        info!("Reading blob from disk");
        let mut buf = vec![0; count as usize];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            data: buf,
            sha,
            mode: format!("{:o}", mode),
        })
    }

    pub fn print_type(&self) {
        println!("blob");
    }

    pub fn pretty_print_contents(&self) {
        let as_string = self.data.iter().fold(String::new(), |mut acc, &cur| {
            acc.push(cur as char);
            acc
        });
        println!("{}", as_string);
    }

    fn create_heading(size: usize) -> Vec<u8> {
        let heading = format!("blob {}\0", size);
        heading.chars().map(|ch| ch as u8).collect()
    }
}

impl GitObject for Blob {
    fn write_to_disk(&mut self) -> Result<(), io::Error> {
        debug!("Write to disk called");
        let compressed_data = compress(&*self.data);
        let full_path = self.sha.to_path();
        let dir_path = full_path.parent().unwrap();
        debug!("write directory: {:?}", dir_path);
        if !dir_path.exists() {
            info!("Path doesn't exist, creating");
            create_dir_all(&dir_path)?;
        }
        debug!("Full write directory: {:?}", full_path);
        let mut file = File::create(&full_path)?;
        file.write_all(&compressed_data)?;
        Ok(())
    }

    fn pretty_print(&self) {
        let as_string = self.data.iter().fold(String::new(), |mut acc, &cur| {
            acc.push(cur as char);
            acc
        });
        println!("{}", as_string);
    }

    fn print_type(&self) {
        println!("blob");
    }

    fn get_sha(&self) -> &Sha {
        &self.sha
    }
}
