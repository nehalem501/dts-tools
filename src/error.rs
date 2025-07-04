use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub struct UnknownFileTypeError {
    pub file: String,
}

impl Error for UnknownFileTypeError {}

impl fmt::Display for UnknownFileTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not recognize file type for {}", self.file)
    }
}

#[derive(Debug, Clone)]
pub struct HdrUnexpectedSizeError {
    pub file: String,
    pub size: u64,
}

impl Error for HdrUnexpectedSizeError {}

impl fmt::Display for HdrUnexpectedSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unexpected file size ({}) for HDR file {}",
            self.size, self.file
        )
    }
}

#[derive(Debug, Clone)]
pub struct HdrUnexpectedHeaderError {
    pub file: String,
    pub data: Vec<u8>,
}

impl Error for HdrUnexpectedHeaderError {}

impl fmt::Display for HdrUnexpectedHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unexpected header ({:?}) for HDR file {}",
            self.data, self.file
        )
    }
}

#[derive(Debug, Clone)]
pub struct SndUnexpectedSizeError {
    pub file: String,
    pub size: u64,
}

impl Error for SndUnexpectedSizeError {}

impl fmt::Display for SndUnexpectedSizeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unexpected file size ({}) for SND/AUD/AUE file {}",
            self.size, self.file
        )
    }
}

#[derive(Debug, Clone)]
pub struct NotDtsDiscIsoError {
    pub file: String,
}

impl Error for NotDtsDiscIsoError {}

impl fmt::Display for NotDtsDiscIsoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "This is not a DTS CD ISO file ({})", self.file)
    }
}

#[derive(Debug, Clone)]
pub struct ParseTrailerMetadataTxtError {
    pub file: String,
    pub line: String,
    pub position: u32,
}

impl Error for ParseTrailerMetadataTxtError {}

impl fmt::Display for ParseTrailerMetadataTxtError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Could not parse file ({}), error at line {}: '{}'",
            self.file, self.position, self.line
        )
    }
}

#[derive(Debug, Clone)]
pub struct PathError {
    pub file: String,
    pub path: String,
}

impl Error for PathError {}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Could not find path ({}) in file {}",
            self.path, self.file
        )
    }
}

#[derive(Debug, Clone)]
pub struct NotFilePathError {
    pub file: String,
    pub path: String,
}

impl Error for NotFilePathError {}

impl fmt::Display for NotFilePathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not open file {}, is not a file", self.path)
    }
}

#[derive(Debug, Clone)]
pub struct NotDirPathError {
    pub file: String,
    pub path: String,
}

impl Error for NotDirPathError {}

impl fmt::Display for NotDirPathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Could not read directory {}, is not a directory",
            self.path
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnknownOpticalBackupSoundtrackFormatError {
    pub value: u8,
}

impl Error for UnknownOpticalBackupSoundtrackFormatError {}

impl fmt::Display for UnknownOpticalBackupSoundtrackFormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Unknown optical backup sountrack format: {:#04x}",
            self.value
        )
    }
}
