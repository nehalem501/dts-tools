use std::{error::Error, path::PathBuf};

use crate::{
    detect::{FileType, get_file_type},
    file::{self, DirEntry, File, FileSystem},
    squashfsfile::SquashFsFileSystem,
};

const SQUASHFS_MAGIC_LEN: usize = 4;
const SQUASHFS_MAGIC: [u8; SQUASHFS_MAGIC_LEN] = [0x68, 0x73, 0x71, 0x73];

pub fn is_squashfs_file(file: &mut dyn File) -> bool {
    match file.read_bytes(SQUASHFS_MAGIC_LEN) {
        Ok(buffer) => return check_squashfs_magic(&buffer),
        Err(_) => return false,
    }
}

pub fn decode_squashfs_from_file(
    file: Box<dyn File>,
    verbose: bool,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut fs = SquashFsFileSystem::from_file(file)?;
    let root_dir_entries = fs.read_dir("/")?;
    for e in root_dir_entries {
        if e.file_type()? == file::FileType::Directory {
            continue;
        }
        let path = e.path()?;
        let mut file = fs.open_file(&path)?;
        match get_file_type(&mut file, &path, verbose)? {
            FileType::Iso => todo!(),
            FileType::Aud => todo!(),
            FileType::Aue => todo!(),
            FileType::Hdr => todo!(),
            FileType::Snd => todo!(),
            FileType::SquashFs => todo!(),
            FileType::HddImg => todo!(),
            FileType::PartitionImg => todo!(),
        }
    }
    Ok(vec![])
}

fn check_squashfs_magic(bytes: &[u8]) -> bool {
    return bytes.starts_with(&SQUASHFS_MAGIC);
}
