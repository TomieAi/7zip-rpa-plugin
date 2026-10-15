use std::fmt;

#[derive(Debug)]
pub enum RpaError {
    Io(std::io::Error),
    Decompress(String),
    Pickle(String),
    InvalidHeader(String),
    UnsupportedVersion(String),
    CorruptedIndex(String),
    FileNotFound(String),
    StreamError(String),
}

impl fmt::Display for RpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpaError::Io(e) => write!(f, "I/O error: {}", e),
            RpaError::Decompress(s) => write!(f, "Decompression error: {}", s),
            RpaError::Pickle(s) => write!(f, "Pickle decode error: {}", s),
            RpaError::InvalidHeader(s) => write!(f, "Invalid RPA header: {}", s),
            RpaError::UnsupportedVersion(s) => write!(f, "Unsupported RPA version: {}", s),
            RpaError::CorruptedIndex(s) => write!(f, "Corrupted index: {}", s),
            RpaError::FileNotFound(s) => write!(f, "File not found: {}", s),
            RpaError::StreamError(s) => write!(f, "Stream error: {}", s),
        }
    }
}

impl std::error::Error for RpaError {}

impl From<std::io::Error> for RpaError {
    fn from(e: std::io::Error) -> Self {
        RpaError::Io(e)
    }
}
