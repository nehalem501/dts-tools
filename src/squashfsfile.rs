use std::{
    cell::RefCell,
    error::Error,
    io::{ErrorKind, Read, Seek, SeekFrom},
    num::NonZeroUsize,
    path::{Component, Components, Path, PathBuf},
    rc::Rc,
};

use bitflags::bitflags;
use flate2::read::ZlibDecoder;
use lru::LruCache;
use static_assertions::assert_eq_size;

use crate::{
    error::PathError,
    file::{DirEntry, File, FileSystem, FileType},
};

const SQUASHFS_DIR_COUNT: u32 = 256;
const SQUASHFS_NAME_LEN: u16 = 256;
const SQUASHFS_METADATA_LEN: u64 = 8192;

const SQUASHFS_HEADER_LEN: u64 = 96;

const SQUASHFS_COMMON_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_DIR_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_EXTENDED_DIR_INODE_HEADER_LEN: u64 = 24;
const SQUASHFS_FILE_INODE_HEADER_LEN: u64 = 16;
const SQUASHFS_EXTENDED_FILE_INODE_HEADER_LEN: u64 = 40;

const SQUASHFS_DIR_HEADER_LEN: u64 = 12;
const SQUASHFS_DIR_ENTRY_LEN: u64 = 8;

assert_eq_size!(SquashFsHeaderRaw, [u8; SQUASHFS_HEADER_LEN as usize]);

assert_eq_size!(
    SquashFsCommonInodeHeaderRaw,
    [u8; SQUASHFS_COMMON_INODE_HEADER_LEN as usize]
);
assert_eq_size!(
    SquashFsDirInodeHeaderRaw,
    [u8; SQUASHFS_DIR_INODE_HEADER_LEN as usize]
);
assert_eq_size!(
    SquashFsExtDirInodeHeaderRaw,
    [u8; SQUASHFS_EXTENDED_DIR_INODE_HEADER_LEN as usize]
);
assert_eq_size!(
    SquashFsFileInodeHeaderRaw,
    [u8; SQUASHFS_FILE_INODE_HEADER_LEN as usize]
);
assert_eq_size!(
    SquashFsExtFileInodeHeaderRaw,
    [u8; SQUASHFS_EXTENDED_FILE_INODE_HEADER_LEN as usize]
);

assert_eq_size!(SquashFsDirHeaderRaw, [u8; SQUASHFS_DIR_HEADER_LEN as usize]);
assert_eq_size!(SquashFsDirEntryRaw, [u8; SQUASHFS_DIR_ENTRY_LEN as usize]);

pub struct SquashFsFileSystem {
    fs: Rc<RefCell<Box<SquashFsFileSystemInternal>>>,
    //fragments_cache: LruCache<u32, >,
    root: SquashFsDir,
}

impl SquashFsFileSystem {
    pub fn from_file(mut file: Box<dyn File>) -> Result<Self, Box<dyn Error>> {
        let bytes = file.read_exact_bytes_at(SQUASHFS_HEADER_LEN as usize, 0)?;
        let raw_header: SquashFsHeaderRaw = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };
        /*let mut offset = SQUASHFS_HEADER_LEN;
        let flags = SquashFsHeaderFlags::from_bits_retain(u16::from_le_bytes(raw_header.flags));
        if flags.intersects(SquashFsHeaderFlags::CompressorOptionsArePresent) {
            let metadata_header_bytes = file.read_exact_bytes_at(2, SQUASHFS_HEADER_LEN)?;
            let metadata = decode_metadata_header(u16::from_le_bytes(metadata_header_bytes[..2].try_into().unwrap()));
            if metadata.compressed {
                todo!()
            } else {
                offset += metadata.data_size as u64 + 2;
            }
        }*/

        let version_major = u16::from_le_bytes(raw_header.version_major);
        let version_minor = u16::from_le_bytes(raw_header.version_minor);
        if version_major != 4 || version_minor != 0 {
            todo!()
        }

        let bytes_used = u64::from_le_bytes(raw_header.bytes_used);

