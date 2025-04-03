mod serializer;
mod sender;

use serde::Serialize;

// Re-export types
pub use self::serializer::{Serializer, SerializationError, JsonSerializer, BinarySerializer, SerializeObject};
pub use self::sender::{Sender, TransportError, FileSender};

/// Particle state for serialization
#[derive(Serialize)]
pub struct ParticleState {
    pub id: usize,
    pub position: [f32; 2],
}

/// Complete simulation state for serialization
#[derive(Serialize)]
pub struct SimulationState {
    pub frame: u64,
    pub timestamp: f64,
    pub particles: Vec<ParticleState>,
}

/// Controller for handling serialization and transport of simulation data
pub struct TransportController {
    serializer: Box<dyn Serializer>,
    sender: Box<dyn Sender>,
}

impl TransportController {
    /// Create a new transport controller with the provided serializer and sender
    pub fn new(
        serializer: Box<dyn Serializer>,
        sender: Box<dyn Sender>,
    ) -> Self {
        Self { serializer, sender }
    }
    
    /// Create a transport controller from configuration
    pub fn from_config(config: &crate::config::TransportConfig) -> Result<Self, TransportError> {
        // Create serializer based on configuration
        let serializer: Box<dyn Serializer> = match config.serializer_type {
            crate::config::SerializerType::Json => Box::new(JsonSerializer),
            crate::config::SerializerType::Binary => Box::new(BinarySerializer),
        };
        
        // Create file sender with output path
        let sender: Box<dyn Sender> = Box::new(FileSender::new(&config.output_path)?);
        
        Ok(Self::new(serializer, sender))
    }
    
    /// Serialize and send simulation state
    pub fn send_state<T: SerializeObject + Serialize>(&self, state: &T) -> Result<(), TransportError> {
        // Serialize data
        let data = self.serializer.serialize_to_bytes(state)
            .map_err(TransportError::SerializationError)?;
        
        // Send data through sender
        self.sender.send(&data)?;
        
        Ok(())
    }
    
    /// Flush the sender to ensure data is written
    pub fn flush(&self) -> Result<(), TransportError> {
        self.sender.flush()
    }
}