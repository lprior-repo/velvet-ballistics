#![forbid(unsafe_code)]

pub mod fixture;
pub mod seed;
pub mod temp_keyspace;

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestSetupError {
    #[error("out of memory")]
    OutOfMemory,
    #[error("invalid seed: {0}")]
    InvalidSeed(String),
    #[error("invalid capacity: {0}")]
    InvalidCapacity(String),
    #[error("temp directory error: {0}")]
    TempDirError(String),
    #[error("fjall open error: {0}")]
    FjallOpenError(String),
    #[error("postcard encode error: {0}")]
    PostcardEncodeError(String),
    #[error("postcard decode error: {0}")]
    PostcardDecodeError(String),
    #[error("assertion mismatch: {0}")]
    AssertionMismatch(String),
}
