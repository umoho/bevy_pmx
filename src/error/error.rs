use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum PmxError {
    Io(io::Error),
    InvalidMagic([u8; 4]),
    UnsupportedVersion(f32),
    InvalidFormat(&'static str),
}

pub type PmxResult<T> = Result<T, PmxError>;

impl From<io::Error> for PmxError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for PmxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::InvalidMagic(magic) => {
                let magic = String::from_utf8_lossy(magic);
                write!(f, "invalid PMX magic: {magic:?}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported PMX version: {version}")
            }
            Self::InvalidFormat(message) => write!(f, "invalid PMX data: {message}"),
        }
    }
}

impl Error for PmxError {}
