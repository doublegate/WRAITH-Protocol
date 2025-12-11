// Error types for WRAITH Android JNI

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),

    #[error("WRAITH protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
