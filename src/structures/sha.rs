use std::{
    fmt,
    path::{Path, PathBuf},
};

use sha1::{Digest, Sha1};

#[derive(Clone, PartialEq)]
pub struct Sha {
    bytes: [u8; 20],
}

impl Sha {
    pub fn new_from_bytes(bytes: [u8; 20]) -> Self {
        Self { bytes }
    }

    pub fn new_from_path(path: &Path) -> Self {
        let last_part = path.file_name().unwrap();
        let first_part = path.parent().unwrap().file_name().unwrap();
        let whole = format!(
            "{}{}",
            first_part.to_str().unwrap(),
            last_part.to_str().unwrap()
        );
        Sha::new_from_str(&whole)
    }

    pub fn new_hash(bytes: impl AsRef<[u8]>) -> Self {
        let mut sha = Sha1::new();
        sha.update(bytes);
        let hashed = sha.finalize().into();
        Self { bytes: hashed }
    }

    pub fn empty() -> Self {
        Self { bytes: [0; 20] }
    }

    pub fn new_from_str(s: &str) -> Self {
        let mut out = [0; 20];
        hex::decode_to_slice(s, &mut out).expect("Failed to decode hex");
        Self { bytes: out }
    }

    pub fn to_string(&self) -> String {
        self.bytes.iter().fold(String::new(), |mut acc, cur| {
            acc.push_str(&format!("{:0>2x}", cur));
            acc
        })
    }

    pub fn to_path(&self) -> PathBuf {
        let mut string = self.to_string();
        let first_two_chars: String = string.drain(..2).collect();
        let path_string = format!(".re_flogged/objects/{}", first_two_chars);
        Path::join(Path::new(&path_string), string).to_path_buf()
    }

    pub fn buf(&self) -> &[u8; 20] {
        &self.bytes
    }

    pub fn is_empty(&self) -> bool {
        self.bytes == [0; 20]
    }
}

impl fmt::Display for Sha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for Sha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sha")
            .field("bytes", &self.to_string())
            .finish()
    }
}
