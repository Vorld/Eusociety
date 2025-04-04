use bevy_ecs::prelude::*;
use std::time::Instant; 
use tracing::{error, info, debug}; // Added info and debug for logging

use crate::simulation::components::Position;
use crate::transport::{TransportController, SimulationState, ParticleState};

/// Resource for tracking simulation timing
#[derive(Resource)]
pub struct SimulationTimer {
    pub start_time: Instant,
    pub last_frame_time: Instant,
    pub frame_count: u64,
    // Adding performance tracking metrics
    pub extract_time_ms: f64,
    pub serialize_time_ms: f64,
    pub send_time_ms: f64,
    pub total_transport_time_ms: f64,
    // Performance history for reporting
    pub last_report_time: Instant,
    pub report_interval_secs: u64,
}

impl Default for SimulationTimer {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
            frame_count: 0,
            extract_time_ms: 0.0,
            serialize_time_ms: 0.0,
            send_time_ms: 0.0,
            total_transport_time_ms: 0.0,
            last_report_time: now,
            report_interval_secs: 5, // Report every 5 seconds
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
    // Start overall timing
    let overall_start = Instant::now();
    
    // Update simulation timing
    let now = Instant::now();
    timer.frame_count += 1;
    timer.last_frame_time = now;
    
    // Start extraction timing
    let extract_start = Instant::now();
    
    // Extract entity position data
    let mut particles = Vec::with_capacity(query.iter().len());
    
    for (entity, position) in query.iter() {
        particles.push(ParticleState {
            id: entity.index(),
            x: position.x,
            y: position.y,
        });
    }
    
    // Record extraction time
    let extract_time = extract_start.elapsed();
    timer.extract_time_ms = extract_time.as_secs_f64() * 1000.0;
    
    // Create the complete simulation state
    let state = SimulationState {
        frame: timer.frame_count,
        timestamp: now.duration_since(timer.start_time).as_secs_f64(),
        particles,
    };
    
    // Start send timing (includes serialization)
    let send_start = Instant::now();
    
    // Record the size of data before sending
    let particle_count = state.particles.len();
    
    // Send through transport controller (this includes serialization)
    if let Err(e) = transport.controller.send_simulation_state(&state) {
        error!("Failed to send simulation state: {}", e);
    }
    
    // Record send time (including serialization)
    let send_time = send_start.elapsed();
    timer.send_time_ms = send_time.as_secs_f64() * 1000.0;
    
    // Record overall transport time
    let overall_time = overall_start.elapsed();
    timer.total_transport_time_ms = overall_time.as_secs_f64() * 1000.0;
    
    // Periodically log performance metrics
    if now.duration_since(timer.last_report_time).as_secs() >= timer.report_interval_secs {
        info!(
            particles = particle_count,
            extract_ms = timer.extract_time_ms,
            send_ms = timer.send_time_ms, 
            total_ms = timer.total_transport_time_ms,
            "Transport performance metrics"
        );
        timer.last_report_time = now;
    }
}

/// System for flushing the transport controller (if needed)
pub fn flush_transport(mut transport: ResMut<SimulationTransport>) {
    let start = Instant::now();
    
    if let Err(e) = transport.controller.flush() {
        error!("Failed to flush transport: {}", e);
    }
    
    let elapsed = start.elapsed();
    debug!(flush_time_ms = elapsed.as_secs_f64() * 1000.0, "Transport flush time");
}
