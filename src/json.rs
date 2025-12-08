use std::{error::Error, fs::File, io::Write, path::Path};

use serde::Serialize;

#[derive(Serialize)]
struct MetadataJson {
    data: Vec<EntryJson>,
}

#[derive(Serialize)]
pub enum EntryJson {
    Feature(FeatureEntryJson),
    Trailer(TrailerEntryJson),
}

#[derive(Serialize)]
pub struct FeatureEntryJson {
    pub id: u16,
    pub title: String,
    pub reels: Vec<ReelEntryJson>,
}

#[derive(Serialize)]
pub struct TrailerEntryJson {
    pub id: u16,
    pub title: String,
}

#[derive(Serialize)]
pub struct ReelEntryJson {
    pub number: u8,
}

pub fn save_json<P: AsRef<Path>>(
    path: P,
    entries: Vec<EntryJson>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let data = MetadataJson { data: entries };
    let json_string = serde_json::to_string_pretty(&data)?;
    if verbose {
        println!("JSON output:");
        println!("{}", &json_string);
    }
    let mut file = File::create(&path)?;
    file.write_all(json_string.as_bytes())?;
    Ok(())
}