        let id_count = u16::from_le_bytes(raw_header.id_count);
        let id_table_ref = u64::from_le_bytes(raw_header.id_table);
        if id_count == 0 || id_table_ref > bytes_used {
            todo!()
        }

        let root_inode_ref = u64::from_le_bytes(raw_header.root_inode);

        let inner_fs = Rc::new(RefCell::new(Box::new(SquashFsFileSystemInternal {
            header: SquashFsHeader::from_raw(&raw_header),
            file: Rc::new(RefCell::new(file)),
            blocks_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
        })));

        let mut fs = SquashFsFileSystem {
            fs: inner_fs,
            root: SquashFsDir { entries: vec![] },
        };

        {
            let mut inner_fs = fs.fs.borrow_mut();

            let root_entries = inner_fs.dir_scan(
                to_inode_blk(root_inode_ref),
                to_inode_offset(root_inode_ref),
            )?;

            fs.root = root_entries;
        }

        Ok(fs)
    }

    fn get_dir_entry_from_path(
        &mut self,
        path: &Path,
        components: &mut Components,
    ) -> Result<SquashFsSimpleDirEntry, Box<dyn Error>> {
        match components.next() {
            Some(Component::Normal(name)) => {
                match self
                    .root
                    .entries
                    .iter()
                    .find(|&d| d.name == name.to_string_lossy())
                {
                    Some(d) => {
                        //if d.is_dir() {
                        //    self.get_dir_entry_from_path(d, path, components)
                        //} else {
                        match components.next() {
                            Some(c) => Err(Box::new(PathError {
                                path: path.join(c.as_os_str()).to_string_lossy().to_string(),
                                file: String::new(), // TODO
                            })),
                            None => Ok(d.clone()),
                        }
                        //}
                    }
                    None => {
                        Err(Box::new(PathError {
                            path: path.to_string_lossy().to_string(),
                            file: String::new(), // TODO
                        }))
                    }
                }
            }
            Some(Component::RootDir) => self.get_dir_entry_from_path(path, components),
            Some(a) => {
                println!("component: {:?}", a);
                todo!()
            } // Error
            None => {
                println!("component: None");
                todo!()
                //Ok(current.clone())
            }
        }
    }
}

impl FileSystem for SquashFsFileSystem {
    type File = SquashFsFile;
    type DirEntry = SquashFsDirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        match self.get_dir_entry_from_path(path.as_ref(), &mut path.as_ref().components()) {
            Ok(d) => d.is_file(),
            Err(_) => false,
        }
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        match self.get_dir_entry_from_path(path.as_ref(), &mut path.as_ref().components()) {
            Ok(d) => d.is_dir(),
            Err(_) => false,
        }
    }

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File, Box<dyn Error>> {
        println!("opening: {:?}", path.as_ref());
        let entry = self.get_dir_entry_from_path(path.as_ref(), &mut path.as_ref().components())?;
        let fs = self.fs.borrow_mut();
        let block_size = fs.header.block_size;
        let len = 0; // TODO
        Ok(SquashFsFile {
            len,
            current: 0,
            fs: self.fs.clone(),
        })
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>, Box<dyn Error>> {
        let mut components = path.as_ref().components();
        if let Some(c) = components.next() {
            if c != Component::RootDir {
                // TODO error
                todo!()
            }

            let c = components.next();
            if c != None {
                // TODO error
                todo!()
            }

            let root_path = Path::new("/");

            let r: Vec<SquashFsDirEntry> = self
                .root
                .entries
                .iter()
                .map(|e| {
                    let entry_type = if e.is_dir() {
                        FileType::Directory
                    } else {
                        FileType::File
                    };
                    SquashFsDirEntry {
                        path: root_path.join(&e.name),
                        entry_type,
                        inner: e.clone(),
                    }
                })
                .collect();

            return Ok(r);
        }

        todo!()
    }
}

struct SquashFsFileSystemInternal {
    header: SquashFsHeader,
    file: Rc<RefCell<Box<dyn File>>>,
    blocks_cache: LruCache<u64, SquashFsBlockEntry>,
}

