use serde::Serialize;
use thiserror::Error;
use std::collections::HashMap;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Error types for serialization operations
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Binary serialization error: {0}")]
    BinaryError(#[from] bincode::Error),
    
    #[error("Parallel serialization error: {0}")]
    ParallelError(String),
}

// /// Enum to represent different serializer types (Currently unused)
// #[derive(Debug)]
// pub enum SerializerType {
//     Json,
//     Binary,
// }

/// Base serializer trait without generics for object-safety
pub trait Serializer: Send + Sync + SerializerClone {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError>;
}

impl Clone for Box<dyn Serializer> {
    fn clone(&self) -> Self {
        self.clone_serializer()
    }
}

/// Null serializer implementation (no-op)
#[derive(Clone)]
pub struct NullSerializer;

impl Serializer for NullSerializer {
    // Ensure this signature exactly matches the trait definition
    fn serialize_to_bytes(&self, _data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        // Return an empty Vec as there's nothing to serialize
        Ok(Vec::new())
    }
}

/// Helper trait to make Serializer cloneable via object-safe methods
pub trait SerializerClone {
    fn clone_serializer(&self) -> Box<dyn Serializer>;
}

// Removed the generic implementation to avoid conflicts.
// Explicit implementations are provided below for each Serializer type.

impl SerializerClone for NullSerializer {
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

impl SerializerClone for JsonSerializer {
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
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

impl SerializerClone for BinarySerializer {
    fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}

/// Optimized binary serializer with delta compression
#[derive(Clone)]
pub struct DeltaCompressor {
    /// Previous positions of entities
    last_positions: HashMap<u32, [f32; 2]>,
    /// Threshold for considering an entity as moved (squared distance)
    threshold_squared: f32,
    /// Metrics for monitoring delta compression effectiveness
    metrics: DeltaCompressionMetrics,
}

/// Metrics to track delta compression effectiveness
#[derive(Clone, Debug, Default)]
pub struct DeltaCompressionMetrics {
    /// Total particles processed (cumulative)
    pub total_particles_processed: usize,
    /// Total particles sent after filtering (cumulative)
    pub total_particles_sent: usize,
    /// Particles processed in the last frame
    pub last_frame_particles_processed: usize,
    /// Particles sent in the last frame
    pub last_frame_particles_sent: usize,
    /// Average data reduction percentage over time
    pub avg_reduction_pct: f32,
}

impl DeltaCompressor {
    /// Create a new delta compressor with the specified movement threshold
    pub fn new(threshold: f32) -> Self {
        Self {
            last_positions: HashMap::new(),
            threshold_squared: threshold * threshold,
            metrics: DeltaCompressionMetrics::default(),
        }
    }
    
    /// Filter simulation state to include only entities that have moved significantly
    pub fn filter_state<T>(&mut self, state: &super::SimulationState) -> super::SimulationState 
    where T: Clone
    {
        let original_count = state.particles.len();
        
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

        // Update metrics
        let filtered_count = filtered_particles.len();
        let reduction_pct = if original_count > 0 {
            100.0 * (1.0 - (filtered_count as f32 / original_count as f32))
        } else {
            0.0
        };
        
        self.metrics.total_particles_processed += original_count;
        self.metrics.total_particles_sent += filtered_count;
        self.metrics.last_frame_particles_processed = original_count;
        self.metrics.last_frame_particles_sent = filtered_count;
        
        // Update running average
        if self.metrics.total_particles_processed > 0 {
            self.metrics.avg_reduction_pct = 100.0 * (1.0 - (self.metrics.total_particles_sent as f32 / 
                                                      self.metrics.total_particles_processed as f32));
        }
        
        // Log metrics periodically (every 60 frames = ~1 second at 60 fps)
        if state.frame % 60 == 0 {
            tracing::info!(
                frame = state.frame,
                original_particles = original_count,
                filtered_particles = filtered_count,
                reduction_pct = format!("{:.2}%", reduction_pct),
                avg_reduction = format!("{:.2}%", self.metrics.avg_reduction_pct),
                threshold = (self.threshold_squared as f32).sqrt(),
                "Delta compression metrics"
            );
        }
        
        // Create a new state with only the filtered particles
        super::SimulationState {
            frame: state.frame,
            timestamp: state.timestamp,
            particles: filtered_particles,
        }
    }
    
