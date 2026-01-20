use std::path::Path;

use anyhow::{Result, anyhow};

use crate::{
    cd::{CdTreeEntries, get_if_dts_cd_dir_entry},
    file::{DirEntry, File, FileSystem},
    isofile::IsoFileSystem,
};

const ISO_MAGIC: [u8; ISO_MAGIC_LEN] = [0x43, 0x44, 0x30, 0x30, 0x31];
const ISO_MAGIC_LEN: usize = 5;

pub fn is_iso_file(file: &mut dyn File) -> bool {
    let buffer = file.read_exact_bytes_at(ISO_MAGIC_LEN, 0x8001);
    match buffer {
        Ok(data) => return check_iso_magic(&data),
        Err(_) => return false,
    }
}

pub fn decode_iso_from_file(
    file: Box<dyn File>,
    path: &Path,
    verbose: bool,
) -> Result<CdTreeEntries> {
    let mut fs = IsoFileSystem::from_file(file)?;
    let root_dir_entries = fs.read_dir("/")?;
    if verbose {
        println!("decoding iso, found:");
        for e in &root_dir_entries {
            println!("  {}", e.file_name());
        }
    }
    let disc = get_if_dts_cd_dir_entry(&mut fs, &root_dir_entries, verbose)?;
    match disc {
        Some(d) => Ok(d),
        None => Err(anyhow!(
            "This is not a DTS CD ISO file ({})",
            path.display()
        )),
    }
}

fn check_iso_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&ISO_MAGIC)
}
