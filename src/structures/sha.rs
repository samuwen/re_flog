use std::{
    fmt,
    fs::read_dir,
    path::{Path, PathBuf},
    str::FromStr,
    string::ParseError,
};

use sha1::{Digest, Sha1};

use crate::exit_with_message;

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
        whole.parse().unwrap()
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

impl FromStr for Sha {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 4 {
            let msg = format!(
                "fatal: ambiguous argument '{}': unknown revision or path not in the working tree.",
                s
            );
            exit_with_message(&msg);
        }
        let mut full_str = s.to_string();
        if s.len() < 20 {
            let mut string = s.to_string();
            let first_two_chars: String = string.drain(..2).collect();
            let path_string = format!(".re_flogged/objects/{}", first_two_chars);
            let path = Path::new(&path_string);
            if path.exists() {
                for dir_entry in read_dir(path) {
                    for entry in dir_entry {
                        let entry = entry.unwrap();
                        let file_name = entry.file_name().into_string().unwrap();
                        if file_name.contains(&string) {
                            let full = format!("{}{}", first_two_chars, file_name);
                            full_str = full;
                        }
                    }
                }
            }
        }
        let mut out = [0; 20];
        hex::decode_to_slice(full_str, &mut out).expect("Failed to decode hex");
        Ok(Self { bytes: out })
    }
}
