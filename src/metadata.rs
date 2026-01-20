use std::fmt;

use crate::{
    json::{EntryJson, FeatureEntryJson, ReelEntryJson, TrailerEntryJson},
    snd::SND_HEADER_LEN_WITH_ENCRYPTION,
};

enum _Type {
    Packed,
    Individual,
}

pub struct Offset {
    pub frames: u8,
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{:02}:{:02}.{:03}",
            self.hours,
            self.minutes,
            self.seconds,
            (self.frames as u64 * 1000) / 30
        )
    }
}

pub enum Revision {
    H1,
    XD,
    XDA,
}

impl Revision {
    pub fn from_header(bytes: &[u8; SND_HEADER_LEN_WITH_ENCRYPTION]) -> Revision {
        if bytes[60] == b'*' {
            if bytes[18] == b' '
                && bytes[31] == b' '
                && bytes[47] == b' '
                && bytes[50] == b'D'
                && bytes[51] == b' '
                && bytes[55] == b' '
                && bytes[59] == b' '
                && bytes[65] == 0
                && bytes[66] == 0
            {
                Revision::XDA
            } else {
                Revision::XD
            }
        } else {
            Revision::H1
        }
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Revision::H1 => "H1",
                Revision::XD => "XD",
                Revision::XDA => "XDA",
            }
        )
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum BackupSoundtrackFormat {
    DolbyA = 0x00,
    DolbySR = 0x01,
    Academy = 0x02,
    NonSync = 0x80,
    LastReelDolbySR = 0x81,
}

impl fmt::Display for BackupSoundtrackFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackupSoundtrackFormat::DolbyA => write!(f, "Dolby A"),
            BackupSoundtrackFormat::DolbySR => write!(f, "Dolby SR"),
            BackupSoundtrackFormat::Academy => write!(f, "Academy"),
            BackupSoundtrackFormat::NonSync => write!(f, "Non-Sync"),
            BackupSoundtrackFormat::LastReelDolbySR => write!(f, "Last reel - Dolby SR"),
        }
    }
}

struct _Metadata {
    title: String,
    language: Option<String>,
    studio: String,
    reels: u8,
    serial: u16,
    tracks: u8,
    encrypted: bool,
}

pub struct HdrFileMetadata {
    pub id: u16,
    pub reel: u8,
    pub title: String,
    pub studio: String,
}

impl HdrFileMetadata {
    pub fn to_entry(&self) -> EntryJson {
        if self.reel == 14 {
            EntryJson::Trailer(TrailerEntryJson {
                id: self.id,
                title: self.title.clone(),
            })
        } else {
            EntryJson::Feature(FeatureEntryJson {
                id: self.id,
                title: self.title.clone(),
                reels: vec![ReelEntryJson { number: self.reel }],
            })
        }
    }
}

pub enum SndType {
    Feature,
    Trailer,
}

pub struct SndFileMetadata {
    pub revision: Revision,
    pub snd_type: SndType,
    pub id: u16,
    pub reel: u8,
    pub title: String,
    pub studio: Option<String>,
    pub optical_backup: BackupSoundtrackFormat,
    pub tracks: u8,
    pub start_offset: Option<Offset>,
    pub end_offset: Option<Offset>,
    pub encryption_key: Option<u16>,
    pub xd: Option<XDMetadata>,
}

pub struct XDMetadata {
    pub language: Option<String>,
    pub xda: Option<XDAMetadata>,
}

pub struct XDAMetadata {
    pub source: Option<String>,
    pub mix: Option<String>,
    pub lfe_level: Option<String>,
    pub surround_delay: Option<String>,
    pub filters: Option<String>,
}

pub struct DtsCdMetadata {}

pub struct TrailersMetadataTxtEntry {
    pub title: String,
    pub id: u16,
    pub start: u32,
    pub end: usize,
    pub offset: usize,
}
pub struct TrailersMetadata {
    pub entries: Vec<TrailersMetadataTxtEntry>,
}
