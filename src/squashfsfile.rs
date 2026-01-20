use std::{
    error::Error,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use anyhow::Result;
use backhand::FilesystemReader;

use crate::file::{DirEntry, File, FileSystem, FileType};

/*const SQUASHFS_DIR_COUNT: u32 = 256;
const SQUASHFS_NAME_LEN: u16 = 256;
const SQUASHFS_METADATA_LEN: u64 = 8192;

const SQUASHFS_HEADER_LEN: u64 = 96;

const SQUASHFS_COMMON_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_DIR_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_EXTENDED_DIR_INODE_HEADER_LEN: u64 = 24;
const SQUASHFS_FILE_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_EXTENDED_FILE_INODE_HEADER_LEN: u64 = 40;

const SQUASHFS_DIR_HEADER_LEN: u64 = 12;
const SQUASHFS_DIR_ENTRY_LEN: u64 = 8;*/

pub struct SquashFsFileSystem {}

impl SquashFsFileSystem {
    pub fn from_file(file: Box<dyn File>) -> Result<Self> {
        //let reader = BufReader::new(file);
        //FilesystemReader::from_reader(reader)?;
        todo!()
    }
}

impl FileSystem for SquashFsFileSystem {
    type File = SquashFsFile;
    type DirEntry = SquashFsDirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        todo!()
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        todo!()
    }

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File> {
        todo!()
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>> {
        todo!()
    }
}

pub struct SquashFsFile {}

impl SquashFsFile {}

impl File for SquashFsFile {
    fn len(&mut self) -> Result<u64> {
        todo!()
    }
}

impl Read for SquashFsFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl Seek for SquashFsFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        todo!()
    }
}

pub struct SquashFsDirEntry {}

impl DirEntry for SquashFsDirEntry {
    fn path(&self) -> Result<PathBuf> {
        todo!()
    }

    fn file_type(&self) -> Result<FileType> {
        todo!()
    }
}
