use crate::exit_with_message;
use crate::structures::common::*;
use crate::structures::git_objects::Blob;
use crate::utils::iterable_to_string;
use derive_getters::Getters;
use log::*;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead, Seek};
use std::io::{BufReader, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{fmt, vec};

use super::{GitObject, Sha};

const VERSION_NUMBER: u32 = 2;

fn first_flags_byte() -> u16 {
    let assume_valid = 0;
    let extended_flag = 0; // false in v2
    let stage_bits = 0; // 2 bits, used during merge?
    let total = assume_valid & extended_flag << 1 & stage_bits << 2;
    total << 0xC // shift to make it the upper 4 bits
}

fn path_name(path: &Path) -> Vec<u8> {
    let mut stack = vec![];
    stack.push(
        path.file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap(),
    );
    let parent = path.parent().unwrap();
    for ancestor in parent.ancestors() {
        let mut dir_entries = {
            let this = ancestor.read_dir();
            match this {
                Ok(t) => t,
                Err(e) => {
                    error!("Couldn't read directory: {:?}", ancestor);
                    panic!("failed to read dir {}", &e);
                }
            }
        };
        let root_dir_opt = dir_entries.find(|entry| {
            let entry = entry.as_ref().unwrap();
            entry.file_name() == ".re_flogged"
        });
        if root_dir_opt.is_some() {
            break;
        }
        stack.push(
            ancestor
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap(),
        );
    }
    stack.reverse();
    let joined = stack.join("/");
    joined.chars().map(|ch| ch as u8).collect()
}

fn get_needed_padding_count(index_len: isize) -> usize {
    let x = index_len % 8;
    if x == 0 {
        return 0;
    }
    (x - 8).abs() as usize
}

#[derive(Debug, Getters, PartialEq)]
pub struct IndexEntry {
    ctime: u32,
    ctime_nsec: u32,
    mtime: u32,
    mtime_nsec: u32,
    dev: u32,
    ino: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u32,
    file_sha: Sha,
    flags: u16,
    file_name: Vec<u8>, // full path
    padding: Vec<u8>,
}

impl IndexEntry {
    /// New index entry from files on disk
    pub fn new<P: AsRef<OsStr>>(file_path: P) -> Result<Self, io::Error> {
        let file_path = Path::new(&file_path);
        let file = File::open(file_path)?;
        let metadata = file.metadata()?;
        let mut mode = metadata.mode();
        if mode == 0o100664 {
            mode = 0o100644; // git only supports 3 modes for files
        }
        let file_bytes: Vec<u8> = file.bytes().map(|b| b.unwrap()).collect();
        let mut blob = Blob::new_from_bytes(file_bytes, mode);
        blob.write_to_disk()?;
        let flags_byte = first_flags_byte();
        let file_name = path_name(file_path);
        let name_length = if file_name.len() < 0xFFF {
            file_name.len()
        } else {
            0xFFF
        } as u16;
        let flags = name_length | flags_byte;
        let total_bytes_before_padding = 62 + file_name.len();
        let padding_count = get_needed_padding_count(total_bytes_before_padding as isize);
        let padding = (0..padding_count).map(|_x| 0).collect();
        Ok(Self {
            ctime: metadata.ctime() as u32,
            ctime_nsec: metadata.ctime_nsec() as u32,
            mtime: metadata.mtime() as u32,
            mtime_nsec: metadata.mtime_nsec() as u32,
            dev: metadata.dev() as u32,
            ino: metadata.ino() as u32,
            mode,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size() as u32,
            flags,
            file_sha: blob.sha().clone(),
            file_name,
            padding,
        })
    }

    /// Constructs index entry object from an existing git/reflogged index file
    pub fn from_index_file(reader: &mut BufReader<File>) -> Result<Self, io::Error> {
        let before = reader.stream_position()?;
        let ctime = read_next_u32(reader)?;
        let ctime_nsec = read_next_u32(reader)?;
        let mtime = read_next_u32(reader)?;
        let mtime_nsec = read_next_u32(reader)?;
        let dev = read_next_u32(reader)?;
        let ino = read_next_u32(reader)?;
        let mode = read_next_u32(reader)?;
        let uid = read_next_u32(reader)?;
        let gid = read_next_u32(reader)?;
        let size = read_next_u32(reader)?;
        let file_sha = read_next_sha(reader)?;
        let flags = read_next_u16(reader)?;
        let name_length = flags & 0xFFF;
        let file_name = read_next_variable_length(reader, name_length as usize)?;
        let after = reader.stream_position()?;
        let total_byte_count = after - before;
        let padding_count = get_needed_padding_count(total_byte_count as isize);
        let padding = read_next_variable_length(reader, padding_count)?;
        Ok(Self {
            ctime,
            ctime_nsec,
            mtime,
            mtime_nsec,
            dev,
            ino,
            mode,
            uid,
            gid,
            size,
            flags,
            file_sha,
            file_name,
            padding,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write(&self.ctime.to_be_bytes())?;
        writer.write(&self.ctime_nsec.to_be_bytes())?;
        writer.write(&self.mtime.to_be_bytes())?;
        writer.write(&self.mtime_nsec.to_be_bytes())?;
        writer.write(&self.dev.to_be_bytes())?;
        writer.write(&self.ino.to_be_bytes())?;
        writer.write(&self.mode.to_be_bytes())?;
        writer.write(&self.uid.to_be_bytes())?;
        writer.write(&self.gid.to_be_bytes())?;
        writer.write(&self.size.to_be_bytes())?;
        writer.write(self.file_sha.buf())?;
        writer.write(&self.flags.to_be_bytes())?;
        writer.write(&self.file_name)?;
        writer.write(&self.padding)?;
        Ok(())
    }

    pub fn get_readable_sha(&self) -> String {
        self.file_sha.to_string()
    }

    pub fn get_readable_file_name(&self) -> String {
        iterable_to_string(&mut self.file_name.iter())
    }
}

impl fmt::Display for IndexEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = iterable_to_string(&mut self.file_name.iter());
        write!(f, "{}: {}", name, self.file_sha)
    }
}

#[derive(Debug, Getters, PartialEq)]
pub struct Header {
    dirc: [u8; 4],
    version_number: u32,
    file_count: u32,
}

impl Header {
    /// Constructs a new header object with the appropriately sized file count
    fn new(file_count: u32) -> Self {
        let dirc = ['D' as u8, 'I' as u8, 'R' as u8, 'C' as u8];
        Self {
            dirc,
            version_number: VERSION_NUMBER,
            file_count,
        }
    }

    /// Validates that an index file has a proper header and returns the number of index entries present
    fn validate<R: Read>(reader: &mut BufReader<R>) -> Result<Self, io::Error> {
        let mut buf = [0; 4];
        reader.read(&mut buf)?;
        let as_chars: Vec<char> = buf.iter().map(|&b| b as char).collect();
        if as_chars != vec!['D', 'I', 'R', 'C'] {
            exit_with_message("Index file corrupt")
        }
        reader.read(&mut buf)?;
        if u32::from_be_bytes(buf) != VERSION_NUMBER {
            exit_with_message("Index file corrupt")
        }
        reader.read(&mut buf)?;
        let file_count = u32::from_be_bytes(buf);
        debug!("Header is valid");
        debug!("Files in index: {}", file_count);
        Ok(Self::new(file_count))
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write(&self.dirc)?;
        writer.write(&self.version_number.to_be_bytes())?;
        writer.write(&self.file_count.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Getters, PartialEq)]
struct TreeExtensionEntry {
    path_component: String,
    entry_count: String,
    subtree_count: String,
    object_name: String,
}

impl TreeExtensionEntry {
    fn new_from_disk<R: BufRead>(
        reader: &mut R,
        bytes_read: &mut usize,
    ) -> Result<Self, io::Error> {
        let path_component = TreeExtensionEntry::read_data(reader, 0, bytes_read)?;
        let entry_count = TreeExtensionEntry::read_data(reader, 32, bytes_read)?;
        let subtree_count = TreeExtensionEntry::read_data(reader, 10, bytes_read)?;
        let object_name = match &entry_count == "-1 " {
            true => String::new(),
            false => TreeExtensionEntry::read_data(reader, 0, bytes_read)?,
        };
        Ok(Self {
            path_component,
            entry_count,
            subtree_count,
            object_name,
        })
    }

    fn read_data<R: BufRead>(
        reader: &mut R,
        byte: u8,
        total: &mut usize,
    ) -> Result<String, io::Error> {
        let mut buf = vec![];
        *total += reader.read_until(byte, &mut buf)?;
        Ok(iterable_to_string(&mut buf.iter()))
    }
}

#[derive(Debug, Getters, PartialEq)]
pub struct IndexFile {
    header: Header,
    index_entries: Vec<IndexEntry>,
}

impl IndexFile {
    /// New index file from files on disk
    pub fn new(file_count: u32) -> Self {
        let idx_file = match IndexFile::from_disk() {
            Ok(f) => f,
            Err(_) => {
                debug!("Creating new index file");
                let header = Header::new(file_count);
                Self {
                    header,
                    index_entries: vec![],
                }
            }
        };
        idx_file
    }

    pub fn add_files(&mut self, path_strings: &Vec<PathBuf>) {
        info!("Adding files to index: {:?}", path_strings);
        let mut index_entries = path_strings
            .iter()
            .map(|s| {
                if !s.exists() {
                    let message =
                        format!("fatal: pathspec '{}' did not match any files", s.display());
                    exit_with_message(&message);
                }
                let path = Path::canonicalize(s).unwrap();
                let ie = IndexEntry::new(path);
                if let Err(e) = ie {
                    error!("{}", e);
                    let path_msg = format!("fatal: Unable to process path {:?}", s);
                    exit_with_message(&path_msg)
                }
                ie.unwrap()
            })
            .collect();
        self.index_entries.append(&mut index_entries);
        self.header.file_count = self.index_entries.len() as u32;
    }

    pub fn remove_files(&mut self, path_strings: &Vec<PathBuf>) {
        info!("Removing files");
        path_strings.iter().for_each(|s| {
            info!("{}", s.display());
            if !s.exists() {
                let entry_to_remove = self.index_entries.iter().position(|entry| {
                    let entry_name_as_string = entry.get_readable_file_name();
                    let p_name = s.display().to_string();
                    debug!("{} == {}", &entry_name_as_string, p_name);
                    entry_name_as_string == p_name
                });
                if let Some(index) = entry_to_remove {
                    info!("Removing entry at index: {}", index);
                    self.index_entries.remove(index);
                    self.header.file_count = self.header.file_count - 1;
                }
            } else {
                info!("s is burdened with a terrible existence");
            }
        });
    }

    pub fn from_disk() -> Result<Self, io::Error> {
        let path_to_file = Path::new(".re_flogged/index");
        let file = File::open(path_to_file)?;
        let mut reader = BufReader::new(file);
        let header = Header::validate(&mut reader)?;
        let mut index_entries = vec![];
        for _i in 0..header.file_count {
            let ie = IndexEntry::from_index_file(&mut reader)?;
            index_entries.push(ie);
        }
        IndexFile::_check_extensions(&mut reader)?;
        Ok(Self {
            header,
            index_entries,
        })
    }

    fn _check_extensions<R: BufRead>(reader: &mut R) -> Result<(), io::Error> {
        let mut buf = [0; 4];
        if let Err(_e) = reader.read_exact(&mut buf) {
            return Ok(()); // no extensions
        }
        // these read properly (the trees)
        // these get added to the index when 'write-tree' is called
        match buf[0] as char {
            'T' => {
                if buf != [84, 82, 69, 69] {
                    exit_with_message("Index is corrupted");
                }
                if let Err(_e) = reader.read_exact(&mut buf) {
                    exit_with_message("Index is corrupted");
                }
                let size = u32::from_be_bytes(buf);
                let mut bytes_read = 0;
                let mut entries = vec![];
                while bytes_read < size as usize {
                    let entry = TreeExtensionEntry::new_from_disk(reader, &mut bytes_read)?;
                    entries.push(entry);
                }
                println!("{:?}", entries);
            }
            'R' => {
                // Resolve undo
            }
            'l' => {
                // link // split index
            }
            _ => (),
        }
        Ok(())
    }

    pub fn write(&self) -> Result<(), io::Error> {
        let mut out = vec![];
        self.header.write(&mut out)?;
        for ie in self.index_entries.iter() {
            ie.write(&mut out)?;
        }
        let sha = Sha::new_hash(&out);
        let checksum = sha.buf();
        out.write(checksum)?;
        let mut file = File::create(".re_flogged/index")?;
        file.write(&mut out)?;
        Ok(())
    }
}

impl fmt::Display for IndexFile {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index_entries
            .iter()
            .for_each(|entry| println!("{}", entry));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_hardcoded_entry() -> IndexEntry {
        IndexEntry {
            ctime: 1645850906,
            ctime_nsec: 704990044,
            mtime: 1645498338,
            mtime_nsec: 535928285,
            dev: 66306,
            ino: 2492403,
            mode: 33188,
            uid: 1000,
            gid: 1000,
            size: 12,
            file_sha: Sha::new_from_bytes([
                0x80, 0x29, 0x92, 0xc4, 0x22, 0xd, 0xe1, 0x9a, 0x90, 0x76, 0x7f, 0x30, 0x0, 0xa7,
                0x9a, 0x31, 0xb9, 0x8d, 0xd, 0xf7,
            ]),
            flags: u16::from_be_bytes([0x0, 0xE]),
            file_name: vec![
                0x62, 0x6f, 0x6f, 0x70, 0x2f, 0x52, 0x45, 0x41, 0x44, 0x4d, 0x45, 0x2e, 0x6d, 0x64,
            ],
            padding: vec![0, 0, 0, 0],
        }
    }

    #[test]
    fn header_deserialize() {
        let fake_file = vec![
            'D' as u8, 'I' as u8, 'R' as u8, 'C' as u8, 0, 0, 0, 2, 0, 0, 0, 1,
        ];
        let mut reader = BufReader::new(fake_file.as_slice());
        assert!(Header::validate(&mut reader).is_ok());
    }

    #[test]
    fn header_serialize() {
        let header = Header::new(1);
        assert_eq!(header.file_count, 1);
        assert_eq!(header.dirc, ['D' as u8, 'I' as u8, 'R' as u8, 'C' as u8]);
        assert_eq!(header.version_number, VERSION_NUMBER);
    }

    #[test]
    fn idx_entry_deserialize() {
        let file = File::open("test_data/index").unwrap();
        let mut reader = BufReader::new(file);
        Header::validate(&mut reader).unwrap();
        let from_entry = IndexEntry::from_index_file(&mut reader).unwrap();
        let hardcoded = get_hardcoded_entry();
        assert_eq!(from_entry, hardcoded);
    }

    #[test]
    fn idx_entry_serialize() {
        let ie = IndexEntry::new("/home/samuwen/Documents/repos/entirely_fake_repo/boop/README.md")
            .unwrap();
        let hardcoded = get_hardcoded_entry();
        assert_eq!(ie, hardcoded);
    }

    #[test]
    fn new_index_file() {
        // validates that our new index file construction matches the index file git generates for the same single file
        let path_strings = vec![PathBuf::from(
            "/home/samuwen/Documents/repos/entirely_fake_repo/boop/README.md",
        )];
        let mut idx_file = IndexFile::new(1);
        idx_file.add_files(&path_strings);
        let from_disk = IndexFile::from_disk().unwrap();
        assert_eq!(idx_file, from_disk);
    }

    #[test]
    fn get_padding_count() {
        let count = get_needed_padding_count(5);
        assert_eq!(count, 3);
        let count = get_needed_padding_count(8);
        assert_eq!(count, 0);
        let count = get_needed_padding_count(1);
        assert_eq!(count, 7);
        let count = get_needed_padding_count(70);
        assert_eq!(count, 2);
    }

    #[test]
    fn tree_ext() {
        let f = IndexFile::from_disk().unwrap();
        assert_eq!(true, false);
    }
}
