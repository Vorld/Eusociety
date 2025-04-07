//! Contains the Bevy system responsible for collecting the current simulation state
//! into a resource for transport.

use bevy_ecs::prelude::*;
use crate::simulation::components::{ParticleId, Position, Velocity};
use crate::simulation::resources::{CurrentSimulationState, FrameCounter}; 
use crate::transport::{ParticleState, SimulationState};

/// Bevy system that gathers the current state of all particles (`ParticleId`, `Position`)
/// and updates the `CurrentSimulationState` resource.
///
/// This system should typically run *after* all other simulation logic systems
/// (movement, boundary handling, etc.) within the update schedule to capture the
/// final state of the frame before it's potentially sent by the transport layer.
///
/// # Arguments
///
/// * `state_resource` - Mutable access to the `CurrentSimulationState` resource to update it.
/// * `particles` - A Bevy query to access the `ParticleId`, `Position`, and `Velocity` components of all particle entities. (Note: Velocity is queried but currently unused in the state snapshot).
/// * `frame_counter` - The `FrameCounter` resource providing the current frame number and timestamp.
pub fn update_current_simulation_state_resource(
    mut state_resource: ResMut<CurrentSimulationState>,
    particles: Query<(&ParticleId, &Position, &Velocity)>,
    frame_counter: Res<FrameCounter>, 
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