    /// Get the current delta compression metrics
    pub fn metrics(&self) -> &DeltaCompressionMetrics {
        &self.metrics
    }
    
    /// Get the movement threshold
    pub fn threshold(&self) -> f32 {
        self.threshold_squared.sqrt()
    }
    
    /// Set a new movement threshold
    pub fn set_threshold(&mut self, threshold: f32) -> &mut Self {
        self.threshold_squared = threshold * threshold;
        self
    }
}

/// Optimized binary serializer with parallel processing capabilities
#[derive(Clone)]
pub struct OptimizedBinarySerializer {
    /// Delta compressor for filtering unchanged entities
    delta_compressor: Option<DeltaCompressor>,
    /// Determines whether to use parallel serialization for large particle counts
    use_parallel: bool,
    /// Threshold for switching to parallel serialization
    parallel_threshold: usize,
    /// Number of threads to use for serialization (0 = auto)
    thread_count: usize,
}

impl OptimizedBinarySerializer {
    /// Create a new optimized binary serializer
    pub fn new(delta_threshold: Option<f32>) -> Self {
        // Create delta compressor if threshold provided
        let delta_compressor = delta_threshold.map(DeltaCompressor::new);
            
        Self { 
            delta_compressor,
            use_parallel: true,
            parallel_threshold: 50000, // Use parallel serialization for more than 50K particles
            thread_count: 0,           // 0 means use Rayon's default thread pool
        }
    }
    
    /// Serialize simulation state with optimizations
    pub fn serialize_state(&mut self, state: &super::SimulationState) -> Result<Vec<u8>, SerializationError> {
        // Apply delta compression if enabled
        let final_state = if let Some(compressor) = &mut self.delta_compressor {
            compressor.filter_state::<super::ParticleState>(state)
        } else {
            state.clone()
        };
        
        // Check if we should use parallel serialization
        if self.use_parallel && final_state.particles.len() >= self.parallel_threshold {
            self.serialize_state_parallel_compatible(&final_state)
        } else {
            // Serialize using standard bincode serialization
            bincode::serialize(&final_state)
                .map_err(SerializationError::BinaryError)
        }
    }

    /// Serialize simulation state using parallel processing while maintaining
    /// binary compatibility with the frontend parser
    fn serialize_state_parallel_compatible(&self, state: &super::SimulationState) -> Result<Vec<u8>, SerializationError> {
        // Calculate the size of the final binary buffer
        // Note: This assumes Bincode's default serialization format
        // Frame (u64) + Timestamp (f64) + Particle count (length prefix, u64) + particles
        let particle_size = std::mem::size_of::<u32>() + // id
                          std::mem::size_of::<f32>() * 2; // x, y
        
        let header_size = std::mem::size_of::<u64>() + // frame
                         std::mem::size_of::<f64>() + // timestamp
                         std::mem::size_of::<u64>(); // array length prefix
        
        let total_size = header_size + (state.particles.len() * particle_size);
        
        // Pre-allocate buffer
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(total_size)));
        
        // First, serialize the header (frame, timestamp, and particle count)
        {
            let mut buffer_lock = buffer.lock().unwrap();
            
            // Frame (u64)
            let frame_bytes = bincode::serialize(&state.frame)
                .map_err(SerializationError::BinaryError)?;
            buffer_lock.extend_from_slice(&frame_bytes);
            
            // Timestamp (f64)
            let timestamp_bytes = bincode::serialize(&state.timestamp)
                .map_err(SerializationError::BinaryError)?;
            buffer_lock.extend_from_slice(&timestamp_bytes);
            
            // Particle count as u64 (for bincode length prefix)
            let count_bytes = bincode::serialize(&(state.particles.len() as u64))
                .map_err(SerializationError::BinaryError)?;
            buffer_lock.extend_from_slice(&count_bytes);
        }
        
