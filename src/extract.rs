use std::{
    error::Error,
    fs::{self, create_dir_all},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    detect::{DirType, get_dir_type},
    file::{DirEntry, File, FileSystem},
    hdr::decode_hdr_from_file,
    metadata::{HdrFileMetadata, SndFileMetadata, TrailersMetadata, TrailersMetadataTxtEntry},
    osfile::OsFileSystem,
    snd::{decode_snd_header_from_file, encode_header, get_generic_trailers_header},
    trailers::encode_trailers_to_txt_file,
};

pub enum Feature {
    Id(FeatureId),
    Name(FeatureName),
}

pub struct FeatureName {
    pub name: String,
}

pub struct FeatureId {
    pub id: u16,
}

pub enum Trailers {
    Ids(TrailerIds),
    Names(TrailerNames),
}

pub struct TrailerNames {
    pub names: Vec<String>,
}

pub struct TrailerIds {
    pub ids: Vec<u16>,
}

struct Entry {
    hdr: Option<Box<dyn File>>,
    snd: Box<dyn File>,
}

struct EntryWithMetadata {
    hdr: Option<HdrEntryWithMetadata>,
    snd: SndEntryWithMetadata,
}

struct HdrEntryWithMetadata {
    path: PathBuf,
    metadata: HdrFileMetadata,
}

struct SndEntryWithMetadata {
    path: PathBuf,
    metadata: SndFileMetadata,
}

/*struct TrailerEntry {
    hdr: Box<dyn File>,
    snd: Box<dyn File>,
}*/

struct Files {
    entries: Vec<EntryWithMetadata>,
}

pub fn extract_files(
    input: PathBuf,
    output: PathBuf,
    feature: Option<Feature>,
    trailers: Option<Trailers>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let mut os_fs = OsFileSystem;
    if os_fs.is_dir(&input) {
        return extract_from_dir(&mut os_fs, &input, &output, &feature, &trailers, verbose);
    } else if os_fs.is_file(&input) {
        //return print_file_info(Box::new(os_fs.open_file(path)?), path);
    }
    //unreachable!();
    Ok(())
}

fn extract_from_dir<FS: FileSystem, P: AsRef<Path>>(
    fs: &mut FS,
    input: &P,
    output: &P,
    feature: &Option<Feature>,
    trailers: &Option<Trailers>,
    verbose: bool,
) -> Result<(), Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    let entries = fs.read_dir(input)?;
    match get_dir_type(fs, &entries, verbose)? {
        DirType::DiscTree(_disc) => todo!(), // print_disc_dir_info(disc),
        DirType::Regular => extract_from_regular_dir(fs, output, &entries, feature, trailers),
    }
}

fn extract_from_regular_dir<FS: FileSystem, D: DirEntry, P: AsRef<Path>>(
    fs: &mut FS,
    output: &P,
    entries: &Vec<D>,
    feature: &Option<Feature>,
    trailers: &Option<Trailers>,
) -> Result<(), Box<dyn Error>>
where
    <FS as FileSystem>::File: 'static,
{
    let entries: Vec<EntryWithMetadata> = entries
        .iter()
        .map(|e| {
            let path = e.path().ok()?;
            let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
            if extension == "snd" { Some(path) } else { None }
        })
        .flatten()
        .map(|p| {
            let mut snd_file: Box<dyn File> = Box::new(fs.open_file(&p).ok()?);
            let snd_metadata = decode_snd_header_from_file(snd_file.as_mut(), &p).ok()?;
            let snd = SndEntryWithMetadata {
                //file: snd_file,
                path: p.clone(),
                metadata: snd_metadata,
            };
            let hdr = match get_hdr_from_snd(entries, &p.as_path()) {
                Some(f) => {
                    let mut hdr_file: Box<dyn File> = Box::new(fs.open_file(&f).ok()?);
                    let hdr_metadata = decode_hdr_from_file(hdr_file.as_mut(), &f).ok()?;
                    Some(HdrEntryWithMetadata {
                        //file: hdr_file,
                        path: f,
                        metadata: hdr_metadata,
                    })
                }
                None => None,
            };
            Some(EntryWithMetadata { hdr, snd })
        })
        .flatten()
        .collect();

    let mut files = Files { entries };

    match feature {
        Some(Feature::Name(f)) => {
            let entries = files.find_entries_by_title(&f.name);
            files.convert_to_feature_files(entries, output)?;
        }
        Some(Feature::Id(f)) => {
            let entries = files.find_entries_by_id(f.id);
            files.convert_to_feature_files(entries, output)?;
        }
        None => (),
    };

    match trailers {
        Some(Trailers::Names(t)) => {
            let mut entries: Vec<usize> = vec![];
            for name in &t.names {
                match files.find_entries_by_title(&name).first() {
                    Some(i) => {
                        let e = &files.entries[*i];
                        if e.snd.metadata.reel != 14 {
                            println!("Error: this is not a trailer: {}", name);
                            todo!()
                        }
                        if e.snd.metadata.encrypted {
                            println!("Error: this is not a trailer: {}", name);
                            todo!()
                        }
                        entries.push(*i)
                    }
                    None => {
                        println!("Error: could not find: {}", name);
                        todo!()
                    }
                }
            }
            files.convert_to_trailer_file(entries, output)?;
        }
        Some(Trailers::Ids(t)) => {
            let mut entries: Vec<usize> = vec![];
            for id in &t.ids {
                match files.find_entries_by_id(*id).first() {
                    Some(i) => {
                        let e = &files.entries[*i];
                        if e.snd.metadata.reel != 14 {
                            println!("Error: this is not a trailer: {}", id);
                            todo!()
                        }
                        if e.snd.metadata.encrypted {
                            println!("Error: this is not a trailer: {}", id);
                            todo!()
                        }
                        entries.push(*i)
                    }
                    None => {
                        println!("Error: could not find: {}", id);
                        todo!()
                    }
                }
            }
            files.convert_to_trailer_file(entries, output)?;
        }
        None => (),
    };

    println!("Done");

    Ok(())
}

