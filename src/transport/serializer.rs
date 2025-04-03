use serde::Serialize;
use thiserror::Error;
use std::collections::HashMap;
// Removed unused imports: Entity, World

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
pub trait Serializer: Send + Sync + SerializerClone {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError>;
}

impl Clone for Box<dyn Serializer> {
    fn clone(&self) -> Self {
        self.clone_serializer()
    }
}

/// Helper trait to make Serializer cloneable via object-safe methods
pub trait SerializerClone {
    fn clone_serializer(&self) -> Box<dyn Serializer>;
}

// Implement SerializerClone for all T that implement Serializer + Clone
impl<T> SerializerClone for T 
where 
    T: Serializer + Clone + 'static 
{
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
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
#[derive(Clone)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_json()
    }
}

/// Binary serializer implementation using bincode
#[derive(Clone)]
pub struct BinarySerializer;

impl Serializer for BinarySerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        data.to_binary()
    }
}

/// Optimized binary serializer with delta compression
#[derive(Clone)]
pub struct DeltaCompressor {
    /// Previous positions of entities
    last_positions: HashMap<u32, [f32; 2]>,
    /// Threshold for considering an entity as moved (squared distance)
    threshold_squared: f32,
}

impl DeltaCompressor {
    /// Create a new delta compressor with the specified movement threshold
    pub fn new(threshold: f32) -> Self {
        Self {
            last_positions: HashMap::new(),
            threshold_squared: threshold * threshold,
        }
    }
    
    /// Filter simulation state to include only entities that have moved significantly
    pub fn filter_state<T>(&mut self, state: &super::SimulationState) -> super::SimulationState 
    where T: Clone
    {
        // Create a new state with only particles that have moved
        let mut filtered_particles = Vec::new();
        
        for particle in &state.particles {
            let entity_id = particle.id; // Already u32
            let current_pos = [particle.x, particle.y]; // Create array for comparison
            
            // Check if the entity has moved significantly
            let should_include = match self.last_positions.get(&entity_id) {
                Some(last_pos) => {
                    let dx = current_pos[0] - last_pos[0];
                    let dy = current_pos[1] - last_pos[1];
                    let dist_squared = dx*dx + dy*dy;
                    
                    // Include if moved more than threshold
                    dist_squared > self.threshold_squared
                },
                None => true, // Always include new entities
            };
            
            if should_include {
                // Update the last known position
                self.last_positions.insert(entity_id, current_pos);
                filtered_particles.push(particle.clone());
            }
        }
        
        // Create a new state with only the filtered particles
        super::SimulationState {
            frame: state.frame,
            timestamp: state.timestamp,
            particles: filtered_particles,
        }
    }
}

/// Optimized binary serializer with configurable settings
#[derive(Clone)]
pub struct OptimizedBinarySerializer {
    // Removed bincode config field as it caused compilation errors with bincode 1.3.3 API
    /// Delta compressor for filtering unchanged entities
    delta_compressor: Option<DeltaCompressor>,
}

impl OptimizedBinarySerializer {
    /// Create a new optimized binary serializer
    pub fn new(delta_threshold: Option<f32>) -> Self {
        // Create delta compressor if threshold provided
        let delta_compressor = delta_threshold.map(DeltaCompressor::new);
            
        Self { delta_compressor } // Removed config initialization
    }
    
    /// Serialize simulation state with optimizations
    pub fn serialize_state(&mut self, state: &super::SimulationState) -> Result<Vec<u8>, SerializationError> {
        // Apply delta compression if enabled
        let final_state = if let Some(compressor) = &mut self.delta_compressor {
            compressor.filter_state::<super::ParticleState>(state)
        } else {
            state.clone()
        };
        
        // Serialize using standard bincode serialization
        bincode::serialize(&final_state)
            .map_err(SerializationError::BinaryError)
    }
}

impl Serializer for OptimizedBinarySerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        // For now, fall back to standard binary serialization
        // The specialized serialize_state method should be used for SimulationState
        data.to_binary()
    }
}