impl SquashFsFileSystemInternal {
    fn dir_scan(&mut self, start: u64, offset: u64) -> Result<SquashFsDir, Box<dyn Error>> {
        let dir = self.open_dir(start, offset)?;
        Ok(dir)
    }

    fn open_dir(&mut self, start: u64, offset: u64) -> Result<SquashFsDir, Box<dyn Error>> {
        let inode = self.read_inode(start, offset)?;

        if inode.file_size() == 3 {
            // TODO empty
        }

        let len = inode.file_size() - 3;
        let mut entries: Vec<SquashFsSimpleDirEntry> = vec![];
        let mut start = self.header.dir_table + inode.block_index();
        let mut offset = inode.block_offset();
        let mut bytes = 0;
        while bytes < len {
            let entry =
                self.read_metadata(start, offset as usize, SQUASHFS_DIR_HEADER_LEN as usize)?;
            start = entry.block;
            offset = entry.offset;
            let raw_header: SquashFsDirHeaderRaw =
                unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
            let dir_header = SquashFsDirHeader::from_raw(&raw_header);

            let mut dir_count = dir_header.count + 1;
            bytes += SQUASHFS_DIR_HEADER_LEN;

            if dir_count > SQUASHFS_DIR_COUNT {
                // TODO error
            }

            while dir_count > 0 {
                dir_count -= 1;
                let entry = self.read_metadata(start, offset, SQUASHFS_DIR_ENTRY_LEN as usize)?;
                start = entry.block;
                offset = entry.offset;
                let raw_entry: SquashFsDirEntryRaw =
                    unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
                bytes += SQUASHFS_DIR_ENTRY_LEN;
                let name_size = u16::from_le_bytes(raw_entry.name_size);
                if name_size > SQUASHFS_NAME_LEN {
                    // TODO error
                }
                let entry = self.read_metadata(start, offset, (name_size + 1) as usize)?;
                start = entry.block;
                offset = entry.offset;
                let dir_entry =
                    SquashFsSimpleDirEntry::from_raw(&dir_header, &raw_entry, &entry.data);
                bytes += name_size as u64 + 1;

                // TODO
                entries.push(dir_entry);
            }
        }

        Ok(SquashFsDir { entries })
    }

    fn read_inode(&mut self, start_block: u64, offset: u64) -> Result<Inode, Box<dyn Error>> {
        let start = self.header.inode_table + start_block;
        let entry = self.read_metadata(
            start,
            offset as usize,
            SQUASHFS_COMMON_INODE_HEADER_LEN as usize,
        )?;
        let raw_header: SquashFsCommonInodeHeaderRaw =
            unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
        let header = SquashFsCommonInodeHeader::from_raw(&raw_header);

        let inode_data = match header.inode_type {
            InodeTypeRaw::BasicDir => {
                let entry = self.read_metadata(
                    entry.block,
                    entry.offset,
                    SQUASHFS_DIR_INODE_HEADER_LEN as usize,
                )?;
                let raw_header: SquashFsDirInodeHeaderRaw =
                    unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
                InodeType::BasicDir(SquashFsDirInodeHeader::from_raw(&raw_header))
            }
            InodeTypeRaw::ExtendedDir => {
                let entry = self.read_metadata(
                    entry.block,
                    entry.offset,
                    SQUASHFS_EXTENDED_DIR_INODE_HEADER_LEN as usize,
                )?;
                let raw_header: SquashFsExtDirInodeHeaderRaw =
                    unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
                InodeType::ExtendedDir(SquashFsExtDirInodeHeader::from_raw(&raw_header))
            }
            InodeTypeRaw::BasicFile => {
                let entry = self.read_metadata(
                    entry.block,
                    entry.offset,
                    SQUASHFS_FILE_INODE_HEADER_LEN as usize,
                )?;
                let raw_header: SquashFsFileInodeHeaderRaw =
                    unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
                InodeType::BasicFile(SquashFsFileInodeHeader::from_raw(&raw_header))
            }
            InodeTypeRaw::ExtendedFile => {
                let entry = self.read_metadata(
                    entry.block,
                    entry.offset,
                    SQUASHFS_EXTENDED_FILE_INODE_HEADER_LEN as usize,
                )?;
                let raw_header: SquashFsExtFileInodeHeaderRaw =
                    unsafe { std::ptr::read(entry.data.as_ptr() as *const _) };
                InodeType::ExtendedFile(SquashFsExtFileInodeHeader::from_raw(&raw_header))
            }
            _ => todo!(),
            /*InodeTypeRaw::BasicSymlink => todo!(),
            InodeTypeRaw::BasicBlockDevice => todo!(),
            InodeTypeRaw::BasicCharDevice => todo!(),
            InodeTypeRaw::BasicNamedPipe => todo!(),
            InodeTypeRaw::BasicSocket => todo!(),
            InodeTypeRaw::ExtendedSymlink => todo!(),
            InodeTypeRaw::ExtendedBlockDevice => todo!(),
            InodeTypeRaw::ExtendedCharDevice => todo!(),
            InodeTypeRaw::ExtendedNamedPipe => todo!(),
            InodeTypeRaw::ExtendedSocket => todo!(),*/
        };

        Ok(Inode {
            inode_number: header.inode_number,
            data: inode_data,
        })
    }

