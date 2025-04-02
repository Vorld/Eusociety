use eusociety_core::{World, Position, Entity};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use thiserror::Error;

// --- Error Handling ---

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Serialization failed: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("I/O error during sending: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid configuration or options for transport: {0}")]
    ConfigError(String),
    #[error("Unsupported transport type: {0}")]
    UnsupportedType(String),
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

// --- Traits ---

/// Trait for serializing simulation state (or parts of it).
pub trait Serializer: Send + Sync {
    // Send + Sync needed if runner uses multiple threads potentially
    /// Serializes the relevant parts of the world state.
    /// For M1, we'll just serialize the positions HashMap.
    fn serialize(&self, world: &World) -> Result<Vec<u8>, TransportError>;
}

/// Trait for sending serialized data.
pub trait Sender: Send + Sync {
    /// Sends the provided data buffer.
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
}

// --- Implementations ---

// ## Serializers ##

pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    fn serialize(&self, world: &World) -> Result<Vec<u8>, TransportError> {
        // Convert the positions from the new component system to a HashMap for serialization
        let mut positions_map: HashMap<Entity, Position> = HashMap::new();
        
        // Use the query API to collect all positions
        for (entity, pos) in world.components.query::<Position>() {
            positions_map.insert(entity, *pos);
        }
        
        Ok(bincode::serialize(&positions_map)?)
    }
}

// Add JsonSerializer for debugging
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize(&self, world: &World) -> Result<Vec<u8>, TransportError> {
        // Convert the positions from the new component system to a HashMap for serialization
        let mut positions_map: HashMap<Entity, Position> = HashMap::new();
        
        for (entity, pos) in world.components.query::<Position>() {
            positions_map.insert(entity, *pos);
        }
        
        // Serialize the positions to pretty-printed JSON for readability
        let json_data = serde_json::to_string_pretty(&positions_map)?;
        Ok(json_data.into_bytes())
    }
}

// ## Senders ##

pub struct FileSender {
    file: File,
    // We might add options like appending or truncating later
}

impl FileSender {
    /// Creates a new FileSender. Expects options["path"] to be a valid string path.
    pub fn new(options: &Option<HashMap<String, Value>>) -> Result<Self, TransportError> {
        let path_str = options
            .as_ref()
            .and_then(|opts| opts.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TransportError::ConfigError("FileSender requires 'options.path' string".to_string())
            })?;

        let path = PathBuf::from(path_str);
        // Create or truncate the file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true) // Overwrite existing file each run
            .open(path)?;

        Ok(FileSender { file })
    }
}

impl Sender for FileSender {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.file.write_all(data)?;
        // Optional: Add a delimiter if sending multiple frames to the same file stream
        // self.file.write_all(b"\n")?;
        self.file.flush()?; // Ensure data is written immediately for validation
        Ok(())
    }
}

// Add ConsoleSender for simple debugging output
pub struct ConsoleSender;

impl ConsoleSender {
     pub fn new(_options: &Option<HashMap<String, Value>>) -> Result<Self, TransportError> {
         // No options needed for console sender
         Ok(ConsoleSender)
     }
}

impl Sender for ConsoleSender {
     fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
         // Attempt to print as lossy UTF-8 for debugging, otherwise print hex
         println!("Transport Data: {}", String::from_utf8_lossy(data));
         // Or for binary:
         // println!("Transport Data ({} bytes): {:02X?}", data.len(), data);
         Ok(())
     }
}


// --- Factory Functions (Example) ---

/// Creates a Serializer instance based on configuration name.
pub fn create_serializer(name: &str) -> Result<Box<dyn Serializer>, TransportError> {
    match name.to_lowercase().as_str() {
        "binary" => Ok(Box::new(BinarySerializer)),
        "json" => Ok(Box::new(JsonSerializer)),
        _ => Err(TransportError::UnsupportedType(format!(
            "Unsupported serializer type: {}",
            name
        ))),
    }
}

