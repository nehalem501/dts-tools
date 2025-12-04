use std::{error::Error, path::PathBuf};

use regex::Regex;

use crate::file::{DirEntry, File, FileSystem, FileType};

pub struct TrailerEntries {
    pub metadata: (Box<dyn File>, PathBuf),
    pub audio: (Box<dyn File>, PathBuf),
}

pub struct CdTreeEntries {
    pub reels: Vec<(Box<dyn File>, PathBuf)>,
    pub trailers: Option<TrailerEntries>,
}

pub fn get_if_dts_cd_dir_entry<
    FS: FileSystem<File = F, DirEntry = D>,
    D: DirEntry,
    F: File + 'static,
>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Option<CdTreeEntries>, Box<dyn Error>> {
    match get_if_dts_cd(fs, entries, verbose)? {
        Some(entries) => get_dts_files(fs, &entries, verbose),
        None => Ok(None),
    }
}

fn get_if_dts_cd<FS: FileSystem<DirEntry = D>, D: DirEntry>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Option<Vec<D>>, Box<dyn Error>> {
    let mut dts_exe_found = false;
    let mut dts_dir: Option<Vec<D>> = None;

    if verbose {
        println!("trying to detect if DTS cd:")
    }

    for entry in entries {
        if verbose {
            println!("  {}", entry.file_name())
        }
        if let Ok(t) = entry.file_type() {
            match t {
                FileType::Directory => {
                    if entry.file_name() == "dts" {
                        let dir: Vec<D> = fs.read_dir(entry.path()?)?;
                        dts_dir = Some(dir);
                    }
                }
                FileType::File => {
                    if entry.file_name() == "dts.exe" {
                        // TODO check EXE version
                        dts_exe_found = true;
                    }
                }
            }
        }

        if dts_exe_found && dts_dir.is_some() {
            return Ok(dts_dir);
        }
    }

    return Ok(None);
}

fn get_dts_files<FS: FileSystem<File = F>, D: DirEntry, F: File + 'static>(
    fs: &mut FS,
    entries: &Vec<D>,
    verbose: bool,
) -> Result<Option<CdTreeEntries>, Box<dyn Error>> {
    let re = Regex::new(r"r[1-9][0-3]?t5\.(aud|aue)").unwrap();

    let mut trailers_metadata: Option<&D> = None;
    let mut trailers_audio: Option<&D> = None;
    let mut reels: Vec<&D> = vec![];

    if verbose {
        println!("get_dts_files");
    }

    for entry in entries {
        if verbose {
            print!("  {}: ", entry.file_name());
        }
        match entry.file_type() {
            Ok(t) => {
                if verbose {
                    println!("");
                    print!("    {}: ", entry.file_name());
                }
                if let FileType::File = t {
                    let name = entry.file_name();
                    if name == "r14trlr.txt" || name == "r14.txt" {
                        if verbose {
                            println!("trailers txt");
                        }
                        trailers_metadata = Some(entry);
                    } else if name == "r14t5.aud" || name == "r14t5.aue" {
                        // TODO AUE?
                        if verbose {
                            println!("trailers audio");
                        }
                        trailers_audio = Some(entry);
                    } else if re.is_match(&name) {
                        if verbose {
                            println!("reel audio");
                        }
                        reels.push(entry);
                    }
                }
            }
            Err(_) => {
                if verbose {
                    println!("");
                }
            }
        }
    }

    reels.sort_by_key(|e| e.file_name());
    let mut reel_files: Vec<(Box<dyn File>, PathBuf)> = vec![];
    for r in reels {
        let path = r.path()?;
        reel_files.push((Box::new(fs.open_file(path.as_path())?), path));
    }

    let trailers: Option<TrailerEntries> = match trailers_metadata {
        Some(m) => match trailers_audio {
            Some(a) => {
                let metadata_path = m.path()?;
                let audio_path = a.path()?;
                Some(TrailerEntries {
                    metadata: (
                        Box::new(fs.open_file(metadata_path.as_path())?),
                        metadata_path,
                    ),
                    audio: (Box::new(fs.open_file(audio_path.as_path())?), audio_path),
                })
            }
            None => None,
        },
        None => None,
    };

    return Ok(Some(CdTreeEntries {
        reels: reel_files,
        trailers: trailers,
    }));
}