    fn read_metadata(
        &mut self,
        block: u64,
        offset: usize,
        length: usize,
    ) -> Result<SquashFsMetadataEntry, Box<dyn Error>> {
        let mut data: Vec<u8> = vec![];
        let mut block = block;
        let mut offset = offset;
        let mut length = length;
        loop {
            let entry = self.get_metadata(block)?;
            if offset > entry.data.len() {
                // TODO
                todo!()
            }
            if entry.data.len() - offset < length {
                let copy = entry.data.len() - offset;
                data.extend_from_slice(&entry.data[offset..offset + copy]);
                length -= copy;
                block = entry.next;
                offset = 0;
            } else if entry.data.len() - offset == length {
                data.extend_from_slice(&entry.data[offset..offset + length]);
                return Ok(SquashFsMetadataEntry {
                    data,
                    block: entry.next,
                    offset: 0,
                });
            } else {
                data.extend_from_slice(&entry.data[offset..offset + length]);
                return Ok(SquashFsMetadataEntry {
                    data,
                    block,
                    offset: offset + length,
                });
            }
        }
    }

    fn get_metadata(&mut self, start: u64) -> Result<&SquashFsBlockEntry, Box<dyn Error>> {
        if !self.blocks_cache.contains(&start) {
            let entry = self.read_block(start, None)?;
            self.blocks_cache.push(start, entry);
        }

        Ok(self.blocks_cache.get(&start).unwrap())
    }

    fn read_block(
        &mut self,
        start: u64,
        expected: Option<u64>,
    ) -> Result<SquashFsBlockEntry, Box<dyn Error>> {
        let len = if let Some(e) = expected {
            e
        } else {
            SQUASHFS_METADATA_LEN
        };

        if len > SQUASHFS_METADATA_LEN {
            // TODO error
        }

        let mut offset = 2;

        if self.header.flags.intersects(SquashFsHeaderFlags::Unused) {
            offset = 3; // TODO WTF?
        }

        let mut file = self.file.borrow_mut();
        let header = file.read_le_u16_at(start)?;
        let (compressed, size) = decode_block_header(header);

        if size as u64 > len {
            // TODO error
        }

        let next = start + offset + size as u64;

        let buf = file.read_exact_bytes_at(size as usize, start + offset)?;

        if compressed {
            // TODO other compressors
            let mut decompressor = ZlibDecoder::new(&buf[..]);
            let mut out_buf: Vec<u8> = vec![];
            decompressor.read_to_end(&mut out_buf)?;
            Ok(SquashFsBlockEntry {
                data: out_buf,
                next,
            })
        } else {
            Ok(SquashFsBlockEntry { data: buf, next })
        }
    }
}
struct SquashFsBlockEntry {
    data: Vec<u8>,
    next: u64,
}

struct SquashFsMetadataEntry {
    data: Vec<u8>,
    block: u64,
    offset: usize,
}

