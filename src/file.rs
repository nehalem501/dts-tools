use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use anyhow::Result;

#[derive(Clone, PartialEq, Eq)]
pub enum FileType {
    Directory,
    File,
}

pub trait File: Read /*+ Write*/ + Seek /*+ Send + Sync*/ {
    fn len(&mut self) -> Result<u64>;

    fn read_bytes(&mut self, bytes: usize) -> Result<Vec<u8>> {
        self.read_exact_bytes_at(bytes, 0)
    }

    fn read_buffer_at(&mut self, buffer: &mut [u8], at: u64) -> std::io::Result<usize> {
        let position = self.stream_position()?;
        self.seek(SeekFrom::Start(at))?;
        let len = self.read(buffer)?;
        self.seek(SeekFrom::Start(position))?;
        Ok(len)
    }

    fn read_exact_bytes_at(&mut self, bytes: usize, at: u64) -> Result<Vec<u8>> {
        let position = self.stream_position()?;
        self.seek(SeekFrom::Start(at))?;
        let mut buffer = vec![0; bytes];
        self.read_exact(&mut buffer)?;
        self.seek(SeekFrom::Start(position))?;
        Ok(buffer)
    }

    fn read_le_u16_at(&mut self, at: u64) -> Result<u16> {
        let position = self.stream_position()?;
        self.seek(SeekFrom::Start(at))?;
        let mut buffer = vec![0; 2];
        self.read_exact(&mut buffer)?;
        self.seek(SeekFrom::Start(position))?;
        let value = u16::from_le_bytes(buffer.try_into().unwrap());
        Ok(value)
    }
}

pub trait DirEntry {
    fn path(&self) -> Result<PathBuf>;
    fn file_type(&self) -> Result<FileType>;
    fn file_name(&self) -> String {
        match self.path() {
            Ok(p) => match p.as_path().file_name() {
                Some(n) => n.to_string_lossy().to_ascii_lowercase(),
                None => String::new(),
            },
            Err(_) => String::new(),
        }
    }
}

pub trait FileSystem {
    type File: File;
    type DirEntry: DirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool;
    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool;

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File>;
    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>>;
}
