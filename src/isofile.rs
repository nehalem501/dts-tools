use std::{
    cell::RefCell,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Component, Components, Path, PathBuf},
    rc::Rc,
};

use anyhow::{Result, anyhow};
use bitflags::bitflags;
use chrono::{DateTime, FixedOffset, TimeDelta, TimeZone};
use static_assertions::assert_eq_size;

use crate::file::{DirEntry, File, FileSystem, FileType};

const ISO_SECTOR_LEN: u64 = 2048;
const ISO_HEADER_START: u64 = ISO_SECTOR_LEN * 16;

const ISO_DIRECTORY_RECORD_LEN: u64 = 33;

assert_eq_size!(IsoHeaderRaw, [u8; ISO_SECTOR_LEN as usize]);
assert_eq_size!(IsoDirectoryRaw, [u8; ISO_DIRECTORY_RECORD_LEN as usize]);

pub struct IsoFileSystem {
    header: IsoHeader,
    root: IsoDirectory,
    file: Rc<RefCell<Box<dyn File>>>,
}

impl IsoFileSystem {
    pub fn from_file(mut file: Box<dyn File>) -> Result<Self> {
        let bytes = file.read_exact_bytes_at(2048, ISO_HEADER_START)?;
        let raw_header: IsoHeaderRaw = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };
        if raw_header.volume_descriptor_type != 0x01 {
            // TODO
        }
        if raw_header.version != 0x01 {
            // TODO
        }
        let header = IsoHeader::from_raw(&raw_header);
        let root = IsoDirectory::from_raw(&raw_header.directory_entry, &[], &PathBuf::from(""));
        Ok(IsoFileSystem {
            header,
            root,
            file: Rc::new(RefCell::new(file)),
        })
    }

    fn get_dir_entry_from_path(
        &mut self,
        current: &IsoDirectory,
        path: &Path,
        components: &mut Components,
    ) -> Result<IsoDirectory> {
        match components.next() {
            Some(Component::Normal(name)) => {
                let children = self.get_children(current);
                match children.iter().find(|&d| d.name() == name) {
                    Some(d) => {
                        if d.is_dir() {
                            self.get_dir_entry_from_path(d, path, components)
                        } else {
                            match components.next() {
                                Some(c) => Err(anyhow!(
                                    "Could not find path ({}) in file {}",
                                    path.join(c.as_os_str()).display(),
                                    /*TODO*/ String::new()
                                )),
                                None => Ok(d.clone()),
                            }
                        }
                    }
                    None => {
                        Err(anyhow!(
                            "Could not find path ({}) in file {}",
                            path.display(),
                            /*TODO*/ String::new()
                        ))
                    }
                }
            }
            Some(Component::RootDir) => self.get_dir_entry_from_path(current, path, components),
            Some(a) => {
                println!("component: {:?}", a);
                todo!()
            } // Error
            None => Ok(current.clone()),
        }
    }

    fn get_children(&mut self, dir: &IsoDirectory) -> Vec<IsoDirectory> {
        let mut children = vec![];
        let start = (dir.lba as u64) * ISO_SECTOR_LEN;
        let len = dir.data_length as usize;
        let bytes = match self.file.borrow_mut().read_exact_bytes_at(len, start) {
            Ok(data) => data,
            Err(_) => return children, // TODO
        };
        let mut current = 0;
        loop {
            let previous = current;
            if current + ISO_DIRECTORY_RECORD_LEN as usize >= len {
                break;
            }
            let pointer = unsafe { bytes.as_ptr().add(current) };
            let raw: IsoDirectoryRaw = unsafe { std::ptr::read(pointer as *const _) };
            if raw.length == 0 {
                break;
            }
            current += ISO_DIRECTORY_RECORD_LEN as usize;
            let name_len = raw.file_identifier_length as usize;
            let record = IsoDirectory::from_raw(
                &raw,
                &bytes[current..(current + name_len)],
                &dir.path_to_entry,
            );
            current = previous + record.length as usize;
            children.push(record);
        }
        children
    }
}