pub struct SquashFsFile {
    len: u64,
    current: u64,
    fs: Rc<RefCell<Box<SquashFsFileSystemInternal>>>,
}

impl SquashFsFile {}

impl File for SquashFsFile {
    fn len(&mut self) -> Result<u64, Box<dyn Error>> {
        Ok(self.len)
    }
}

impl Read for SquashFsFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl Seek for SquashFsFile {
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

pub struct SquashFsDirEntry {
    path: PathBuf,
    entry_type: FileType,
    inner: SquashFsSimpleDirEntry,
}

impl DirEntry for SquashFsDirEntry {
    fn path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self.path.clone())
    }

    fn file_type(&self) -> Result<FileType, Box<dyn Error>> {
        Ok(self.entry_type.clone())
    }
}

#[repr(C, packed(1))]
struct SquashFsHeaderRaw {
    magic: [u8; 4],
    inode_count: [u8; 4],
    mod_time: [u8; 4],
    block_size: [u8; 4],
    frag_count: [u8; 4],
    compressor: [u8; 2],
    block_log: [u8; 2],
    flags: [u8; 2],
    id_count: [u8; 2],
    version_major: [u8; 2],
    version_minor: [u8; 2],
    root_inode: [u8; 8],
    bytes_used: [u8; 8],
    id_table: [u8; 8],
    xattr_table: [u8; 8],
    inode_table: [u8; 8],
    dir_table: [u8; 8],
    frag_table: [u8; 8],
    export_table: [u8; 8],
}

struct SquashFsHeader {
    flags: SquashFsHeaderFlags,
    block_size: u32,
    inode_table: u64,
    dir_table: u64,
}

impl SquashFsHeader {
    fn from_raw(header: &SquashFsHeaderRaw) -> Self {
        Self {
            flags: SquashFsHeaderFlags::from_bits_retain(u16::from_le_bytes(header.flags)),
            block_size: u32::from_le_bytes(header.block_size),
            inode_table: u64::from_le_bytes(header.inode_table),
            dir_table: u64::from_le_bytes(header.dir_table),
        }
    }
}

#[repr(u16)]
enum Compressor {
    Gzip,
    Lzo,
    Lzma,
    Xz,
    Lz4,
    Zstd,
}

fn decode_compressor(value: u16) -> Compressor {
    match value {
        1 => Compressor::Gzip,
        2 => Compressor::Lzma,
        3 => Compressor::Lzo,
        4 => Compressor::Xz,
        5 => Compressor::Lz4,
        6 => Compressor::Zstd,
        _ => todo!(), // TODO error
    }
}

#[inline]
fn decode_block_header(header: u16) -> (bool, u16) {
    const SQUASHFS_COMPRESSED_BIT: u16 = 0b10000000_00000000;
    let is_compressed = if header & SQUASHFS_COMPRESSED_BIT != 0 {
        false
    } else {
        true
    };
    let len = header & (!SQUASHFS_COMPRESSED_BIT);
    (is_compressed, len)
}

struct SquashFsMetadataHeader {
    data_size: u16,
    compressed: bool,
}

fn decode_metadata_header(header: u16) -> SquashFsMetadataHeader {
    SquashFsMetadataHeader {
        data_size: header & 0x7FFF,
        compressed: !((header & 0x8000) != 0),
    }
}

bitflags! {
    #[derive(Clone)]
    pub struct SquashFsHeaderFlags: u16 {
        const InodesStoredUncompressed    = 0b00000000_00000001;
        const DataBlockStoredUncompressed = 0b00000000_00000010;
        const Unused                      = 0b00000000_00000100;
        const FragmentsStoredUncompressed = 0b00000000_00001000;
        const FragmentsAreNotUsed         = 0b00000000_00010000;
        const FragmentsAreAlwaysGenerated = 0b00000000_00100000;
        const DataHasBeenDeduplicated     = 0b00000000_01000000;
        const NFSExportTableExists        = 0b00000000_10000000;
        const XattrsAreStoredUncompressed = 0b00000001_00000000;
        const NoXattrsInArchive           = 0b00000010_00000000;
        const CompressorOptionsArePresent = 0b00000100_00000000;
    }
}

