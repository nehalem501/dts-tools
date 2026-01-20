use std::str;

use anyhow::Result;

use crate::file::File;

pub const EXT234_SUPERBLOCK_LEN: usize = 512 * 3;

pub fn is_ext234_image_file(file: &mut dyn File) -> bool {
    let buffer = file.read_bytes(EXT234_SUPERBLOCK_LEN);
    match buffer {
        Ok(data) => return check_ext234_magic(&data),
        Err(_) => return false,
    }
}

pub fn check_ext234_magic(bytes: &[u8]) -> bool {
    bytes[1080] == 0x53 && bytes[1081] == 0xEF
}

pub fn get_ext234_label(bytes: &[u8]) -> Result<String> {
    let label = str::from_utf8(&bytes[1144..1160])?.trim_matches(char::from(0));
    Ok(label.to_string())
}
