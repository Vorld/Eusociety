use std::fs::File;
use std::io::{Write, Error as IoError};
use std::sync::Mutex;
use thiserror::Error;

use super::serializer::SerializationError;

/// Error types for transport operations
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    IoError(#[from] IoError),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
}

/// Trait for sending serialized data
pub trait Sender: Send + Sync {
    fn send(&self, data: &[u8]) -> Result<(), TransportError>;
    fn flush(&self) -> Result<(), TransportError>;
}

/// File-based sender implementation
pub struct FileSender {
    file_path: String,
    file: Mutex<File>,
}

impl FileSender {
    pub fn new(file_path: &str) -> Result<Self, TransportError> {
        let file = File::create(file_path)?;
        Ok(Self {
            file_path: file_path.to_string(),
            file: Mutex::new(file),
        })
    }
}

impl Sender for FileSender {
    fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        let mut file = self.file.lock().expect("Failed to lock file mutex");
        file.write_all(data)?;
        file.write_all(b"\n")?; // Add a newline for JSON readability
        Ok(())
    }
    
    fn flush(&self) -> Result<(), TransportError> {
        let mut file = self.file.lock().expect("Failed to lock file mutex");
        file.flush()?;
        Ok(())
    }
}