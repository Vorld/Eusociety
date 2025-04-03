mod serializer;
mod sender;

use serde::Serialize;

// Re-export types
pub use self::serializer::{
    Serializer, SerializationError, JsonSerializer, BinarySerializer, 
    SerializeObject, OptimizedBinarySerializer, DeltaCompressor
};
pub use self::sender::{Sender, TransportError, FileSender, WebSocketSender};

/// Particle state for serialization
#[derive(Serialize, Clone)]
pub struct ParticleState {
    pub id: u32,
    pub x: f32, // Use separate fields instead of array
    pub y: f32,
}

/// Complete simulation state for serialization
#[derive(Serialize, Clone)]
pub struct SimulationState {
    pub frame: u64,
    pub timestamp: f64,
    pub particles: Vec<ParticleState>,
}

/// Controller for handling serialization and transport of simulation data
#[derive(Clone)]
pub struct TransportController {
    serializer: Box<dyn Serializer>,
    sender: Box<dyn Sender>,
    optimized_serializer: Option<OptimizedBinarySerializer>,
    update_frequency: Option<u32>,
    current_frame: u32,
    sender_type: crate::config::SenderType,
}

impl TransportController {
    /// Create a new transport controller with the provided serializer and sender
    pub fn new(
        serializer: Box<dyn Serializer>,
        sender: Box<dyn Sender>,
    ) -> Self {
        Self { 
            serializer, 
            sender,
            optimized_serializer: None,
            update_frequency: None,
            current_frame: 0,
            sender_type: crate::config::SenderType::File,
        }
    }
    
    /// Create a transport controller from configuration
    pub fn from_config(config: &crate::config::TransportConfig) -> Result<Self, TransportError> {
        // Create regular serializer based on configuration
        let serializer: Box<dyn Serializer> = match config.serializer_type {
            crate::config::SerializerType::Json => Box::new(JsonSerializer),
            crate::config::SerializerType::Binary => Box::new(BinarySerializer),
        };
        
        // Create sender based on configuration
        let sender: Box<dyn Sender> = match config.sender_type {
            crate::config::SenderType::File => {
                Box::new(FileSender::new(&config.output_path)?)
            },
            crate::config::SenderType::WebSocket => {
                // Use WebSocketSender with the configured address
                if let Some(addr) = &config.websocket_address {
                    Box::new(WebSocketSender::new(addr)?)
                } else {
                    return Err(TransportError::ConfigurationError(
                        "WebSocket address not provided in configuration".to_string()
                    ));
                }
            }
        };
        
        // Create optimized binary serializer with delta compression if using WebSocket and it's enabled
        let optimized_serializer = if let crate::config::SenderType::WebSocket = config.sender_type {
            let threshold = if config.delta_compression == Some(true) {
                // Use delta compression with a reasonable threshold (0.1 units)
                Some(0.1f32)
            } else {
                None
            };
            
            Some(OptimizedBinarySerializer::new(threshold))
        } else {
            None
        };
        
        // Set update frequency if specified in config
        let update_frequency = config.update_frequency;
        
        let mut controller = Self::new(serializer, sender);
        controller.optimized_serializer = optimized_serializer;
        controller.update_frequency = update_frequency;
        controller.sender_type = config.sender_type.clone();
        
        Ok(controller)
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
    
    /// Serialize and send simulation state with optimized serializer if available
    pub fn send_simulation_state(&mut self, state: &SimulationState) -> Result<(), TransportError> {
        // Increment frame counter
        self.current_frame += 1;
        
        // Check if we should send this frame based on update frequency
        if let Some(freq) = self.update_frequency {
            if self.current_frame % freq != 0 {
                return Ok(());
            }
        }
        
        // Use optimized serializer if available
        let data = if let Some(serializer) = &mut self.optimized_serializer {
            serializer.serialize_state(state)
                .map_err(TransportError::SerializationError)?
        } else {
            self.serializer.serialize_to_bytes(state)
                .map_err(TransportError::SerializationError)?
        };
        
        // Send data
        self.sender.send(&data)?;
        
        Ok(())
    }
    
    /// Flush the sender to ensure data is written
    pub fn flush(&self) -> Result<(), TransportError> {
        self.sender.flush()
    }
    
    /// Get a reference to the WebSocket sender if available
    pub fn get_websocket_sender(&self) -> Option<&WebSocketSender> {
        if let crate::config::SenderType::WebSocket = self.sender_type {
            self.sender.as_websocket_sender()
        } else {
            None
        }
    }
}
