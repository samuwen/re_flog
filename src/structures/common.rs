use std::{
    fs::File,
    io::{self, BufReader, Read},
};

use flate2::{
    read::{ZlibDecoder, ZlibEncoder},
    Compression,
};
use log::{error, info};

use crate::structures::Sha;

pub fn read_next_u16(reader: &mut BufReader<File>) -> Result<u16, io::Error> {
    let mut u16_buf = [0; 2];
    reader.read_exact(&mut u16_buf)?;
    Ok(u16::from_be_bytes(u16_buf))
}

pub fn read_next_u32(reader: &mut BufReader<File>) -> Result<u32, io::Error> {
    let mut u32_buf: [u8; 4] = [0; 4];
    reader.read_exact(&mut u32_buf)?;
    Ok(u32::from_be_bytes(u32_buf))
}

pub fn read_next_sha(reader: &mut BufReader<File>) -> Result<Sha, io::Error> {
    let mut sha_buf = [0; 20];
    reader.read_exact(&mut sha_buf)?;
    Ok(Sha::new_from_bytes(sha_buf))
}

pub fn read_next_variable_length(
    reader: &mut BufReader<File>,
    length: usize,
) -> Result<Vec<u8>, io::Error> {
    let mut buf = vec![0; length];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn compress<R: Read>(data: R) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(data, Compression::default());
    let mut buffer = vec![];
    if let Err(_e) = encoder.read_to_end(&mut buffer) {
        error!("Failed to compress")
    }
    buffer
}

pub fn decompress<R: Read>(data: R) -> Vec<u8> {
    info!("Decompressing");
    let mut decoder = ZlibDecoder::new(data);
    let mut buffer = vec![];
    if let Err(_e) = decoder.read_to_end(&mut buffer) {
        error!("Failed to decompress")
    }
    buffer
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn sha_from_path() {
        let path = Path::new("80/2992c4220de19a90767f3000a79a31b98d0df7");
        let sha = Sha::new_from_path(path);
        assert_eq!(
            sha.buf(),
            &[
                0x80, 0x29, 0x92, 0xc4, 0x22, 0xd, 0xe1, 0x9a, 0x90, 0x76, 0x7f, 0x30, 0x0, 0xa7,
                0x9a, 0x31, 0xb9, 0x8d, 0xd, 0xf7
            ]
        );
        let path = Path::new(".re_flogged/objects/80/2992c4220de19a90767f3000a79a31b98d0df7");
        let sha = Sha::new_from_path(path);
        assert_eq!(
            sha.buf(),
            &[
                0x80, 0x29, 0x92, 0xc4, 0x22, 0xd, 0xe1, 0x9a, 0x90, 0x76, 0x7f, 0x30, 0x0, 0xa7,
                0x9a, 0x31, 0xb9, 0x8d, 0xd, 0xf7
            ]
        );
    }
}
