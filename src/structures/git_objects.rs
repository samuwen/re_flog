use std::{
    fs::{metadata, File},
    io::{self, BufRead, BufReader, Cursor},
    os::unix::prelude::MetadataExt,
    path::Path,
};

use log::{debug, error};

pub use crate::structures::{Blob, Tree};

use crate::{
    exit_with_message,
    structures::{decompress, Commit},
    utils::iterable_to_string,
};

use super::Sha;

pub trait GitObject {
    fn write_to_disk(&mut self) -> Result<(), io::Error>;
    fn pretty_print(&self);
    fn print_type(&self);
    fn get_sha(&self) -> &Sha;
}

pub fn check_file_is_of_kind(sha: &Sha, kind: &str) -> bool {
    let path = sha.to_path();
    let mut reader = {
        let this = init_bufreader(&path);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("Error checking file: {}", e);
                let msg = format!("Invalid sha: {}", sha);
                exit_with_message(&msg);
            }
        }
    };
    let mut buf = vec![];
    reader.read_until(0, &mut buf).unwrap();
    let signature = match kind {
        "commit" => vec!['c', 'o', 'm', 'm', 'i', 't'],
        "tree" => vec!['t', 'r', 'e', 'e'],
        "blob" => vec!['b', 'l', 'o', 'b'],
        _ => panic!("Invalid object type"),
    };
    let count = check_header_is_valid(&buf, signature);
    count > 0
}

pub fn read_from_disk(sha: Sha) -> Result<Box<dyn GitObject>, io::Error> {
    let mut buf = vec![];
    let path = Path::new("/home/samuwen/Documents/repos/multi_merge/.git/objects/63/12c14f195ad8be7dfe2fd682c9b6b6bc71c9a3");
    // let path = Path::new("/home/samuwen/Documents/repos/re_flog/.re_flogged/objects/24/e0ed76e64a48945bd93a1d2cf00ba9c6294a8c");
    // let path = sha.to_path();
    debug!("Path: {:?}", path);
    let mut reader = init_bufreader(&path)?;
    reader.read_until(0, &mut buf)?;
    let object: Box<dyn GitObject> = match buf[0] as char {
        'b' => {
            let mode = metadata(path).unwrap().mode() as u32;
            let count = check_header_is_valid(&buf, vec!['b', 'l', 'o', 'b']);
            Box::new(Blob::new_from_disk(&mut reader, count, sha, mode)?)
        }
        'c' => {
            let count = check_header_is_valid(&buf, vec!['c', 'o', 'm', 'm', 'i', 't']);
            Box::new(Commit::new_from_disk(&mut reader, count, &sha)?)
        }
        't' => {
            let count = check_header_is_valid(&buf, vec!['t', 'r', 'e', 'e']);
            Box::new(Tree::new_from_disk(&mut reader, count, sha)?)
        }
        _ => exit_with_message("Fatal: unknown type with this sha"),
    };
    Ok(object)
}

pub fn load_commit_from_sha(sha: &Sha) -> Result<Commit, io::Error> {
    let mut buf = vec![];
    let path = sha.to_path();
    let mut reader = init_bufreader(&path)?;
    reader.read_until(0, &mut buf)?;
    let count = check_header_is_valid(&buf, vec!['c', 'o', 'm', 'm', 'i', 't']);
    Ok(Commit::new_from_disk(&mut reader, count, sha)?)
}

fn init_bufreader(path: &Path) -> Result<BufReader<Cursor<Vec<u8>>>, io::Error> {
    let file = File::open(&path)?;
    let data = decompress(file);
    debug!("{}", iterable_to_string(&mut data.iter()));
    let cursor = Cursor::new(data);
    Ok(BufReader::new(cursor))
}

/// Checks if the header for a given file is valid, and returns the integer found in the header
/// Mostly this just error checks and aborts early if something goes wrong
fn check_header_is_valid(buf: &Vec<u8>, expected: Vec<char>) -> usize {
    debug!("Opening file. Checking if it has a valid header");
    let mut split = buf.split(|&b| b as char == ' ');
    let first_half = {
        let this = split.next();
        match this {
            Some(val) => val,
            None => {
                error!("No space found in first chunk of header bytes");
                exit_with_message("Database is corrupted")
            }
        }
    };
    let expected_bytes: Vec<u8> = expected.iter().map(|&c| c as u8).collect();
    if first_half != &expected_bytes {
        error!(
            "Expected file to have {:?} header, had {:?} instead",
            expected, first_half
        );
        exit_with_message("Database is corrupted");
    }
    let second_half = {
        let this = split.next();
        match this {
            Some(val) => val,
            None => {
                error!("No second half to the header size split");
                exit_with_message("Database is corrupted");
            }
        }
    };
    let count_string = iterable_to_string(&mut second_half.iter());
    let count = {
        let this = usize::from_str_radix(&count_string, 10);
        match this {
            Ok(t) => t,
            Err(e) => {
                error!("{}", e);
                exit_with_message("Database is corrupted")
            }
        }
    };
    count
}

#[cfg(test)]
mod tests {
    use flexi_logger::*;

    use super::*;

    #[test]
    fn test_sha() {
        Logger::try_with_str("debug")
            .unwrap()
            .duplicate_to_stdout(Duplicate::All)
            .format(colored_detailed_format)
            .start()
            .expect("Failed to start logger");
        let sha = Sha::new_from_str("6312c14f195ad8be7dfe2fd682c9b6b6bc71c9a3");
        read_from_disk(sha);
    }
}
