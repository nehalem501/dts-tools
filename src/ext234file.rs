use std::{
    cell::RefCell,
    error::Error,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Result, anyhow};
use ext4_view::{
    DirEntry as Ext4DirEntry, Ext4, Ext4Read, File as Ext4File, PathBuf as Ext4PathBuf,
};

use crate::file::{DirEntry, File, FileSystem, FileType};

type BoxedError = Box<dyn Error + Send + Sync + 'static>;

struct Ext4FileReader {
    file: Rc<RefCell<Box<dyn File>>>,
}

impl Ext4Read for Ext4FileReader {
    fn read(&mut self, start_byte: u64, dst: &mut [u8]) -> Result<(), BoxedError> {
        let mut file = self.file.borrow_mut();
        file.seek(SeekFrom::Start(start_byte))?;
        file.read_exact(dst)?;
        Ok(())
    }
}

pub struct Ext234FileSystem {
    fs: Ext4,
}

impl Ext234FileSystem {
    pub fn from_file(file: Box<dyn File>) -> Result<Self> {
        let file = Ext4FileReader {
            file: Rc::new(RefCell::new(file)),
        };
        let fs = Ext4::load(Box::new(file))?;
        Ok(Ext234FileSystem { fs })
    }

    pub fn from_partition(file: Rc<RefCell<Box<dyn File>>>) -> Result<Self> {
        let file = Ext4FileReader { file };
        let fs = Ext4::load(Box::new(file))?;
        Ok(Ext234FileSystem { fs })
    }
}

impl FileSystem for Ext234FileSystem {
    type File = Ext234File;
    type DirEntry = Ext234DirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let p = Ext4PathBuf::new(path.as_ref().as_os_str().to_str().unwrap()); // TODO
        match self.fs.metadata(&p) {
            Ok(m) => !m.is_dir(),
            Err(_) => false,
        }
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let p = Ext4PathBuf::new(path.as_ref().as_os_str().to_str().unwrap()); // TODO
        match self.fs.metadata(&p) {
            Ok(m) => m.is_dir(),
            Err(_) => false,
        }
    }

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File> {
        let p: Ext4PathBuf = Ext4PathBuf::new(
            path.as_ref()
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("Couldn't convert to ext4 path"))?,
        );
        let f = self.fs.open(&p)?;
        Ok(Ext234File::from(f))
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>> {
        let p: Ext4PathBuf = Ext4PathBuf::new(
            path.as_ref()
                .as_os_str()
                .to_str()
                .ok_or(anyhow!("Couldn't convert to ext4 path"))?,
        );
        let read_dir = self.fs.read_dir(&p)?.filter_map(Result::ok);
        Ok(read_dir
            .filter(|e| e.file_name().as_str().is_ok_and(|n| n != "." && n != ".."))
            .map(Ext234DirEntry::from)
            .collect())
    }
}

pub struct Ext234File {
    file: Ext4File,
}

impl File for Ext234File {
    fn len(&mut self) -> Result<u64> {
        let position = self.file.stream_position()?;
        let len = self.file.seek(std::io::SeekFrom::End(0))?;
        if position != len {
            self.file.seek(std::io::SeekFrom::Start(position))?;
        }
        Ok(len)
    }
}

impl Read for Ext234File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for Ext234File {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl From<Ext4File> for Ext234File {
    fn from(file: Ext4File) -> Self {
        Ext234File { file }
    }
}

pub struct Ext234DirEntry {
    dir: Ext4DirEntry,
}

impl DirEntry for Ext234DirEntry {
    fn path(&self) -> Result<PathBuf> {
        let p = self.dir.path();
        let s = p.to_str()?;
        Ok(PathBuf::from(s))
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

impl From<Ext4DirEntry> for Ext234DirEntry {
    fn from(dir_entry: Ext4DirEntry) -> Self {
        Ext234DirEntry { dir: dir_entry }
    }
}
