//! Defines traits and implementations for sending serialized simulation data.
//!
//! This module provides:
//! - The `Sender` trait defining the interface for sending byte data.
//! - Concrete implementations: `FileSender`, `NullSender`.
//! - (Note: `WebSocketSender` is now in `websocket.rs`).
//! - Helper traits (`SenderClone`) and error types (`TransportError`).

use std::fs::File;
use std::io::{Write, Error as IoError};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{info, error}; // Ensure tracing macros are imported (removed warn)

// Import WebSocketSender from the parent module (transport::mod.rs re-exports it)
use super::WebSocketSender; 
use super::serializer::SerializationError;

/// Error types that can occur during data transport (sending).
#[derive(Error, Debug)]
pub enum TransportError {
    /// An I/O error occurred (e.g., writing to a file).
    #[error("I/O error: {0}")]
    IoError(#[from] IoError),
    /// An error occurred during serialization before sending.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] SerializationError),
    /// A WebSocket-specific error occurred (e.g., connection failure).
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    /// An error related to the Tokio runtime occurred (e.g., spawning tasks).
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    /// An error occurred due to invalid transport configuration.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Base trait for sending serialized data.
///
/// Defines methods for sending byte slices and flushing buffers.
/// Requires `Send + Sync + SenderClone` for thread safety and clonability
/// when used as a trait object (`Box<dyn Sender>`).
pub trait Sender: Send + Sync + SenderClone {
    /// Sends the provided byte slice to the destination.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte slice containing the serialized data to send.
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if the send operation fails.
    fn send(&self, data: &[u8]) -> Result<(), TransportError>;

    /// Flushes any internal buffers to ensure data is sent/written.
    /// May be a no-op for some implementations (like WebSocket).
    ///
    /// # Errors
    ///
    /// Returns `TransportError` if flushing fails.
    fn flush(&self) -> Result<(), TransportError>;

    /// Attempts to downcast this sender to a `WebSocketSender`.
    ///
    /// Returns `Some(&WebSocketSender)` if the underlying type is `WebSocketSender`,
    /// otherwise returns `None`. This is useful for accessing WebSocket-specific methods.
    fn as_websocket_sender(&self) -> Option<&WebSocketSender> {
        // Default implementation returns None. WebSocketSender overrides this.
        None 
    }
}

/// Enables cloning of `Box<dyn Sender>`.
impl Clone for Box<dyn Sender> {
    fn clone(&self) -> Self {
        self.clone_sender() // Delegates to the object-safe clone method
    }
}

/// Helper trait providing an object-safe cloning method for `Sender`.
/// Necessary to allow `Box<dyn Sender>` to be cloneable.
pub trait SenderClone {
    /// Creates a boxed clone of the `Sender`.
    fn clone_sender(&self) -> Box<dyn Sender>;
}

// Implement `SenderClone` for each concrete sender type.
/// Sender implementation that writes data to a file.
///
/// Each call to `send` appends the data followed by a newline character.
/// Uses an `Arc<Mutex<File>>` for thread-safe access if cloned.
#[derive(Clone)]
pub struct FileSender {
    /// The path to the output file (stored for potential debugging).
    _file_path: String, 
    /// Thread-safe handle to the output file.
    file: Arc<Mutex<File>>,
}

impl FileSender {
    /// Creates a new `FileSender` that writes to the specified file path.
    /// Creates the file if it doesn't exist, truncates it if it does.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the output file.
    ///
    /// # Errors
    ///
    /// Returns `TransportError::IoError` if the file cannot be created or opened.
    pub fn new(file_path: &str) -> Result<Self, TransportError> {
        let file = File::create(file_path)?; // Create/truncate the file
        info!("Initialized FileSender for path: {}", file_path);
        Ok(Self {
            _file_path: file_path.to_string(), 
            file: Arc::new(Mutex::new(file)),
        })
    }
}

impl Sender for FileSender {
    /// Appends the data slice and a newline character to the file.
    fn send(&self, data: &[u8]) -> Result<(), TransportError> {
        // Lock the mutex to get exclusive access to the file handle
        let mut file_guard = self.file.lock().map_err(|_| TransportError::RuntimeError("File mutex poisoned".to_string()))?;
        file_guard.write_all(data)?; // Write the data
        file_guard.write_all(b"\n")?; // Append a newline
        Ok(())
    }

    /// Flushes the file's internal buffer to ensure data is written to disk.
    fn flush(&self) -> Result<(), TransportError> {
        let mut file_guard = self.file.lock().map_err(|_| TransportError::RuntimeError("File mutex poisoned".to_string()))?;
        file_guard.flush()?;
        Ok(())
    }
}

impl SenderClone for FileSender {
    fn clone_sender(&self) -> Box<dyn Sender> {
        Box::new(self.clone()) // Simply clone the struct (Arc makes this cheap)
    }
}

/// A sender implementation that does nothing.
/// Useful for disabling data transport via configuration.
#[derive(Clone)]
pub struct NullSender;

impl Sender for NullSender {
    /// Performs no operation.
    fn send(&self, _data: &[u8]) -> Result<(), TransportError> {
        Ok(()) // Always succeeds, does nothing
    }

    /// Performs no operation.
    fn flush(&self) -> Result<(), TransportError> {
        Ok(()) // Always succeeds, does nothing
    }
}

impl SenderClone for NullSender {
    fn clone_sender(&self) -> Box<dyn Sender> {
        Box::new(self.clone())
    }
}

// --- WebSocket Sender Logic Moved to websocket.rs ---