impl FileSystem for IsoFileSystem {
    type File = IsoFile;
    type DirEntry = IsoDirEntry;

    fn is_file<P: AsRef<Path>>(&mut self, path: P) -> bool {
        match self.get_dir_entry_from_path(
            &self.root.clone(),
            path.as_ref(),
            &mut path.as_ref().components(),
        ) {
            Ok(d) => d.is_file(),
            Err(_) => false,
        }
    }

    fn is_dir<P: AsRef<Path>>(&mut self, path: P) -> bool {
        match self.get_dir_entry_from_path(
            &self.root.clone(),
            path.as_ref(),
            &mut path.as_ref().components(),
        ) {
            Ok(d) => d.is_dir(),
            Err(_) => false,
        }
    }

    fn open_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Self::File> {
        let record = self.get_dir_entry_from_path(
            &self.root.clone(),
            path.as_ref(),
            &mut path.as_ref().components(),
        )?;
        let start = (record.lba as u64) * ISO_SECTOR_LEN;
        let len = record.data_length as u64;
        let file = IsoFile {
            start,
            len,
            current: 0,
            file: self.file.clone(),
            path: path.as_ref().to_path_buf(),
        };
        Ok(file)
    }

    fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<Self::DirEntry>> {
        let dir = self.get_dir_entry_from_path(
            &self.root.clone(),
            path.as_ref(),
            &mut path.as_ref().components(),
        )?;
        Ok(self
            .get_children(&dir)
            .into_iter()
            .map(IsoDirEntry::from)
            .collect())
    }
}

pub struct IsoFile {
    start: u64,
    len: u64,
    current: u64,
    file: Rc<RefCell<Box<dyn File>>>,
    path: PathBuf,
}

impl IsoFile {}

impl File for IsoFile {
    fn len(&mut self) -> Result<u64> {
        Ok(self.len)
    }
}

impl Read for IsoFile {
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

impl Seek for IsoFile {
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

pub struct IsoDirEntry {
    dir: IsoDirectory,
}

impl DirEntry for IsoDirEntry {
    fn path(&self) -> Result<PathBuf> {
        /*
        Ok(PathBuf::from(clean_name))*/
        Ok(self.dir.path_to_entry.clone())
    }

