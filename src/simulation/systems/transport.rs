use bevy_ecs::prelude::*;
use std::time::Instant; // Removed unused Duration
use tracing::error;

use crate::simulation::components::Position;
use crate::transport::{TransportController, SimulationState, ParticleState};

/// Resource for tracking simulation timing
#[derive(Resource)]
pub struct SimulationTimer {
    pub start_time: Instant,
    pub last_frame_time: Instant,
    pub frame_count: u64,
}

impl Default for SimulationTimer {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
            frame_count: 0,
        }
    }
}

/// Resource that holds the transport controller
#[derive(Resource)]
pub struct SimulationTransport {
    pub controller: TransportController,
}

/// System for extracting entity position data and sending it through the transport layer
pub fn extract_and_send(
    query: Query<(Entity, &Position)>,
    mut transport: ResMut<SimulationTransport>,
    mut timer: ResMut<SimulationTimer>,
) {
    // Update simulation timing
    let now = Instant::now();
    timer.frame_count += 1;
    timer.last_frame_time = now;
    
    // Extract entity position data
    let mut particles = Vec::with_capacity(query.iter().len());
    
    for (entity, position) in query.iter() {
        particles.push(ParticleState {
            id: entity.index(),
            x: position.x, // Use separate fields
            y: position.y,
        });
    }
    
    // Create the complete simulation state
    let state = SimulationState {
        frame: timer.frame_count,
        timestamp: now.duration_since(timer.start_time).as_secs_f64(),
        particles,
    };
    
    // Send through transport controller
    if let Err(e) = transport.controller.send_simulation_state(&state) {
        error!("Failed to send simulation state: {}", e);
    }
}

/// System for flushing the transport controller (if needed)
pub fn flush_transport(transport: ResMut<SimulationTransport>) {
    if let Err(e) = transport.controller.flush() {
        error!("Failed to flush transport: {}", e);
    }
}
