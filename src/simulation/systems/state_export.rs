use bevy_ecs::prelude::*;
use crate::simulation::components::{ParticleId, Position, Velocity};
use crate::simulation::resources::{CurrentSimulationState, FrameCounter}; // Assuming FrameCounter is the correct resource for frame number
use crate::transport::{ParticleState, SimulationState};

/// System to query the current state of particles and update the CurrentSimulationState resource.
/// This should run after simulation logic systems.
pub fn update_current_simulation_state_resource(
    mut state_resource: ResMut<CurrentSimulationState>,
    particles: Query<(&ParticleId, &Position, &Velocity)>,
    frame_counter: Res<FrameCounter>, // Access frame counter resource
) {
    // Collect particle states efficiently
    let particle_states: Vec<ParticleState> = particles
        .iter()
        // Add explicit types to closure parameters to avoid ambiguity
        .map(|(id_ref, pos_ref, _vel_ref): (&ParticleId, &Position, &Velocity)| ParticleState {
            id: id_ref.0 as u32, // Cast ParticleId(usize) to ParticleState.id(u32)
            x: pos_ref.x,        // Access x from Position reference
            y: pos_ref.y,        // Access y from Position reference
        })
        .collect();

    // Update the resource, ensuring all fields of SimulationState are present
    state_resource.0 = SimulationState {
        frame: frame_counter.count,       // Frame number from FrameCounter resource
        timestamp: frame_counter.timestamp, // Timestamp from FrameCounter resource
        particles: particle_states,       // Collected particle states
    };

    // Optional: Log state update for debugging
    // log::debug!("Updated CurrentSimulationState for frame {}", frame_counter.count);
}
