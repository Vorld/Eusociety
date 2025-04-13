//! System responsible for managing ant state transitions based on interactions.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Ant, AntState, Position, Nest}; // Removed unused FoodSource
use crate::simulation::spatial::{FoodQuadtree, Rect}; // Import Quadtree and Rect
use tracing::trace; // For logging state changes

const INTERACTION_RADIUS: f32 = 5.0; // How close an ant needs to be to interact
const INTERACTION_RADIUS_SQ: f32 = INTERACTION_RADIUS * INTERACTION_RADIUS; // Use squared distance

/// System that updates ant states based on proximity to food and the nest.
pub fn ant_state_machine_system(
    mut commands: Commands,
    mut query_ants: Query<(Entity, &Position, &mut AntState, &mut Ant)>, // Add &mut Ant
    // query_food: Query<(Entity, &Position), With<FoodSource>>, // REMOVED - Use Quadtree instead
    mut food_quadtree: ResMut<FoodQuadtree>, // ADDED - Quadtree resource (mutable for removal)
    query_nest: Query<&Position, With<Nest>>, // Assuming one nest
) {
    // Get nest position (assuming only one exists)
    let nest_position = match query_nest.get_single() {
        Ok(pos) => *pos,
        Err(_) => {
            // If no nest, ants can't return. Log error or handle gracefully.
            // For now, we just won't process ReturningToNest state changes.
            // Consider adding error logging here if needed.
            return; // Can't proceed without a nest
        }
    };

    // Use temporary vecs to store state changes to avoid mutable borrow conflicts
    let mut ants_found_food: Vec<(Entity, Entity, Position)> = Vec::new();
    let mut ants_reached_nest: Vec<Entity> = Vec::new(); // ADDED: Store ants reaching nest

    // Iterate immutably first to check states and collect changes
    for (ant_entity, ant_pos, ant_state, _ant) in query_ants.iter() {
        match *ant_state {
            AntState::Foraging => {
                // Define the query area around the ant
                let query_rect = Rect::new(
                    ant_pos.x - INTERACTION_RADIUS,
                    ant_pos.y - INTERACTION_RADIUS,
                    ant_pos.x + INTERACTION_RADIUS,
                    ant_pos.y + INTERACTION_RADIUS,
                );

                // Query the quadtree for nearby food
                let nearby_food = food_quadtree.query_range(&query_rect); // Type: Vec<&(Entity, Position)>

                let mut closest_food_dist_sq = f32::MAX;
                // Explicitly type the Option
                let mut closest_food_in_range: Option<(Entity, Position)> = None;

                // Iterate through potential candidates from the quadtree query
                // nearby_food.iter() yields &&(Entity, Position)
                for food_data_ref in nearby_food.iter() {
                    // Dereference twice to get the actual tuple (Entity, Position)
                    // Explicitly type the result of the dereference
                    let (food_entity, food_pos): (Entity, Position) = **food_data_ref;

                    let dist_sq = distance_squared(ant_pos, &food_pos); // Pass food_pos by reference

                    // Check if it's within the actual interaction radius AND closer than previous finds
                    if dist_sq <= INTERACTION_RADIUS_SQ && dist_sq < closest_food_dist_sq {
                        closest_food_dist_sq = dist_sq;
                        // Assign the explicitly typed owned values
                        closest_food_in_range = Some((food_entity, food_pos));
                    }
                }

                // If we found a suitable food item
                if let Some((food_entity_to_take, food_pos_to_take)) = closest_food_in_range {
                    // Store the interaction details to process after the loop
                    ants_found_food.push((ant_entity, food_entity_to_take, food_pos_to_take));
                }
            }
            AntState::ReturningToNest => {
                // Check distance to nest (logic remains the same)
                let dist_to_nest_sq = distance_squared(ant_pos, &nest_position);
                if dist_to_nest_sq <= INTERACTION_RADIUS_SQ*50.0 {
                    // Store ant entity to change state after the loop
                    ants_reached_nest.push(ant_entity);
                }
            }
        }
    }

    // --- Process State Changes After Main Loop ---

    // Process the ants that found food
    for (ant_entity, food_entity, food_pos) in ants_found_food {
        // Attempt to remove the food from the quadtree
        // We check if removal is successful in case another ant grabbed it in the same frame
        if food_quadtree.remove(food_entity, &food_pos) {
            // If removal was successful, despawn the entity and update ant state
            commands.entity(food_entity).despawn();

            // Get the ant's state and timer mutably now
            if let Ok((_, _, mut state, mut ant)) = query_ants.get_mut(ant_entity) {
                *state = AntState::ReturningToNest;
                ant.time_since_last_source = 0.0; // Reset timer
                trace!(ant_id = ?ant_entity, food_id = ?food_entity, "Picked up food (Quadtree), state -> ReturningToNest");
            } else {
                 trace!(ant_id = ?ant_entity, food_id = ?food_entity, "Ant not found for state update after finding food?");
            }
        } else {
            // Food was already removed (likely by another ant this frame)
            trace!(ant_id = ?ant_entity, food_id = ?food_entity, "Attempted to pick up food already taken");
        }
    }

    // Process ants that reached the nest
    for ant_entity in ants_reached_nest {
        if let Ok((_, _, mut state, mut ant)) = query_ants.get_mut(ant_entity) {
            *state = AntState::Foraging;
            ant.time_since_last_source = 0.0; // Reset timer
            trace!(ant_id = ?ant_entity, "Reached nest, state -> Foraging");
        } else {
            trace!(ant_id = ?ant_entity, "Ant not found for state update after reaching nest?");
        }
    }
}

/// Helper function to calculate squared distance between two positions.
#[inline]
fn distance_squared(pos1: &Position, pos2: &Position) -> f32 {
    let dx = pos1.x - pos2.x;
    let dy = pos1.y - pos2.y;
    dx * dx + dy * dy
}
// --- New System for Updating Ant Timers ---

/// System to increment the `time_since_last_source` for all ants each frame.
/// This should run *before* pheromone deposition.
pub fn update_ant_timers_system(
    mut query_ants: Query<&mut Ant>,
    time: Res<crate::simulation::resources::Time>, // Use the fully qualified Time resource
) {
    let delta = time.delta_seconds; // Get delta time once
    // Consider parallelization if performance becomes an issue
    for mut ant in query_ants.iter_mut() {
        ant.time_since_last_source += delta;
    }
}
