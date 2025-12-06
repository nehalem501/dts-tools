use std::{cell::RefCell, error::Error, rc::Rc};

use crate::{
    ext234::{EXT234_SUPERBLOCK_LEN, check_ext234_magic, get_ext234_label},
    ext234file::Ext234FileSystem,
    file::{DirEntry, File, FileSystem},
    partitionfile::PartitionFileSystem,
};

const MBR_LEN: usize = 512;
const CONTENTS_PARTITION_LABEL: &str = "/contents";

pub fn is_hdd_img_file(file: &mut dyn File) -> bool {
    let buffer = file.read_bytes(MBR_LEN);
    match buffer {
        Ok(data) => return check_mbr_magic(&data),
        Err(_) => return false,
    }
}

pub fn decode_hdd_img_from_file(
    mut file: Box<dyn File>,
    verbose: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    if verbose {
        println!("decode_hdd_img_from_file:")
    }

    let mbr = mbrman::MBR::read_from(&mut file, 512)?;

    if verbose {
        println!("  found {} partitions", mbr.logical_partitions.len());
    }

    let found = mbr.iter().find_map(|(i, p)| {
        if verbose {
            println!(
                "  reading partition {}: LBA: {}, length: {} sectors",
                i, p.starting_lba, p.sectors
            );
        }
        let offset = p.starting_lba as u64 * mbr.sector_size as u64;
        match file.read_exact_bytes_at(EXT234_SUPERBLOCK_LEN, offset) {
            Ok(bytes) => {
                if check_ext234_magic(&bytes) {
                    if verbose {
                        println!("    type: EXT2/3/4");
                    }
                    let maybe_label = get_ext234_label(&bytes).ok();
                    if let Some(label) = maybe_label {
                        if verbose {
                            println!("    label: {}", label);
                        }
                        if label == CONTENTS_PARTITION_LABEL {
                            println!("Found DTS content partition!"); // TODO
                            return Some(p.clone());
                        }
                    }
                } else {
                }
            }
            Err(_) => (), // TODO
        }
        None
    });

    if let Some(partition) = found {
        if verbose {
            println!("reading ext2/3/4 filesystem:");
        }
        let start = partition.starting_lba as u64 * mbr.sector_size as u64;
        let length = partition.sectors as u64 * mbr.sector_size as u64;
        let partition_fs = Rc::new(RefCell::new(PartitionFileSystem::from_file(
            file, start, length,
        )?));
        let partition_file = Rc::new(RefCell::new(partition_fs.borrow().get_file()?));
        let mut fs = Ext234FileSystem::from_partition(partition_file)?;
        let data_dir = fs.read_dir("/data")?;
    }

    Ok(vec![])
}

fn check_mbr_magic(bytes: &[u8]) -> bool {
    bytes[510] == 0x55 && bytes[511] == 0xAA
}
