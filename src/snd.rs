use std::{path::Path, str};

use anyhow::{Result, anyhow};

use crate::{
    bcd::{bcd_to_decimal, decimal_to_bcd},
    file::File,
    metadata::{
        BackupSoundtrackFormat, Offset, Revision, SndFileMetadata, SndType, XDAMetadata, XDMetadata,
    },
};

pub const SND_HEADER_LEN: usize = 92;
pub const SND_HEADER_LEN_WITH_ENCRYPTION: usize = SND_HEADER_LEN + 3;

pub fn decode_snd_header_from_file(file: &mut dyn File, path: &Path) -> Result<SndFileMetadata> {
    let size_check = check_snd_size(file);
    if !size_check.0 {
        return Err(anyhow!(
            "Unexpected file size ({}) for SND/AUD/AUE file {}",
            size_check.1,
            path.display()
        ));
    }

    let bytes = file.read_bytes(SND_HEADER_LEN_WITH_ENCRYPTION)?;
    let array = &bytes[0..SND_HEADER_LEN_WITH_ENCRYPTION].try_into()?;
    decode_snd_header(file, array)
}

pub fn decode_snd_header(
    _file: &dyn File,
    bytes: &[u8; SND_HEADER_LEN_WITH_ENCRYPTION],
) -> Result<SndFileMetadata> {
    let revision = Revision::from_header(bytes);
    let (title, xd) = match revision {
        Revision::H1 => {
            let title = str::from_utf8(&bytes[0..67])?;
            (title, None)
        }
        Revision::XD => {
            let title = str::from_utf8(&bytes[0..60])?;
            let language = get_language(&bytes[60..65])?;
            let xd = XDMetadata {
                language,
                xda: None,
            };
            (title, Some(xd))
        }
        Revision::XDA => {
            let title = str::from_utf8(&bytes[0..18])?;
            let language = get_language(&bytes[60..65])?;
            let source = get_optional(&bytes[18..31])?;
            let mix = get_optional(&bytes[31..47])?;
            let lfe_level = if bytes[50] == b'D' {
                get_optional(&bytes[47..50])?
            } else {
                None
            };
            let surround_delay = get_optional(&bytes[51..55])?;
            let filters = get_optional(&bytes[55..59])?;
            let xda = XDAMetadata {
                source,
                mix,
                lfe_level,
                surround_delay,
                filters,
            };
            let xd = XDMetadata {
                language,
                xda: Some(xda),
            };
            (title, Some(xd))
        }
    };
    let title = title.trim_matches(char::from(0)).trim().to_string();
    let studio = get_studio(&bytes[68..72])?;
    let optical_backup = get_optical_backup_format(bytes[75])?;
    let id = u16::from_le_bytes([bytes[80], bytes[81]]);
    let tracks = bytes[82];
    let reel = bytes[78];
    let start_offset = get_offset(&bytes[84..88])?;
    let end_offset = get_offset(&bytes[88..92])?;
    let encryption_key = if bytes[92] == 1 {
        Some(u16::from_le_bytes([bytes[93], bytes[94]]))
    } else {
        None
    };
    Ok(SndFileMetadata {
        revision,
        snd_type: if reel == 14 {
            SndType::Trailer
        } else {
            SndType::Feature
        },
        id,
        reel,
        title,
        studio,
        optical_backup,
        tracks,
        encryption_key,
        start_offset,
        end_offset,
        xd,
    })
}

fn get_language(bytes: &[u8]) -> Result<Option<String>> {
    if bytes[0] == b'*' {
        let language = str::from_utf8(&bytes[1..])?.trim_matches(char::from(0));
        if language.is_empty() {
            Ok(None)
        } else {
            Ok(Some(language.to_string()))
        }
    } else {
        Ok(None)
    }
}

fn get_studio(bytes: &[u8]) -> Result<Option<String>> {
    let studio = str::from_utf8(&bytes)?.trim_matches(char::from(0));
    if studio.is_empty() {
        Ok(None)
    } else {
        Ok(Some(studio.to_string()))
    }
}

fn get_optional(bytes: &[u8]) -> Result<Option<String>> {
    if bytes[0] == b' ' {
        let value = str::from_utf8(&bytes[1..])?.trim();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.to_string()))
        }
    } else {
        Ok(None)
    }
}

fn get_offset(bytes: &[u8]) -> Result<Option<Offset>> {
    let frames = bcd_to_decimal(bytes[0])?;
    let raw_seconds = bytes[1];
    let seconds = bcd_to_decimal(if raw_seconds > 0x60 {
        raw_seconds - 0x60
    } else {
        raw_seconds
    })?;
    let raw_minutes = bytes[2];
    let minutes = bcd_to_decimal(if raw_minutes > 0x60 {
        raw_minutes - 0x60
    } else {
        raw_minutes
    })?;
    let hours = bcd_to_decimal(bytes[3])?;
    if frames == 0 && seconds == 0 && minutes == 0 && hours == 0 {
        Ok(None)
    } else {
        Ok(Some(Offset {
            frames,
            seconds,
            minutes,
            hours,
        }))
    }
}

