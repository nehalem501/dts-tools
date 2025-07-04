use std::{error::Error, path::Path, str};

use crate::{
    error::{SndUnexpectedSizeError, UnknownOpticalBackupSoundtrackFormatError},
    file::File,
    metadata::{BackupSoundtrackFormat, SndFileMetadata, SndType},
};

pub const SND_HEADER_LEN: usize = 92;
pub const SND_HEADER_LEN_WITH_ENCRYPTION: usize = SND_HEADER_LEN + 2;

pub fn decode_snd_header_from_file(
    file: &mut dyn File,
    path: &Path,
) -> Result<SndFileMetadata, Box<dyn Error>> {
    let size_check = check_snd_size(file);
    if !size_check.0 {
        return Err(Box::new(SndUnexpectedSizeError {
            size: size_check.1,
            file: path.to_string_lossy().into_owned(),
        }));
    }

    let bytes = file.read_bytes(SND_HEADER_LEN_WITH_ENCRYPTION)?;
    decode_snd_header(file, &bytes)
}

pub fn decode_snd_header(
    _file: &dyn File,
    bytes: &[u8],
) -> Result<SndFileMetadata, Box<dyn Error>> {
    let title = str::from_utf8(&bytes[0..18])?.trim_matches(char::from(0));
    let language = str::from_utf8(&bytes[61..65])?.trim_matches(char::from(0));
    let studio = str::from_utf8(&bytes[68..72])?.trim_matches(char::from(0));
    let optical_backup = get_optical_backup_format(bytes[75])?;
    let id = u16::from_le_bytes([bytes[80], bytes[81]]);
    let tracks = bytes[82];
    let reel = bytes[78];
    let encrypted = bytes[92] == 1;
    // TODO other fields
    Ok(SndFileMetadata {
        snd_type: if reel == 14 {
            SndType::Trailer
        } else {
            SndType::Feature
        },
        id: id,
        reel: reel,
        title: title.to_string(),
        language: language.to_string(),
        studio: Some(studio.to_string()), // TODO
        optical_backup,
        tracks,
        encrypted,
    })
}

pub fn encode_header(data: &SndFileMetadata) -> Vec<u8> {
    let mut buffer = vec![];
    buffer.extend_from_slice(data.title.as_bytes());
    let zeroes: Vec<u8> = std::iter::repeat_n(0, 60 - buffer.len()).collect();
    buffer.extend_from_slice(&zeroes);
    buffer.push(b'*');
    buffer.extend_from_slice(data.language.as_bytes());
    let zeroes: Vec<u8> = std::iter::repeat_n(0, 68 - buffer.len()).collect();
    buffer.extend_from_slice(&zeroes);
    if let Some(studio) = &data.studio {
        buffer.extend_from_slice(studio.as_bytes());
    }
    let zeroes: Vec<u8> = std::iter::repeat_n(0, 75 - buffer.len()).collect();
    buffer.extend_from_slice(&zeroes);
    buffer.push(data.optical_backup as u8);
    buffer.push(0);
    buffer.push(0);
    buffer.push(data.reel);
    buffer.push(0);
    buffer.extend_from_slice(&data.id.to_le_bytes());
    buffer.push(data.tracks);
    buffer.push(0);
    let zeroes: Vec<u8> = std::iter::repeat_n(0, 92 - buffer.len()).collect();
    buffer.extend_from_slice(&zeroes);

    // TODO
    buffer[85] = 6;
    buffer[89] = 0x26;
    buffer[90] = 0xA8;
    buffer[91] = 1;

    buffer
}

pub fn get_generic_trailers_header() -> SndFileMetadata {
    SndFileMetadata {
        snd_type: SndType::Trailer,
        id: 1045,
        reel: 14,
        title: "Trailers Reel 14".to_string(),
        language: "ENG".to_string(),
        studio: Some("none".to_string()),
        optical_backup: BackupSoundtrackFormat::DolbySR,
        tracks: 5,
        encrypted: false,
    }
}

pub fn check_snd_size(file: &mut dyn File) -> (bool, u64) {
    match file.len() {
        Ok(len) => (len >= SND_HEADER_LEN_WITH_ENCRYPTION as u64, len),
        Err(_) => (false, 0),
    }
}

fn get_optical_backup_format(value: u8) -> Result<BackupSoundtrackFormat, Box<dyn Error>> {
    match value {
        0x00 => Ok(BackupSoundtrackFormat::DolbyA),
        0x01 => Ok(BackupSoundtrackFormat::DolbySR),
        0x02 => Ok(BackupSoundtrackFormat::Academy),
        0x80 => Ok(BackupSoundtrackFormat::NonSync),
        0x81 => Ok(BackupSoundtrackFormat::LastReelDolbySR),
        _ => Err(Box::new(UnknownOpticalBackupSoundtrackFormatError {
            value,
        })),
    }
}
