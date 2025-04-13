//! Contains the Bevy system responsible for collecting the current simulation state
//! into a resource for transport.

use bevy_ecs::prelude::*;
// Import Ant, Nest, Food, and Pheromone components
use crate::simulation::components::{ParticleId, Position, Ant, AntState, Nest, FoodSource, Pheromone};
use crate::simulation::resources::{CurrentSimulationState, FrameCounter, WallGeometry}; // Import WallGeometry
// Import specific export state structs and the main SimulationState
use crate::transport::{AntExportState, NestExportState, FoodSourceExportState, PheromoneExportState, SimulationState}; // Added PheromoneExportState

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
/// * `query_pheromones` - Query for pheromone entities and their data.
/// * `frame_counter` - The `FrameCounter` resource providing the current frame number and timestamp.
pub fn update_current_simulation_state_resource(
    mut state_resource: ResMut<CurrentSimulationState>,
    query_ants: Query<(Entity, &ParticleId, &Position, &AntState), With<Ant>>,
    query_nest: Query<&Position, With<Nest>>,
    query_food: Query<(Entity, &Position), With<FoodSource>>,
    query_pheromones: Query<(Entity, &Position, &Pheromone)>, // Added query for pheromones
    frame_counter: Res<FrameCounter>,
    wall_geometry: Res<WallGeometry>, // Add WallGeometry resource parameter
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

    // Collect pheromone states
    let pheromone_states: Vec<PheromoneExportState> = query_pheromones
        .iter()
        .map(|(entity, pos_ref, pheromone_ref)| PheromoneExportState {
            id: entity.index(), // Use entity index as ID
            x: pos_ref.x,
            y: pos_ref.y,
            type_: pheromone_ref.type_, // Copy the type enum
            strength: pheromone_ref.strength,
        })
        .collect();

    // Update the resource with the new structure including pheromones
    state_resource.0 = SimulationState {
        frame: frame_counter.count,
        timestamp: frame_counter.timestamp,
        ants: ant_states,
        nest: nest_state,
        food_sources: food_states,
        pheromones: pheromone_states,
        walls: wall_geometry.polygons.clone(), // Clone wall data into the state
    };

    // Optional: Log state update for debugging
    // log::debug!("Updated CurrentSimulationState for frame {}", frame_counter.count);
}
