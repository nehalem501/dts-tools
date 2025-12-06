use std::error::Error;
use std::path::Path;
use std::str;

use crate::error::{HdrUnexpectedHeaderError, HdrUnexpectedSizeError};
use crate::file::File;
use crate::metadata::HdrFileMetadata;

const HDR_LEN: u8 = 0xCA;
const HDR_HEADER: [u8; 8] = [0x00, 0x01, 0x00, 0x04, 0x00, 0x44, 0x54, 0x53];

pub fn is_hdr_file(file: &mut dyn File) -> bool {
    if check_hdr_size(file).0 {
        let bytes = match file.read_bytes(HDR_LEN as usize) {
            Ok(b) => b,
            Err(_) => return false,
        };
        if check_hdr_magic(&bytes) {
            return true;
        }
    }
    return false;
}

pub fn decode_hdr_from_file(
    file: &mut dyn File,
    path: &Path,
) -> Result<HdrFileMetadata, Box<dyn Error>> {
    let size_check = check_hdr_size(file);
    if !size_check.0 {
        return Err(Box::new(HdrUnexpectedSizeError {
            size: size_check.1,
            file: path.to_string_lossy().into_owned(),
        }));
    }
    let bytes = file.read_bytes(HDR_LEN as usize)?;
    decode_hdr(&bytes, path)
}

pub fn decode_hdr(bytes: &[u8], path: &Path) -> Result<HdrFileMetadata, Box<dyn Error>> {
    if !check_hdr_magic(bytes) {
        return Err(Box::new(HdrUnexpectedHeaderError {
            data: bytes[..10].to_vec(),
            file: path.to_string_lossy().into_owned(),
        }));
    }
    let title = str::from_utf8(&bytes[9..27 /*18*/])?.trim_matches(char::from(0));
    let studio = str::from_utf8(&bytes[69..79])?.trim_matches(char::from(0));
    let id = u16::from_le_bytes([bytes[79], bytes[80]]);
    let reel = bytes[91];
    Ok(HdrFileMetadata {
        id: id,
        reel: reel,
        title: title.to_string(),
        studio: studio.to_string(),
    })
}

fn check_hdr_size(file: &mut dyn File) -> (bool, u64) {
    match file.len() {
        Ok(len) => (len == HDR_LEN as u64, len),
        Err(_) => (false, 0),
    }
}

pub fn check_hdr_magic(bytes: &[u8]) -> bool {
    return bytes[0] == HDR_LEN && bytes[1..].starts_with(&HDR_HEADER);
}