impl Files {
    fn find_entries_by_title(&self, name: &str) -> Vec<usize> {
        let mut found = vec![];
        for (i, e) in self.entries.iter().enumerate() {
            if e.snd.metadata.title == name {
                found.push(i);
                println!("Found: {:?} with title {}", e.snd.path, name);
            }
        }
        found
    }

    fn find_entries_by_id(&self, id: u16) -> Vec<usize> {
        let mut found = vec![];
        for (i, e) in self.entries.iter().enumerate() {
            if e.snd.metadata.id == id {
                found.push(i);
                println!("Found: {:?} with id {}", e.snd.path, id);
            }
        }
        found
    }

    fn convert_to_trailer_file<P: AsRef<Path>>(
        &mut self,
        entries: Vec<usize>,
        output: P,
    ) -> Result<(), Box<dyn Error>> {
        if entries.len() == 0 {
            todo!();
            //return Ok(())
        }

        create_dir_all(output.as_ref())?;

        let snd_path = output.as_ref().join("r14t5.aud");
        let txt_path = output.as_ref().join("r14trlr.txt");
        let mut snd_file = fs::File::create(&snd_path)?;
        let mut txt_file = fs::File::create(&txt_path)?;

        let mut trailers_metadata = TrailersMetadata { entries: vec![] };
        let mut offset: usize = 92;

        snd_file.write_all(&encode_header(&get_generic_trailers_header()))?;

        for i in entries {
            let e = &mut self.entries[i];
            let mut data = vec![];
            //e.snd.file.read_to_end(&mut data)?;
            let mut file = fs::File::open(&e.snd.path)?;
            file.read_to_end(&mut data)?;

            let md5 = md5::compute(&data);
            println!(
                "Path: {:?}, md5: {:x}, title: {}, id: {}",
                e.snd.path, md5, e.snd.metadata.title, e.snd.metadata.id
            );

            let data = &data[92..];
            let len = data.len();

            snd_file.write_all(data)?;

            let end = len / 3675;

            let metadata = TrailersMetadataTxtEntry {
                title: e.snd.metadata.title.clone(),
                id: e.snd.metadata.id,
                start: 0,
                end,
                offset,
            };
            trailers_metadata.entries.push(metadata);
            offset += len;
        }

        println!("Created {:?}", &snd_path);

        encode_trailers_to_txt_file(&mut txt_file, &trailers_metadata)?;

        println!("Created {:?}", &txt_path);

        Ok(())
    }

    fn convert_to_feature_files<P: AsRef<Path>>(
        &mut self,
        entries: Vec<usize>,
        output: P,
    ) -> Result<(), Box<dyn Error>> {
        if entries.len() == 0 {
            todo!();
            //return Ok(())
        }

        create_dir_all(output.as_ref())?;

        let mut reels: Vec<u8> = vec![];
        for i in &entries {
            let e = &self.entries[*i];
            reels.push(e.snd.metadata.reel);
        }

        reels.sort();

        println!(
            "Found {} reels ({})",
            reels.len(),
            reels
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );

        if let Some(last) = reels.last() {
            if reels.len() != *last as usize {
                println!("Warning: Seems like some reels are missing");
            }
        }

        for i in entries {
            let e = &self.entries[i];
            let snd_path_from = &e.snd.path;
            let snd_path_to = output.as_ref().join(format!(
                "r{}t5.{}",
                e.snd.metadata.reel,
                if e.snd.metadata.encrypted {
                    "aue"
                } else {
                    "aud"
                }
            ));

            fs::copy(&snd_path_from, &snd_path_to)?;

            println!("Created {:?}", &snd_path_to);
        }

        Ok(())
    }
}

fn get_hdr_from_snd<D: DirEntry>(entries: &Vec<D>, snd: &Path) -> Option<PathBuf> {
    let snd_stem = snd.file_stem()?.to_string_lossy().to_ascii_lowercase();
    for e in entries {
        match e.path().ok() {
            Some(p) => {
                let stem = p.file_stem();
                let extension = p.extension();
                if let Some(stem) = stem {
                    let stem = stem.to_string_lossy().to_lowercase();
                    if snd_stem == stem {
                        if let Some(extension) = extension {
                            if extension.to_string_lossy().to_lowercase() == "hdr" {
                                return Some(p);
                            }
                        }
                    }
                }
            }
            None => (),
        }
    }
    None
}
