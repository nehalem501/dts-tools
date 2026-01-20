use std::{
    fs,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::file::{DirEntry, File, FileSystem, FileType};

pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
    type File = OsFile;
    type DirEntry = OsDirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        path.as_ref().is_file()
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        path.as_ref().is_dir()
    }

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File> {
        Ok(OsFile::from(fs::File::open(path)?))
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>> {
        let read_dir = std::fs::read_dir(path)?.filter_map(Result::ok);
        Ok(read_dir.map(OsDirEntry::from).collect())
    }
}

pub struct OsFile {
    file: fs::File,
}

impl File for OsFile {
    fn len(&mut self) -> Result<u64> {
        let position = self.stream_position()?;
        let len = self.seek(std::io::SeekFrom::End(0))?;
        if position != len {
            self.seek(std::io::SeekFrom::Start(position))?;
        }
        Ok(len)
    }
}

impl Read for OsFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for OsFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl From<fs::File> for OsFile {
    fn from(file: fs::File) -> Self {
        OsFile { file }
    }
}

pub struct OsDirEntry {
    dir: fs::DirEntry,
}

impl DirEntry for OsDirEntry {
    fn path(&self) -> Result<PathBuf> {
        Ok(self.dir.path())
    }

    fn file_type(&self) -> Result<FileType> {
        let metadata = self.dir.metadata()?;
        if metadata.is_dir() {
            Ok(FileType::Directory)
        } else {
            Ok(FileType::File)
        }
    }
}

impl From<std::fs::DirEntry> for OsDirEntry {
    fn from(dir_entry: std::fs::DirEntry) -> Self {
        OsDirEntry { dir: dir_entry }
    }
}
