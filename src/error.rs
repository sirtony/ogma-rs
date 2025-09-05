use thiserror::Error;

pub type Result<T> = std::result::Result<T, crate::error::Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("file is not a valid store file or is corrupted")]
    InvalidFile,

    #[error("file format is version {actual}, but this library only supports version {expected}")]
    WrongVersion { expected: u16, actual: u16 },

    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),

    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),
}
