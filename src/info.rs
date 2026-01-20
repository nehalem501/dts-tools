use std::{
    error::Error,
    path::{Path, PathBuf},
};

use itertools::Itertools;

use crate::{
    cd::CdTreeEntries,
    detect::{DirType, FileType, SndFileType, get_dir_type, get_file_type},
    file::{DirEntry, File, FileSystem},
    hdd::decode_hdd_img_from_file,
    hdr::decode_hdr_from_file,
    iso::decode_iso_from_file,
    json::{EntryJson, save_json},
    osfile::OsFileSystem,
    snd::decode_snd_header_from_file,
    squash::decode_squashfs_from_file,
    trailers::decode_trailers_from_txt_file,
};

pub fn print_info(
    paths: &[PathBuf],
    output_json: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let json_entries: Vec<EntryJson> = paths
        .iter()
        .map(|path| print_path_info(&path, verbose))
        .process_results(|e| e.flatten().collect())?;

    match output_json {
        Some(p) => save_json(p, json_entries, verbose),
        None => Ok(()),
    }
}

fn print_path_info(path: &Path, verbose: bool) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let mut os_fs = OsFileSystem;

    if os_fs.is_dir(path) {
        return print_dir_info(&mut os_fs, &path, verbose);
    } else if os_fs.is_file(path) {
        return print_file_info(Box::new(os_fs.open_file(path)?), path, verbose);
    }

    unreachable!()
}

fn print_dir_info<FS: FileSystem, P: AsRef<Path>>(
    fs: &mut FS,
    path: &P,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    let mut entries = fs.read_dir(path)?;
    entries.sort_by_key(|e| e.file_name());
    detect_and_print_entries_info(fs, &entries, verbose)
}

fn detect_and_print_entries_info<FS: FileSystem<DirEntry = D>, D: DirEntry>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    match get_dir_type(fs, entries, verbose)? {
        DirType::DiscTree(disc) => print_disc_dir_info(disc, verbose),
        DirType::Regular => {
            return print_regular_dir_info(fs, &entries, verbose);
        }
    }
}

fn print_regular_dir_info<FS: FileSystem, D: DirEntry>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    print_entries_info(fs, entries, verbose)
}

fn print_disc_dir_info(
    disc: CdTreeEntries,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let trailers = match disc.trailers {
        Some(t) => {
            let (mut file, path) = t.metadata;
            Some(decode_trailers_from_txt_file(
                file.as_mut(),
                path.as_path(),
            )?)
        }
        None => None,
    };

    println!(
        "DTS CD containing {} reels{}:",
        disc.reels.len(),
        match &trailers {
            Some(t) => format!(" and {} trailers", t.entries.len()),
            None => String::new(),
        }
    );
    // TODO
    print_files_info(disc.reels, verbose)?;
    match trailers {
        Some(m) => {
            println!("Trailers:");
            for t in m.entries {
                println!("  Id: {}, Title: {}", t.id, t.title);
            }
        }
        None => (),
    }
    Ok(vec![])
}

fn print_entries_info<FS: FileSystem, D: DirEntry>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    entries
        .iter()
        .map(|d| {
            let path = d.path()?;
            println!("{:?}", path);
            let r = print_file_info(Box::new(fs.open_file(path.as_path())?), &path, verbose);
            if r.is_ok() {
                println!();
            }
            return r;
        })
        .process_results(|e| e.flatten().collect())
}

fn print_files_info<P: AsRef<Path>>(
    files: Vec<(Box<dyn File>, P)>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    files
        .into_iter()
        .map(|(f, p)| {
            let r = print_file_info(f, p.as_ref(), verbose);
            if r.is_ok() {
                println!();
            }
            return r;
        })
        .process_results(|e| e.flatten().collect())
}

/*fn print_files_info_without_squashfs<P: AsRef<Path>>(
    files: Vec<(Box<dyn File>, P)>,
) -> Result<(), Box<dyn Error>> {
    files
        .into_iter()
        .map(|(f, p)| {
            let r = print_file_info_without_squashfs(f, p.as_ref());
            if r.is_ok() {
                println!();
            }
            return r;
        })
        .collect()
}*/

fn print_file_info(
    mut file: Box<dyn File>,
    path: &Path,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    return match get_file_type(file.as_mut(), path, verbose)? {
        FileType::Aud => print_snd_header_info(file.as_mut(), path, Some(SndFileType::Aud)),
        FileType::Aue => print_snd_header_info(file.as_mut(), path, Some(SndFileType::Aue)),
        FileType::Hdr => print_hdr_info(file.as_mut(), path),
        FileType::Snd => print_snd_header_info(file.as_mut(), path, None),
        FileType::Iso => print_iso_info(file, path, verbose),
        FileType::SquashFs => print_squashfs_info(file, verbose),
        FileType::HddImg => print_hdd_img_info(file, verbose),
        FileType::PartitionImg => print_partition_img_info(file),
    };
}