        // Determine optimal chunk size
        let chunk_size = std::cmp::max(1, state.particles.len() / rayon::current_num_threads());
        
        // Create offsets for each thread
        let offsets: Vec<(usize, usize)> = (0..state.particles.len())
            .step_by(chunk_size)
            .map(|start| {
                let end = std::cmp::min(start + chunk_size, state.particles.len());
                (start, end)
            })
            .collect();
        
        // Process particles in parallel and directly insert into the pre-allocated buffer
        let result = offsets.par_iter()
            .try_for_each(|(start_idx, end_idx)| {
                // Serialize each particle in this chunk
                let mut chunk_data = Vec::with_capacity((end_idx - start_idx) * particle_size);
                
                for i in *start_idx..*end_idx {
                    let particle = &state.particles[i];
                    
                    // ID (u32)
                    let id_bytes = bincode::serialize(&particle.id)
                        .map_err(SerializationError::BinaryError)?;
                    chunk_data.extend_from_slice(&id_bytes);
                    
                    // X position (f32)
                    let x_bytes = bincode::serialize(&particle.x)
                        .map_err(SerializationError::BinaryError)?;
                    chunk_data.extend_from_slice(&x_bytes);
                    
                    // Y position (f32)
                    let y_bytes = bincode::serialize(&particle.y)
                        .map_err(SerializationError::BinaryError)?;
                    chunk_data.extend_from_slice(&y_bytes);
                }
                
                // Append this chunk's data to the main buffer
                let mut buffer_lock = buffer.lock().unwrap();
                buffer_lock.extend_from_slice(&chunk_data);
                
                Ok(())
            });
        
        // Handle any errors from parallel processing
        if let Err(e) = result {
            return Err(e);
        }
        
        // Return the complete buffer
        let final_buffer = Arc::try_unwrap(buffer)
            .map_err(|_| SerializationError::ParallelError("Failed to unwrap buffer".to_string()))?
            .into_inner()
            .map_err(|e| SerializationError::ParallelError(format!("Failed to unlock buffer: {}", e)))?;
            
        Ok(final_buffer)
    }
    
    /// Check if delta compression is enabled
    pub fn has_delta_compression(&self) -> bool {
        self.delta_compressor.is_some()
    }
    
    /// Enable or disable parallel serialization
    pub fn set_parallel(&mut self, enabled: bool) -> &mut Self {
        self.use_parallel = enabled;
        self
    }
    
    /// Set the threshold for parallel serialization
    pub fn set_parallel_threshold(&mut self, threshold: usize) -> &mut Self {
        self.parallel_threshold = threshold;
        self
    }
    
    /// Set the number of threads to use (0 = auto)
    pub fn set_thread_count(&mut self, count: usize) -> &mut Self {
        self.thread_count = count;
        self
    }
    
    /// Get the current parallel serialization threshold
    pub fn parallel_threshold(&self) -> usize {
        self.parallel_threshold
    }
    
    /// Get the current thread count setting
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }
    
    /// Check if parallel serialization is enabled
    pub fn is_parallel(&self) -> bool {
        self.use_parallel
    }
}

impl Serializer for OptimizedBinarySerializer {
    fn serialize_to_bytes(&self, data: &dyn SerializeObject) -> Result<Vec<u8>, SerializationError> {
        // For now, fall back to standard binary serialization
        // The specialized serialize_state method should be used for SimulationState
        data.to_binary()
    }
}

impl SerializerClone for OptimizedBinarySerializer {
     fn clone_serializer(&self) -> Box<dyn Serializer> {
        Box::new(self.clone())
    }
}