#[repr(C, packed(1))]
struct SquashFsCommonInodeHeaderRaw {
    inode_type: [u8; 2],
    permissions: [u8; 2],
    uid: [u8; 2],
    gid: [u8; 2],
    mtime: [u8; 4],
    inode_number: [u8; 4],
}

#[derive(Clone)]
#[repr(u16)]
enum InodeTypeRaw {
    BasicDir = 1,
    BasicFile = 2,
    BasicSymlink = 3,
    BasicBlockDevice = 4,
    BasicCharDevice = 5,
    BasicNamedPipe = 6,
    BasicSocket = 7,
    ExtendedDir = 8,
    ExtendedFile = 9,
    ExtendedSymlink = 10,
    ExtendedBlockDevice = 11,
    ExtendedCharDevice = 12,
    ExtendedNamedPipe = 13,
    ExtendedSocket = 14,
}

fn decode_inode_type(value: u16) -> InodeTypeRaw {
    match value {
        1 => InodeTypeRaw::BasicDir,
        2 => InodeTypeRaw::BasicFile,
        3 => InodeTypeRaw::BasicSymlink,
        4 => InodeTypeRaw::BasicBlockDevice,
        5 => InodeTypeRaw::BasicCharDevice,
        6 => InodeTypeRaw::BasicNamedPipe,
        7 => InodeTypeRaw::BasicSocket,
        8 => InodeTypeRaw::ExtendedDir,
        9 => InodeTypeRaw::ExtendedFile,
        10 => InodeTypeRaw::ExtendedSymlink,
        11 => InodeTypeRaw::ExtendedBlockDevice,
        12 => InodeTypeRaw::ExtendedCharDevice,
        13 => InodeTypeRaw::ExtendedNamedPipe,
        14 => InodeTypeRaw::ExtendedSocket,
        _ => todo!(), // TODO error
    }
}

struct SquashFsCommonInodeHeader {
    inode_type: InodeTypeRaw,
    inode_number: u32,
}

impl SquashFsCommonInodeHeader {
    fn from_raw(header: &SquashFsCommonInodeHeaderRaw) -> Self {
        Self {
            inode_type: decode_inode_type(u16::from_le_bytes(header.inode_type)),
            inode_number: u32::from_le_bytes(header.inode_number),
        }
    }
}

#[repr(C, packed(1))]
struct SquashFsDirInodeHeaderRaw {
    block_index: [u8; 4],
    link_count: [u8; 4],
    file_size: [u8; 2],
    block_offset: [u8; 2],
    parent_inode: [u8; 4],
}

#[derive(Clone)]
struct SquashFsDirInodeHeader {
    block_index: u32,
    file_size: u16,
    block_offset: u16,
    parent_inode: u32,
}

impl SquashFsDirInodeHeader {
    fn from_raw(header: &SquashFsDirInodeHeaderRaw) -> Self {
        Self {
            block_index: u32::from_le_bytes(header.block_index),
            file_size: u16::from_le_bytes(header.file_size),
            block_offset: u16::from_le_bytes(header.block_offset),
            parent_inode: u32::from_le_bytes(header.parent_inode),
        }
    }
}

#[repr(C, packed(1))]
struct SquashFsExtDirInodeHeaderRaw {
    link_count: [u8; 4],
    file_size: [u8; 4],
    block_index: [u8; 4],
    parent_inode: [u8; 4],
    index_count: [u8; 2],
    block_offset: [u8; 2],
    xattr_index: [u8; 4],
}

#[derive(Clone)]
struct SquashFsExtDirInodeHeader {
    link_count: u32,
    file_size: u32,
    block_index: u32,
    parent_inode: u32,
    index_count: u16,
    block_offset: u16,
}

