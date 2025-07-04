use std::{
    error::Error,
    fmt::format,
    fs,
    io::{BufRead, BufReader, Lines, Write},
    os::unix::fs::FileExt,
    path::Path,
};

use crate::{
    error::ParseTrailerMetadataTxtError,
    file::File,
    metadata::{TrailersMetadata, TrailersMetadataTxtEntry},
};

pub fn decode_trailers_from_txt_file(
    file: &mut dyn File,
    path: &Path,
) -> Result<TrailersMetadata, Box<dyn Error>> {
    let lines = read_lines(file);
    let mut entries = vec![];
    for (i, l) in lines.enumerate() {
        let line = l?;
        if line.starts_with(';') {
            continue;
        }
        let entry = line_to_entry(line, i, path)?;
        match entry {
            Some(e) => entries.push(e),
            None => (),
        }
    }
    Ok(TrailersMetadata { entries })
}

pub fn encode_trailers_to_txt_file(
    file: &mut fs::File,
    data: &TrailersMetadata,
) -> Result<(), Box<dyn Error>> {
    let mut buf: Vec<u8> = vec![];
    buf.extend_from_slice(get_header().as_bytes());
    for e in &data.entries {
        buf.extend_from_slice(entry_to_line(e).as_bytes());
    }
    file.write_all(&buf)?;
    Ok(())
}

fn get_header() -> String {
    let line1 = ";NAME           SERIAL  START   END     OFFSET\r\n";
    let line2 = ";----           ------  -----   ---     ------\r\n";
    format!("{}{}", line1, line2)
}

fn entry_to_line(entry: &TrailersMetadataTxtEntry) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\r\n",
        entry.title, entry.id, entry.start, entry.end, entry.offset
    )
}

fn line_to_entry(
    line: String,
    position: usize,
    path: &Path,
) -> Result<Option<TrailersMetadataTxtEntry>, Box<dyn Error>> {
    let tokens: Vec<&str> = line.split_ascii_whitespace().collect();

    if tokens.len() == 5 {
        let title = tokens[0].to_owned();
        let id = tokens[1].parse::<u16>()?;
        let start = tokens[2].parse::<u32>()?;
        let end = tokens[3].parse::<usize>()?;
        let offset = tokens[4].parse::<usize>()?;
        Ok(Some(TrailersMetadataTxtEntry {
            title,
            id,
            start,
            end,
            offset,
        }))
    } else {
        if !line.chars().all(char::is_alphanumeric) {
            Ok(None)
        } else {
            Err(Box::new(ParseTrailerMetadataTxtError {
                file: path.to_string_lossy().into_owned(),
                line,
                position: (position + 2) as u32,
            }))
        }
    }
}

fn read_lines(file: &mut dyn File) -> Lines<BufReader<&mut dyn File>> {
    let buffer = BufReader::new(file);
    buffer.lines()
}