/*fn print_file_info_without_squashfs(
    mut file: Box<dyn File>,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    return match get_file_type(file.as_mut(), path)? {
        FileType::Aud => print_snd_header_info(file.as_mut(), path, Some(SndFileType::Aud)),
        FileType::Aue => print_snd_header_info(file.as_mut(), path, Some(SndFileType::Aue)),
        FileType::Hdr => print_hdr_info(file.as_mut(), path),
        FileType::Snd => print_snd_header_info(file.as_mut(), path, None),
        FileType::Iso => print_iso_info(file, path),
        FileType::SquashFs => todo!(),
        FileType::HddImg => print_hdd_img_info(file),
        FileType::PartitionImg => print_partition_img_info(file),
    };
}*/

fn print_iso_info(
    file: Box<dyn File>,
    path: &Path,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let iso = decode_iso_from_file(file, path, verbose)?;
    print_disc_dir_info(iso, verbose)
}

fn print_hdr_info(file: &mut dyn File, path: &Path) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let data = decode_hdr_from_file(file, path)?;
    println!("DTS XD HDR file: {}", path.display());
    println!("  Id: {}", data.id);
    println!("  Title: {}", data.title);
    println!("  Studio: {}", data.studio);
    Ok(vec![data.to_entry()])
}

fn print_snd_header_info(
    file: &mut dyn File,
    path: &Path,
    snd_type: Option<SndFileType>,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let data = decode_snd_header_from_file(file, path)?;
    let real_snd_type = if data.encryption_key.is_some() {
        SndFileType::Aue
    } else {
        SndFileType::Aud
    };
    println!(
        "DTS {} ({}) file: {}",
        real_snd_type,
        data.revision,
        path.display()
    );
    println!("  Id: {}", data.id);
    println!("  Title: {}", data.title);
    println!("  Reel: {}", data.reel);
    if let Some(xd) = &data.xd {
        if let Some(language) = &xd.language {
            println!("  Language: {}", language);
        }
        if let Some(xda) = &xd.xda {
            if let Some(source) = &xda.source {
                println!("  Source: {}", source);
            }
            if let Some(mix) = &xda.mix {
                println!("  Mix: {}", mix);
            }
            if let Some(lfe_level) = &xda.lfe_level {
                println!("  LFE level (dB): {}", lfe_level);
            }
            if let Some(surround_delay) = &xda.surround_delay {
                println!("  Surround delay: {}", surround_delay);
            }
            if let Some(filters) = &xda.filters {
                println!("  Filters: {}", filters);
            }
        }
    }
    if let Some(studio) = data.studio {
        println!("  Studio: {}", studio);
    }
    println!("  Optical Backup: {}", data.optical_backup);
    println!("  Tracks: {}", data.tracks);
    if let Some(start) = &data.start_offset {
        println!("  Start: {}", start);
    }
    if let Some(end) = &data.end_offset {
        println!("  End: {}", end);
    }
    if let Some(key) = data.encryption_key {
        println!("  Encrypted: {}, key={:#04x}", true, key);
    } else {
        println!("  Encrypted: {}", false);
    }
    if let Some(some_snd_type) = snd_type {
        if some_snd_type != real_snd_type {
            println!(
                "  (Warning: file extension different from encryption property, file type: {}, file extension: {})",
                real_snd_type, some_snd_type
            );
        }
    }
    Ok(vec![])
}

fn print_squashfs_info(
    file: Box<dyn File>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let data = decode_squashfs_from_file(file, verbose)?;

    for f in data {
        println!("file: {}", f.display())
    }

    Ok(vec![])
}

fn print_hdd_img_info(
    file: Box<dyn File>,
    verbose: bool,
) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    let fs_option = decode_hdd_img_from_file(file, verbose)?;
    match fs_option {
        Some(mut fs) => {
            let mut entries = fs.read_dir("/data")?;
            entries.sort_by_key(|e| e.file_name());
            print_regular_dir_info(&mut fs, &entries, verbose)
        }
        None => todo!(),
    }
}

fn print_partition_img_info(_file: Box<dyn File>) -> Result<Vec<EntryJson>, Box<dyn Error>> {
    println!("EXT234");
    Ok(vec![])
}
