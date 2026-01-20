use std::{fmt, path::Path};

use anyhow::{Result, anyhow};

use crate::{
    cd::{CdTreeEntries, get_if_dts_cd_dir_entry},
    ext234::is_ext234_image_file,
    file::{DirEntry, File, FileSystem},
    hdd::is_hdd_img_file,
    hdr::is_hdr_file,
    iso::is_iso_file,
    squash::is_squashfs_file,
};

pub enum DirType {
    Regular,
    DiscTree(CdTreeEntries),
}

#[derive(Debug)]
pub enum FileType {
    Iso,
    Aud,
    Aue,
    Hdr,
    Snd,
    SquashFs,
    HddImg,
    PartitionImg,
}

#[derive(Debug)]
pub enum SimpleFileType {
    Aud,
    Aue,
    Hdr,
    Snd,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SndFileType {
    Aud,
    Aue,
    Snd(SndTrackType),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SndTrackType {
    Aud,
    Aue,
}

impl fmt::Display for SndFileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SndFileType::Aud => write!(f, "AUD"),
            SndFileType::Aue => write!(f, "AUE"),
            SndFileType::Snd(s) => write!(f, "SND ({})", s),
        }
    }
}

impl fmt::Display for SndTrackType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SndTrackType::Aud => write!(f, "AUD"),
            SndTrackType::Aue => write!(f, "AUE"),
        }
    }
}

pub fn get_file_type<P: AsRef<Path>>(
    file: &mut dyn File,
    path: P,
    verbose: bool,
) -> Result<FileType> {
    get_simple_file_type_from_extension(&path, verbose)
        .and_then(|t| Some(simple_file_type_to_file_type(t)))
        .or(get_remaining_file_type_from_extension(&path, verbose))
        .or(try_get_simple_file_type_from_content(file, verbose)
            .and_then(|t| Some(simple_file_type_to_file_type(t))))
        .or(try_get_remaining_file_type_from_content(file, verbose))
        .ok_or(anyhow!(
            "Could not recognize file type for {}",
            path.as_ref().display()
        ))
}

pub fn get_dir_type<FS: FileSystem<File = F, DirEntry = D>, D: DirEntry, F: File + 'static>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<DirType> {
    match get_if_dts_cd_dir_entry(fs, entries, verbose)? {
        Some(cd) => Ok(DirType::DiscTree(cd)),
        None => Ok(DirType::Regular),
    }
}

fn get_simple_file_type_from_extension<P: AsRef<Path>>(
    path: P,
    verbose: bool,
) -> Option<SimpleFileType> {
    let t = match path
        .as_ref()
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "aud" => Some(SimpleFileType::Aud),
        "aue" => Some(SimpleFileType::Aue),
        "hdr" => Some(SimpleFileType::Hdr),
        "snd" => Some(SimpleFileType::Snd),
        _ => None,
    };

    if verbose {
        println!("get_simple_file_type_from_extension: {:?}", t);
    }

    return t;
}

fn get_remaining_file_type_from_extension<P: AsRef<Path>>(
    path: P,
    verbose: bool,
) -> Option<FileType> {
    let t = match path
        .as_ref()
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "iso" => Some(FileType::Iso),
        _ => None,
    };
    if verbose {
        println!("get_remaining_file_type_from_extension, found: {:?}", t);
    }
    return t;
}

fn try_get_simple_file_type_from_content(
    file: &mut dyn File,
    verbose: bool,
) -> Option<SimpleFileType> {
    if is_hdr_file(file) {
        if verbose {
            println!("try_get_simple_file_type_from_content: Header");
        }
        return Some(SimpleFileType::Hdr);
    }

    // TODO AUD/AUE/SND
    if verbose {
        println!("try_get_simple_file_type_from_content: None");
    }

    return None;
}

fn try_get_remaining_file_type_from_content(
    file: &mut dyn File,
    verbose: bool,
) -> Option<FileType> {
    if is_iso_file(file) {
        if verbose {
            println!("try_get_remaining_file_type_from_content: Iso");
        }
        return Some(FileType::Iso);
    }

    if is_squashfs_file(file) {
        if verbose {
            println!("try_get_remaining_file_type_from_content: SquashFS");
        }
        return Some(FileType::SquashFs);
    }

    if is_hdd_img_file(file) {
        if verbose {
            println!("try_get_remaining_file_type_from_content: HDD Image");
        }
        return Some(FileType::HddImg);
    }

    if is_ext234_image_file(file) {
        if verbose {
            println!("try_get_remaining_file_type_from_content: Ext2/3/4 Filesystem");
        }
        return Some(FileType::PartitionImg);
    }

    return None;
}

fn simple_file_type_to_file_type(t: SimpleFileType) -> FileType {
    match t {
        SimpleFileType::Aud => FileType::Aud,
        SimpleFileType::Aue => FileType::Aue,
        SimpleFileType::Hdr => FileType::Hdr,
        SimpleFileType::Snd => FileType::Snd,
    }
}