/// Creates a Sender instance based on configuration name and options.
pub fn create_sender(
    name: &str,
    options: &Option<HashMap<String, Value>>,
) -> Result<Box<dyn Sender>, TransportError> {
    match name.to_lowercase().as_str() {
        "file" => Ok(Box::new(FileSender::new(options)?)),
         "console" => Ok(Box::new(ConsoleSender::new(options)?)),
        // "websocket" => Ok(Box::new(WebSocketSender::new(options)?)), // Add later
        _ => Err(TransportError::UnsupportedType(format!(
            "Unsupported sender type: {}",
            name
        ))),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use eusociety_core::{Position, World};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_binary_serializer() {
        let mut world = World::new();
        let e0: Entity = 0;
        let e1: Entity = 1;
        world.add_component(e0, Position { x: 1.0, y: 2.0 });
        world.add_component(e1, Position { x: 3.0, y: 4.0 });

        let serializer = BinarySerializer;
        let data = serializer.serialize(&world).unwrap();

        // Deserialize back to check
        let positions_back: HashMap<u32, Position> = bincode::deserialize(&data).unwrap();
        assert_eq!(positions_back.len(), 2);
        assert_eq!(positions_back[&0].x, 1.0);
        assert_eq!(positions_back[&1].y, 4.0);
    }

    #[test]
    fn test_json_serializer() {
        let mut world = World::new();
        let e0: Entity = 0;
        let e1: Entity = 1;
        world.add_component(e0, Position { x: 1.0, y: 2.0 });
        world.add_component(e1, Position { x: 3.0, y: 4.0 });

        let serializer = JsonSerializer;
        let data = serializer.serialize(&world).unwrap();
        
        // Convert bytes back to string
        let json_str = String::from_utf8(data).unwrap();
        
        // Validate JSON structure
        let positions_back: HashMap<u32, Position> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(positions_back.len(), 2);
        assert_eq!(positions_back[&0].x, 1.0);
        assert_eq!(positions_back[&1].y, 4.0);
        
        // Verify it's actually pretty-printed JSON
        assert!(json_str.contains("\n"));
        assert!(json_str.contains("  "));
    }

    #[test]
    fn test_file_sender() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("output.test.bin");
        let path_str = file_path.to_str().unwrap().to_string();

        let options = Some(HashMap::from([(
            "path".to_string(),
            Value::String(path_str.clone()),
        )]));

        let mut sender = FileSender::new(&options).unwrap();
        let data_to_send = vec![1, 2, 3, 4, 5];
        sender.send(&data_to_send).unwrap();

        // Check file content
        let content = fs::read(&file_path).unwrap();
        assert_eq!(content, data_to_send);

        // Test overwrite
         let data_to_send2 = vec![10, 20];
         // Need a new sender instance to reopen/truncate the file
         let mut sender2 = FileSender::new(&options).unwrap();
         sender2.send(&data_to_send2).unwrap();
         let content2 = fs::read(&file_path).unwrap();
         assert_eq!(content2, data_to_send2);
    }

     #[test]
    fn test_file_sender_missing_path() {
        let options = Some(HashMap::new()); // Missing path
        let result = FileSender::new(&options);
        assert!(result.is_err());
        match result.err().unwrap() {
            TransportError::ConfigError(msg) => assert!(msg.contains("requires 'options.path'")),
            _ => panic!("Expected ConfigError"),
        }
    }

     #[test]
    fn test_create_serializer_factory() {
        let serializer = create_serializer("binary").unwrap();
        // We can't easily test the type of Box<dyn Trait>, but we know it succeeded.
        let result = create_serializer("unknown");
        assert!(result.is_err());
    }

     #[test]
    fn test_create_sender_factory() {
         let dir = tempdir().unwrap();
         let file_path = dir.path().join("output.factory.bin");
         let path_str = file_path.to_str().unwrap().to_string();
         let options = Some(HashMap::from([(
             "path".to_string(),
             Value::String(path_str.clone()),
         )]));

        let sender = create_sender("file", &options).unwrap();
         // Test console sender creation
         let console_sender = create_sender("console", &None).unwrap();

        let result = create_sender("unknown", &None);
        assert!(result.is_err());
    }
}
