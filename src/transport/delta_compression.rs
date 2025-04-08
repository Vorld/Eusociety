
//! Implements delta compression logic for optimizing simulation state serialization.
//! Only entities that have moved significantly since the last sent state are included.

use std::collections::HashMap;
// Import SimulationState only, AntExportState is accessed via state.ants
use crate::transport::SimulationState;

/// State container for delta compression.
///
/// Stores the last known positions of ants and filters incoming `SimulationState`
/// based on a movement threshold. Also tracks metrics about compression effectiveness.
/// Note: Currently only compresses ants, not nest or food sources.
#[derive(Clone)]
pub struct DeltaCompressor {
    /// Stores the last known position `[x, y]` for each ant ID (`u32`).
    last_positions: HashMap<u32, [f32; 2]>,
    /// The square of the movement distance threshold. Comparing squared distances
    /// avoids costly square root calculations in the filtering logic.
    threshold_squared: f32,
    /// Tracks statistics about the delta compression process.
    metrics: DeltaCompressionMetrics,
}

/// Stores metrics related to the effectiveness of delta compression for ants.
#[derive(Clone, Debug, Default)]
pub struct DeltaCompressionMetrics {
    /// Total number of ants processed across all frames since initialization or reset.
    pub total_ants_processed: usize,
    /// Total number of ants included in the output state after filtering (cumulative).
    pub total_ants_sent: usize,
    /// Number of ants processed in the most recent call to `filter_state`.
    pub last_frame_ants_processed: usize,
    /// Number of ants included in the output state in the most recent call to `filter_state`.
    pub last_frame_ants_sent: usize,
    /// Running average of the percentage of ants filtered out across all processed frames.
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
    /// ants that have moved more than the configured threshold since the last
    /// call to this method for that ant ID. Nest and food sources are passed through unchanged.
    ///
    /// Updates the internal `last_positions` map and calculates performance metrics for ants.
    ///
    /// # Arguments
    ///
    /// * `state` - The current `SimulationState` to filter.
    ///
    /// # Returns
    ///
    /// A new `SimulationState` containing only the ants that met the movement threshold,
    /// plus the original nest and food source data.
    pub fn filter_state(&mut self, state: &SimulationState) -> SimulationState
    {
        let original_ant_count = state.ants.len();

        // Create a new state with only ants that have moved
        let mut filtered_ants = Vec::new();

        for ant in &state.ants { // Iterate over ants now
            let entity_id = ant.id; // Use ant ID
            let current_pos = [ant.x, ant.y]; // Use ant position

            // Check if the ant has moved significantly
            let should_include = match self.last_positions.get(&entity_id) {
                Some(last_pos) => {
                    let dx = current_pos[0] - last_pos[0];
                    let dy = current_pos[1] - last_pos[1];
                    let dist_squared = dx*dx + dy*dy;

                    // Include if moved more than threshold
                    dist_squared > self.threshold_squared
                },
                None => true, // Always include new ants
            };

            if should_include {
                // Update the last known position
                self.last_positions.insert(entity_id, current_pos);
                filtered_ants.push(ant.clone()); // Add ant to filtered list
            }
        }

        // Update metrics for ants
        let filtered_ant_count = filtered_ants.len();
        let reduction_pct = if original_ant_count > 0 {
            100.0 * (1.0 - (filtered_ant_count as f32 / original_ant_count as f32))
        } else {
            0.0
        };

        self.metrics.total_ants_processed += original_ant_count;
        self.metrics.total_ants_sent += filtered_ant_count;
        self.metrics.last_frame_ants_processed = original_ant_count;
        self.metrics.last_frame_ants_sent = filtered_ant_count;

        // Update running average for ants
        if self.metrics.total_ants_processed > 0 {
            self.metrics.avg_reduction_pct = 100.0 * (1.0 - (self.metrics.total_ants_sent as f32 /
                                                      self.metrics.total_ants_processed as f32));
        }

        // Log metrics periodically
        if state.frame % 60 == 0 {
            tracing::info!(
                frame = state.frame,
                original_ants = original_ant_count,
                filtered_ants = filtered_ant_count,
                reduction_pct = format!("{:.2}%", reduction_pct),
                avg_reduction = format!("{:.2}%", self.metrics.avg_reduction_pct),
                threshold = self.threshold(), // Use method to get threshold
                "Ant delta compression metrics" // Updated log message
            );
        }

        // Create a new state with filtered ants and original nest/food
        SimulationState {
            frame: state.frame,
            timestamp: state.timestamp,
            ants: filtered_ants, // Use filtered ants
            nest: state.nest.clone(), // Clone nest state
            food_sources: state.food_sources.clone(), // Clone food sources
            pheromones: state.pheromones.clone(), // Clone pheromones (no delta compression for them yet)
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