impl SquashFsExtDirInodeHeader {
    fn from_raw(header: &SquashFsExtDirInodeHeaderRaw) -> Self {
        Self {
            link_count: u32::from_le_bytes(header.link_count),
            file_size: u32::from_le_bytes(header.file_size),
            block_index: u32::from_le_bytes(header.block_index),
            parent_inode: u32::from_le_bytes(header.parent_inode),
            index_count: u16::from_le_bytes(header.index_count),
            block_offset: u16::from_le_bytes(header.block_offset),
        }
    }
}

#[repr(C, packed(1))]
struct SquashFsFileInodeHeaderRaw {
    blocks_start: [u8; 4],
    frag_index: [u8; 4],
    block_offset: [u8; 4],
    file_size: [u8; 4],
    //block_sizes: [u8; 4],
}

#[derive(Clone)]
struct SquashFsFileInodeHeader {
    blocks_start: u32,
    frag_index: u32,
    block_offset: u32,
    file_size: u32,
}

impl SquashFsFileInodeHeader {
    fn from_raw(header: &SquashFsFileInodeHeaderRaw) -> Self {
        Self {
            blocks_start: u32::from_le_bytes(header.blocks_start),
            frag_index: u32::from_le_bytes(header.frag_index),
            block_offset: u32::from_le_bytes(header.block_offset),
            file_size: u32::from_le_bytes(header.file_size),
        }
    }
}

#[repr(C, packed(1))]
struct SquashFsExtFileInodeHeaderRaw {
    blocks_start: [u8; 8],
    file_size: [u8; 8],
    sparse: [u8; 8],
    link_count: [u8; 4],
    frag_index: [u8; 4],
    block_offset: [u8; 4],
    xattr_index: [u8; 4],
}

#[derive(Clone)]
struct SquashFsExtFileInodeHeader {
    blocks_start: u64,
    file_size: u64,
    link_count: u32,
    frag_index: u32,
    block_offset: u32,
}

impl SquashFsExtFileInodeHeader {
    fn from_raw(header: &SquashFsExtFileInodeHeaderRaw) -> Self {
        Self {
            blocks_start: u64::from_le_bytes(header.blocks_start),
            file_size: u64::from_le_bytes(header.file_size),
            link_count: u32::from_le_bytes(header.link_count),
            frag_index: u32::from_le_bytes(header.frag_index),
            block_offset: u32::from_le_bytes(header.block_offset),
        }
    }
}

#[derive(Clone)]
enum InodeType {
    BasicDir(SquashFsDirInodeHeader),
    BasicFile(SquashFsFileInodeHeader),
    BasicSymlink,
    BasicBlockDevice,
    BasicCharDevice,
    BasicNamedPipe,
    BasicSocket,
    ExtendedDir(SquashFsExtDirInodeHeader),
    ExtendedFile(SquashFsExtFileInodeHeader),
    ExtendedSymlink,
    ExtendedBlockDevice,
    ExtendedCharDevice,
    ExtendedNamedPipe,
    ExtendedSocket,
}

#[derive(Clone)]
struct Inode {
    inode_number: u32,
    data: InodeType,
}

impl Inode {
    fn from_ref(inode_ref: u64, table_start: u64) -> Result<Inode, Box<dyn Error>> {
        let inode_start = to_inode_blk(inode_ref) + table_start;
        let inode_offset = to_inode_offset(inode_ref);
        todo!()
    }

    fn file_size(&self) -> u64 {
        match &self.data {
            InodeType::BasicDir(dir) => dir.file_size as u64,
            InodeType::BasicFile(file) => file.file_size as u64,
            InodeType::BasicSymlink => todo!(),
            InodeType::BasicBlockDevice => todo!(),
            InodeType::BasicCharDevice => todo!(),
            InodeType::BasicNamedPipe => todo!(),
            InodeType::BasicSocket => todo!(),
            InodeType::ExtendedDir(dir) => dir.file_size as u64,
            InodeType::ExtendedFile(file) => file.file_size,
            InodeType::ExtendedSymlink => todo!(),
            InodeType::ExtendedBlockDevice => todo!(),
            InodeType::ExtendedCharDevice => todo!(),
            InodeType::ExtendedNamedPipe => todo!(),
            InodeType::ExtendedSocket => todo!(),
        }
    }