    fn file_type(&self) -> Result<FileType> {
        if self.dir.is_dir() {
            Ok(FileType::Directory)
        } else {
            Ok(FileType::File)
        }
    }
}

#[repr(C, packed(1))]
struct IsoHeaderRaw {
    volume_descriptor_type: u8,
    magic: [u8; 5],
    version: u8,
    unused00: u8,
    system_name: [u8; 32],
    label: [u8; 32],
    unused01: [u8; 8],
    volume_space_size: [u8; 8],
    un_used02: [u8; 32],
    volume_set_size: [u8; 4],
    volume_sequence_number: [u8; 4],
    logical_block_size: [u8; 4],
    path_table_size: [u8; 8],
    loc_of_type_l_path_table: [u8; 4],
    loc_of_opti_l_path_table: [u8; 4],
    loc_of_type_m_path_table: [u8; 4],
    loc_of_opti_m_path_table: [u8; 4],
    directory_entry: IsoDirectoryRaw,
    directory_entry_identifier: u8,
    volume_set_id: [u8; 128],
    publisher_id: [u8; 128],
    data_preparer_id: [u8; 128],
    application_id: [u8; 128],
    copyright_file_id: [u8; 37],
    abstract_file_id: [u8; 37],
    bibliographic_file_id: [u8; 37],
    volume_creation_date: [u8; 17],
    volume_modification_date: [u8; 17],
    volume_expiration_date: [u8; 17],
    volume_effective_date: [u8; 17],
    file_structure_version: i8,
    unused03: i8,
    application_used: [u8; 512],
    reserved: [u8; 653],
}

struct IsoHeader {
    volume_descriptor_type: u8,
    system_name: String,
    label: String,
    volume_space_size: u32,
    volume_set_size: u16,
    volume_sequence_number: u16,
    logical_block_size: u16,
    path_table_size: u32,
    loc_of_type_l_path_table: u32,
    loc_of_opti_l_path_table: u32,
    loc_of_type_m_path_table: u32,
    loc_of_opti_m_path_table: u32,
    volume_set_id: String,
    publisher_id: String,
    data_preparer_id: String,
    application_id: String,
    copyright_file_id: String,
    abstract_file_id: String,
    bibliographic_file_id: String,
    volume_creation_date: Option<DateTime<FixedOffset>>,
    volume_modification_date: Option<DateTime<FixedOffset>>,
    volume_expiration_date: Option<DateTime<FixedOffset>>,
    volume_effective_date: Option<DateTime<FixedOffset>>,
}

impl IsoHeader {
    fn from_raw(header: &IsoHeaderRaw) -> Self {
        Self {
            volume_descriptor_type: header.volume_descriptor_type,
            system_name: String::from_utf8_lossy(&header.system_name).to_string(),
            label: String::from_utf8_lossy(&header.label).to_string(),
            volume_space_size: u32::from_le_bytes(
                header.volume_space_size[..4].try_into().unwrap(),
            ),
            volume_set_size: u16::from_le_bytes(header.volume_set_size[..2].try_into().unwrap()),
            volume_sequence_number: u16::from_le_bytes(
                header.volume_sequence_number[..2].try_into().unwrap(),
            ),
            logical_block_size: u16::from_le_bytes(
                header.logical_block_size[..2].try_into().unwrap(),
            ),
            path_table_size: u32::from_le_bytes(header.path_table_size[..4].try_into().unwrap()),
            loc_of_type_l_path_table: u32::from_le_bytes(header.loc_of_type_l_path_table),
            loc_of_opti_l_path_table: u32::from_le_bytes(header.loc_of_opti_l_path_table),
            loc_of_type_m_path_table: u32::from_be_bytes(header.loc_of_type_m_path_table),
            loc_of_opti_m_path_table: u32::from_be_bytes(header.loc_of_opti_m_path_table),
            volume_set_id: String::from_utf8_lossy(&header.volume_set_id).to_string(),
            publisher_id: String::from_utf8_lossy(&header.publisher_id).to_string(),
            data_preparer_id: String::from_utf8_lossy(&header.data_preparer_id).to_string(),
            application_id: String::from_utf8_lossy(&header.application_id).to_string(),
            copyright_file_id: String::from_utf8_lossy(&header.copyright_file_id).to_string(),
            abstract_file_id: String::from_utf8_lossy(&header.abstract_file_id).to_string(),
            bibliographic_file_id: String::from_utf8_lossy(&header.bibliographic_file_id)
                .to_string(),
            volume_creation_date: iso_dec_datetime_to_datetime(header.volume_creation_date),
            volume_modification_date: iso_dec_datetime_to_datetime(header.volume_modification_date),
            volume_expiration_date: iso_dec_datetime_to_datetime(header.volume_expiration_date),
            volume_effective_date: iso_dec_datetime_to_datetime(header.volume_effective_date),
        }
    }
}

fn iso_dec_datetime_to_datetime(bytes: [u8; 17]) -> Option<DateTime<FixedOffset>> {
    // TODO: only 0 means empty field, should return Option
    let year = String::from_utf8_lossy(&bytes[0..4]).parse::<i32>().ok()?;
    let month = String::from_utf8_lossy(&bytes[4..6]).parse::<u32>().ok()?;
    let day = String::from_utf8_lossy(&bytes[6..8]).parse::<u32>().ok()?;
    let hour = String::from_utf8_lossy(&bytes[8..10]).parse::<u32>().ok()?;
    let minute = String::from_utf8_lossy(&bytes[10..12])
        .parse::<u32>()
        .ok()?;
    let second = String::from_utf8_lossy(&bytes[12..14])
        .parse::<u32>()
        .ok()?;
    let milliseconds = String::from_utf8_lossy(&bytes[14..16])
        .parse::<i64>()
        .ok()?
        * 10;
    let signed_zone_offset = bytes[16] as i8;
    let zone_offset: i32 = ((signed_zone_offset as i32) - 48) * 15 * 60;
    let fixed_offset = FixedOffset::east_opt(zone_offset)?;
    let datetime = fixed_offset
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()?;
    Some(datetime + TimeDelta::milliseconds(milliseconds))
}

#[repr(C, packed(1))]
struct IsoDirectoryRaw {
    length: u8,
    xar_length: u8,
    lba: [u8; 8],
    data_length: [u8; 8],
    datetime: [u8; 7],
    flags: u8,
    unit_size: u8,
    interleave_gap_size: u8,
    volume_seq_number: [u8; 4],
    file_identifier_length: u8,
}

#[derive(Clone)]
struct IsoDirectory {
    length: u8,
    xar_length: u8,
    lba: u32,
    data_length: u32,
    datetime: Option<DateTime<FixedOffset>>,
    flags: IsoDirectoryFlags,
    unit_size: u8,
    interleave_gap_size: u8,
    volume_seq_number: u16,
    file_identifier: String,
    name: String,
    path_to_entry: PathBuf,
}

bitflags! {
    #[derive(Clone)]
    pub struct IsoDirectoryFlags: u8 {
        const Hidden                = 0b00000001;
        const Directory             = 0b00000010;
        const AssociatedFile        = 0b00000100;
        const ExtendedFileAttribute = 0b00001000;
        const ExtendedAttribute     = 0b00010000;
        const Reserved1             = 0b00100000;
        const Reserved2             = 0b01000000;
        const MultipleExtentsFile   = 0b10000000;
    }
}

impl IsoDirectory {
    fn from_raw(raw_dir: &IsoDirectoryRaw, file_name: &[u8], parent: &Path) -> Self {
        let file_identifier = String::from_utf8_lossy(file_name).to_string();
        let clean_name = if let Some(found) = file_identifier.rfind(';') {
            file_identifier[..found].to_string()
        } else {
            file_identifier.to_string()
        };
        let path_to_entry = parent.join(&clean_name);
        Self {
            length: raw_dir.length,
            xar_length: raw_dir.xar_length,
            lba: u32::from_le_bytes(raw_dir.lba[..4].try_into().unwrap()),
            data_length: u32::from_le_bytes(raw_dir.data_length[..4].try_into().unwrap()),
            datetime: iso_directory_datetime_to_datetime(raw_dir.datetime),
            flags: IsoDirectoryFlags::from_bits_retain(raw_dir.flags),
            unit_size: raw_dir.unit_size,
            interleave_gap_size: raw_dir.interleave_gap_size,
            volume_seq_number: u16::from_le_bytes(raw_dir.data_length[..2].try_into().unwrap()),
            file_identifier: file_identifier,
            name: clean_name,
            path_to_entry: path_to_entry,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_file(&self) -> bool {
        !self.is_dir()
    }

    fn is_dir(&self) -> bool {
        self.flags.intersects(IsoDirectoryFlags::Directory)
    }
}

impl From<IsoDirectory> for IsoDirEntry {
    fn from(dir_entry: IsoDirectory) -> Self {
        IsoDirEntry { dir: dir_entry }
    }
}

fn iso_directory_datetime_to_datetime(bytes: [u8; 7]) -> Option<DateTime<FixedOffset>> {
    let year = bytes[0] as i32 + 1900;
    let month = bytes[1] as u32;
    let day = bytes[2] as u32;
    let hour = bytes[3] as u32;
    let minute = bytes[4] as u32;
    let second = bytes[5] as u32;
    let signed_zone_offset = bytes[6] as i8;
    let zone_offset: i32 = ((signed_zone_offset as i32) - 48) * 15 * 60;
    let fixed_offset = FixedOffset::east_opt(zone_offset)?;
    let datetime = fixed_offset
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()?;
    Some(datetime)
}
