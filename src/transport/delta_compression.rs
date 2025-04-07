//! Implements delta compression logic for optimizing simulation state serialization.
//! Only entities that have moved significantly since the last sent state are included.

use std::collections::HashMap;
use crate::transport::SimulationState; // Import necessary types

/// State container for delta compression.
///
/// Stores the last known positions of particles and filters incoming `SimulationState`
/// based on a movement threshold. Also tracks metrics about compression effectiveness.
#[derive(Clone)]
pub struct DeltaCompressor {
    /// Stores the last known position `[x, y]` for each particle ID (`u32`).
    last_positions: HashMap<u32, [f32; 2]>,
    /// The square of the movement distance threshold. Comparing squared distances
    /// avoids costly square root calculations in the filtering logic.
    threshold_squared: f32,
    /// Tracks statistics about the delta compression process.
    metrics: DeltaCompressionMetrics,
}

/// Stores metrics related to the effectiveness of delta compression.
#[derive(Clone, Debug, Default)]
pub struct DeltaCompressionMetrics {
    /// Total number of particles processed across all frames since initialization or reset.
    pub total_particles_processed: usize,
    /// Total number of particles included in the output state after filtering (cumulative).
    pub total_particles_sent: usize,
    /// Number of particles processed in the most recent call to `filter_state`.
    pub last_frame_particles_processed: usize,
    /// Number of particles included in the output state in the most recent call to `filter_state`.
    pub last_frame_particles_sent: usize,
    /// Running average of the percentage of particles filtered out across all processed frames.
    pub avg_reduction_pct: f32,
}

impl DeltaCompressor {
    /// Creates a new `DeltaCompressor`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - The minimum distance a particle must move for its new state
    ///   to be included in the filtered output.
    pub fn new(threshold: f32) -> Self {
        Self {
            last_positions: HashMap::new(),
            // Store the threshold squared for efficient comparison
            threshold_squared: threshold * threshold,
            metrics: DeltaCompressionMetrics::default(),
        }
    }
    
    /// Filters the provided `SimulationState`, returning a new state containing only
    /// particles that have moved more than the configured threshold since the last
    /// call to this method for that particle ID.
    ///
    /// Updates the internal `last_positions` map and calculates performance metrics.
    ///
    /// # Arguments
    ///
    /// * `state` - The current `SimulationState` to filter.
    ///
    /// # Returns
    ///
    /// A new `SimulationState` containing only the particles that met the movement threshold.
    pub fn filter_state(&mut self, state: &SimulationState) -> SimulationState 
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
        SimulationState {
            frame: state.frame,
            timestamp: state.timestamp,
            particles: filtered_particles,
        } 
    } 
    
    /// Returns a reference to the current delta compression metrics.
    pub fn metrics(&self) -> &DeltaCompressionMetrics {
        &self.metrics
    }
    
    /// Returns the configured movement threshold (calculates sqrt from stored squared value).
    pub fn threshold(&self) -> f32 {
        self.threshold_squared.sqrt()
    }
    
    /// Updates the movement threshold used for filtering.
    ///
    /// # Arguments
    ///
    /// * `threshold` - The new minimum distance threshold.
    pub fn set_threshold(&mut self, threshold: f32) -> &mut Self {
        self.threshold_squared = threshold * threshold; // Store squared value
        self
    }
}