    fn block_index(&self) -> u64 {
        match &self.data {
            InodeType::BasicDir(dir) => dir.block_index as u64,
            InodeType::BasicFile(file) => file.blocks_start as u64,
            InodeType::BasicSymlink => todo!(),
            InodeType::BasicBlockDevice => todo!(),
            InodeType::BasicCharDevice => todo!(),
            InodeType::BasicNamedPipe => todo!(),
            InodeType::BasicSocket => todo!(),
            InodeType::ExtendedDir(dir) => dir.block_index as u64,
            InodeType::ExtendedFile(file) => file.blocks_start,
            InodeType::ExtendedSymlink => todo!(),
            InodeType::ExtendedBlockDevice => todo!(),
            InodeType::ExtendedCharDevice => todo!(),
            InodeType::ExtendedNamedPipe => todo!(),
            InodeType::ExtendedSocket => todo!(),
        }
    }

    fn block_offset(&self) -> usize {
        match &self.data {
            InodeType::BasicDir(dir) => dir.block_offset as usize,
            InodeType::BasicFile(file) => file.block_offset as usize,
            InodeType::BasicSymlink => todo!(),
            InodeType::BasicBlockDevice => todo!(),
            InodeType::BasicCharDevice => todo!(),
            InodeType::BasicNamedPipe => todo!(),
            InodeType::BasicSocket => todo!(),
            InodeType::ExtendedDir(dir) => dir.block_offset as usize,
            InodeType::ExtendedFile(file) => file.block_offset as usize,
            InodeType::ExtendedSymlink => todo!(),
            InodeType::ExtendedBlockDevice => todo!(),
            InodeType::ExtendedCharDevice => todo!(),
            InodeType::ExtendedNamedPipe => todo!(),
            InodeType::ExtendedSocket => todo!(),
        }
    }
}

#[inline]
fn to_inode_blk(v: u64) -> u64 {
    v >> 16
}

#[inline]
fn to_inode_offset(v: u64) -> u64 {
    v & 0xffff
}

#[repr(C, packed(1))]
struct SquashFsDirHeaderRaw {
    count: [u8; 4],
    start: [u8; 4],
    inode_number: [u8; 4],
}

struct SquashFsDirHeader {
    count: u32,
    start: u32,
    inode_number: u32,
}

impl SquashFsDirHeader {
    fn from_raw(header: &SquashFsDirHeaderRaw) -> Self {
        Self {
            count: u32::from_le_bytes(header.count),
            start: u32::from_le_bytes(header.start),
            inode_number: u32::from_le_bytes(header.inode_number),
        }
    }
}

#[repr(C, packed(1))]
struct SquashFsDirEntryRaw {
    offset: [u8; 2],
    inode_offset: [u8; 2],
    inode_type: [u8; 2],
    name_size: [u8; 2],
}

#[derive(Clone)]
struct SquashFsSimpleDirEntry {
    block_index: u64,
    inode_type: InodeTypeRaw,
    inode_number: u32,
    name: String,
}

impl SquashFsSimpleDirEntry {
    fn from_raw(header: &SquashFsDirHeader, entry: &SquashFsDirEntryRaw, raw_name: &[u8]) -> Self {
        let offset = u16::from_le_bytes(entry.offset);
        let inode_number = header.inode_number - offset as u32;
        Self {
            block_index: header.start as u64,
            inode_type: decode_inode_type(u16::from_le_bytes(entry.inode_type)),
            inode_number,
            name: String::from_utf8_lossy(raw_name).to_string(),
        }
    }

    fn is_file(&self) -> bool {
        match self.inode_type {
            InodeTypeRaw::BasicFile | InodeTypeRaw::ExtendedFile => true,
            _ => false,
        }
    }

    fn is_dir(&self) -> bool {
        match self.inode_type {
            InodeTypeRaw::BasicDir | InodeTypeRaw::ExtendedDir => true,
            _ => false,
        }
    }
}

struct SquashFsDir {
    entries: Vec<SquashFsSimpleDirEntry>,
}