pub fn encode_header(data: &SndFileMetadata) -> Result<Vec<u8>> {
    let mut buffer = vec![];
    if let Some(xd) = &data.xd {
        if let Some(xda) = &xd.xda {
            insert_max(&mut buffer, &data.title.as_bytes(), b' ', 18);
            insert_optional(&mut buffer, &xda.source, b' ', 12);
            insert_optional(&mut buffer, &xda.mix, b' ', 15);
            insert_optional(&mut buffer, &xda.lfe_level, b' ', 2);
            if xda.lfe_level.is_some() {
                buffer.push(b'D');
            } else {
                buffer.push(b' ');
            }
            insert_optional(&mut buffer, &xda.surround_delay, b' ', 3);
            insert_optional(&mut buffer, &xda.filters, b' ', 3);
            buffer.push(b' ');
        } else {
            insert_max(&mut buffer, &data.title.as_bytes(), 0, 60);
        }
        if let Some(lang) = &xd.language {
            buffer.push(b'*');
            insert_max(&mut buffer, &lang.as_bytes(), 0, 4);
        } else {
            let zeroes: Vec<u8> = std::iter::repeat_n(0, 5).collect();
            buffer.extend_from_slice(&zeroes);
        }
        buffer.push(0); // TODO: sometimes other value, investigate why
        buffer.push(0); // TODO: sometimes other value, investigate why
        buffer.push(0);
    } else {
        insert_max(&mut buffer, &data.title.as_bytes(), 0, 67);
    }

    // 68
    insert_optional(&mut buffer, &data.studio, 0, 4);

    // 72
    buffer.push(0);
    buffer.push(0);
    buffer.push(0);

    // 75
    buffer.push(data.optical_backup as u8);
    buffer.push(0);
    buffer.push(0);

    // 78
    buffer.push(data.reel);
    buffer.push(0);

    // 80
    buffer.extend_from_slice(&data.id.to_le_bytes());

    // 82
    buffer.push(data.tracks);
    buffer.push(0);

    // 84
    insert_offset(&mut buffer, &data.start_offset)?;

    // 88
    insert_offset(&mut buffer, &data.end_offset)?;

    // TODO trailer disc
    // buffer[85] = 6;
    // buffer[89] = 0x26;
    // buffer[90] = 0xA8;
    // buffer[91] = 1;

    // TODO encryption

    Ok(buffer)
}

pub fn get_generic_trailers_header() -> SndFileMetadata {
    SndFileMetadata {
        revision: Revision::XD,
        snd_type: SndType::Trailer,
        id: 1045, // TODO
        reel: 14,
        title: "Trailers Reel 14".to_string(),
        studio: Some("none".to_string()),
        optical_backup: BackupSoundtrackFormat::DolbySR,
        tracks: 5,
        encryption_key: None,
        xd: Some(XDMetadata {
            language: Some("ENG".to_string()),
            xda: None,
        }),
        start_offset: None,
        end_offset: None,
    }
}

fn insert_optional(buffer: &mut Vec<u8>, value: &Option<String>, fill: u8, len: usize) {
    if let Some(v) = value {
        buffer.push(b' ');
        insert_max(buffer, &v.as_bytes(), fill, len);
    } else {
        let spaces: Vec<u8> = std::iter::repeat_n(fill, len + 1).collect();
        buffer.extend_from_slice(&spaces);
    }
}

fn insert_max(buffer: &mut Vec<u8>, value: &[u8], fill: u8, max: usize) {
    let len = value.len();
    if len >= max {
        buffer.extend_from_slice(&value[0..max]);
    } else {
        let fill: Vec<u8> = std::iter::repeat_n(fill, max - len).collect();
        buffer.extend_from_slice(&fill);
    }
}

pub fn check_snd_size(file: &mut dyn File) -> (bool, u64) {
    match file.len() {
        Ok(len) => (len >= SND_HEADER_LEN_WITH_ENCRYPTION as u64, len),
        Err(_) => (false, 0),
    }
}

fn insert_offset(buffer: &mut Vec<u8>, offset: &Option<Offset>) -> Result<()> {
    if let Some(o) = offset {
        buffer.push(decimal_to_bcd(o.frames)?);
        buffer.push(decimal_to_bcd(o.seconds)?);
        buffer.push(decimal_to_bcd(o.minutes)?);
        buffer.push(decimal_to_bcd(o.hours)?);
    } else {
        buffer.push(0);
        buffer.push(0);
        buffer.push(0);
        buffer.push(0);
    }

    Ok(())
}

fn get_optical_backup_format(value: u8) -> Result<BackupSoundtrackFormat> {
    match value {
        0x00 => Ok(BackupSoundtrackFormat::DolbyA),
        0x01 => Ok(BackupSoundtrackFormat::DolbySR),
        0x02 => Ok(BackupSoundtrackFormat::Academy),
        0x80 => Ok(BackupSoundtrackFormat::NonSync),
        0x81 => Ok(BackupSoundtrackFormat::LastReelDolbySR),
        _ => Err(anyhow!(
            "Unknown optical backup sountrack format: {:#04x}",
            value
        )),
    }
}
