use std::{
    cell::RefCell,
    error::Error,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::file::{DirEntry, File, FileSystem, FileType};

pub struct PartitionFileSystem {
    file: Rc<RefCell<Box<dyn File>>>,
    start: u64,
    len: u64,
}

impl PartitionFileSystem {
    pub fn from_file(file: Box<dyn File>, start: u64, len: u64) -> Result<Self, Box<dyn Error>> {
        let fs = PartitionFileSystem {
            file: Rc::new(RefCell::new(file)),
            start,
            len,
        };
        Ok(fs)
    }

    pub fn get_file(&self) -> Result<Box<dyn File>, Box<dyn Error>> {
        let file = Box::new(PartitionFile {
            start: self.start,
            len: self.len,
            current: 0,
            file: self.file.clone(),
        });
        Ok(file)
    }
}

impl FileSystem for PartitionFileSystem {
    type File = PartitionFile;
    type DirEntry = PartitionDirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, _path: P) -> bool {
        true
    }

    fn is_dir<P: AsRef<Path>>(&mut self, _path: P) -> bool {
        false
    }

    fn open_file<P: AsRef<Path>>(&mut self, _path: P) -> Result<Self::File, Box<dyn Error>> {
        let file = PartitionFile {
            start: self.start,
            len: self.len,
            current: 0,
            file: self.file.clone(),
        };
        Ok(file)
    }

    fn read_dir<P: AsRef<Path>>(
        &mut self,
        _path: P,
    ) -> Result<Vec<Self::DirEntry>, Box<dyn Error>> {
        Ok(vec![])
    }
}

pub struct PartitionFile {
    start: u64,
    len: u64,
    current: u64,
    file: Rc<RefCell<Box<dyn File>>>,
}

impl File for PartitionFile {
    fn len(&mut self) -> Result<u64, Box<dyn Error>> {
        Ok(self.len)
    }
}

impl Read for PartitionFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut buf_len = buf.len();
        if buf_len as u64 > self.len - self.current {
            buf_len = (self.len - self.current) as usize;
        }
        let bytes = self
            .file
            .borrow_mut()
            .read_buffer_at(&mut buf[0..buf_len], self.start + self.current)?;
        self.current += bytes as u64;
        Ok(bytes)
    }
}

impl Seek for PartitionFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                if offset > self.len {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "trying to seek past end of file",
                    ));
                } else {
                    self.current = offset;
                }
            }
            SeekFrom::End(from_end) => {
                if from_end > 0 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "trying to seek past end of file",
                    ));
                } else if from_end.unsigned_abs() > self.len {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "trying to seek before start of file",
                    ));
                } else {
                    self.current -= from_end.unsigned_abs();
                }
            }
            SeekFrom::Current(new) => {
                let new_current = self.current as i64 + new;
                if new_current < 0 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "trying to seek before start of file",
                    ));
                } else if new_current > self.len as i64 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "trying to seek past end of file",
                    ));
                } else {
                    self.current = new_current as u64;
                }
            }
        }
        Ok(self.current)
    }
}

pub struct PartitionDirEntry {}

impl DirEntry for PartitionDirEntry {
    fn path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(PathBuf::new())
    }

    fn file_type(&self) -> Result<FileType, Box<dyn Error>> {
        Ok(FileType::File)
    }
}
