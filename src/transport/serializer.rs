use serde::Serialize;
use thiserror::Error;

/// Error types for serialization operations
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Binary serialization error: {0}")]
    BinaryError(#[from] bincode::Error),
}

/// Enum to represent different serializer types
#[derive(Debug)]
pub enum SerializerType {
    Json,
    Binary,
}

/// Base serializer trait without generics for object-safety
pub trait Serializer: Send + Sync {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError>;
}

/// Trait for objects that can be serialized
pub trait SerializeObject {
    fn to_json(&self) -> Result<Vec<u8>, SerializationError>;
    fn to_binary(&self) -> Result<Vec<u8>, SerializationError>;
}

// Implement SerializeObject for any type that implements Serialize
impl<T: Serialize + ?Sized> SerializeObject for T {
    fn to_json(&self) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(self).map_err(SerializationError::JsonError)
    }
    
    fn to_binary(&self) -> Result<Vec<u8>, SerializationError> {
        bincode::serialize(self).map_err(SerializationError::BinaryError)
    }
}

/// JSON serializer implementation
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_json()
    }
}

/// Binary serializer implementation using bincode
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_binary()
    }
}