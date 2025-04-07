//! Contains the Bevy system responsible for collecting the current simulation state
//! into a resource for transport.

use bevy_ecs::prelude::*;
use crate::simulation::components::{ParticleId, Position, Ant, AntState, Nest, FoodSource}; // Import Ant components
use crate::simulation::resources::{CurrentSimulationState, FrameCounter};
// Import specific export state structs and the main SimulationState
use crate::transport::{AntExportState, NestExportState, FoodSourceExportState, SimulationState};

/// Bevy system that gathers the current state of ants, nest, and food sources
/// and updates the `CurrentSimulationState` resource.
///
/// This system should typically run *after* all other simulation logic systems
/// (movement, boundary handling, etc.) within the update schedule to capture the
/// final state of the frame before it's potentially sent by the transport layer.
///
/// # Arguments
///
/// * `state_resource` - Mutable access to the `CurrentSimulationState` resource to update it.
/// * `query_ants` - Query for ant entities and their relevant components.
/// * `query_nest` - Query for the nest entity and its position.
/// * `query_food` - Query for food source entities and their positions.
/// * `frame_counter` - The `FrameCounter` resource providing the current frame number and timestamp.
pub fn update_current_simulation_state_resource(
    mut state_resource: ResMut<CurrentSimulationState>,
    query_ants: Query<(Entity, &ParticleId, &Position, &AntState), With<Ant>>, // Query ants
    query_nest: Query<&Position, With<Nest>>, // Query nest position
    query_food: Query<(Entity, &Position), With<FoodSource>>, // Query food sources
    frame_counter: Res<FrameCounter>,
) {
    // Collect ant states
    let ant_states: Vec<AntExportState> = query_ants
        .iter()
        .map(|(_entity, id_ref, pos_ref, state_ref)| AntExportState {
            id: id_ref.0 as u32, // Cast ParticleId(usize) to u32
            x: pos_ref.x,
            y: pos_ref.y,
            state: *state_ref, // Copy the state enum
        })
        .collect();

    // Get nest state (assuming one nest)
    let nest_state: Option<NestExportState> = query_nest.get_single().ok().map(|pos| NestExportState {
        x: pos.x,
        y: pos.y,
    });

    // Collect food source states
    let food_states: Vec<FoodSourceExportState> = query_food
        .iter()
        .map(|(entity, pos_ref)| FoodSourceExportState {
            id: entity.index(), // Use entity index as ID for food sources
            x: pos_ref.x,
            y: pos_ref.y,
        })
        .collect();

    // Update the resource with the new structure
    state_resource.0 = SimulationState {
        frame: frame_counter.count,
        timestamp: frame_counter.timestamp,
        ants: ant_states,
        nest: nest_state,
        food_sources: food_states,
        // particles: vec![], // Ensure old field is removed or empty if needed temporarily
    };

    // Optional: Log state update for debugging
    // log::debug!("Updated CurrentSimulationState for frame {}", frame_counter.count);
}
