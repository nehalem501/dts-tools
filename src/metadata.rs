use std::fmt;

enum _Type {
    Packed,
    Individual,
}

enum _Revision {
    H1,
    XD,
    XDA,
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
    pub title: String,
    pub studio: String,
}

pub enum SndType {
    Feature,
    Trailer,
}

pub struct SndFileMetadata {
    pub snd_type: SndType,
    pub id: u16,
    pub reel: u8,
    pub title: String,
    pub language: String,
    pub studio: Option<String>,
    pub optical_backup: BackupSoundtrackFormat,
    pub tracks: u8,
    pub encrypted: bool,
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
