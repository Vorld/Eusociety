//! System responsible for managing ant state transitions based on interactions.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Ant, AntState, Position, FoodSource, Nest};
use tracing::trace; // For logging state changes

const INTERACTION_RADIUS: f32 = 5.0; // How close an ant needs to be to interact
const INTERACTION_RADIUS_SQ: f32 = INTERACTION_RADIUS * INTERACTION_RADIUS; // Use squared distance

/// System that updates ant states based on proximity to food and the nest.
pub fn ant_state_machine_system(
    mut commands: Commands,
    mut query_ants: Query<(Entity, &Position, &mut AntState), With<Ant>>,
    query_food: Query<(Entity, &Position), With<FoodSource>>,
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

    for (ant_entity, ant_pos, mut ant_state) in query_ants.iter_mut() {
        match *ant_state {
            AntState::Foraging => {
                let mut closest_food_dist_sq = f32::MAX;
                let mut closest_food_entity: Option<Entity> = None;

                // Find the closest food source
                for (food_entity, food_pos) in query_food.iter() {
                    let dist_sq = distance_squared(ant_pos, food_pos);
                    if dist_sq < closest_food_dist_sq {
                        closest_food_dist_sq = dist_sq;
                        closest_food_entity = Some(food_entity);
                    }
                }

                // Check if the closest food is within interaction radius
                if closest_food_dist_sq <= INTERACTION_RADIUS_SQ {
                    if let Some(food_entity_to_despawn) = closest_food_entity {
                        // Despawn the food source
                        commands.entity(food_entity_to_despawn).despawn();
                        // Change ant state
                        *ant_state = AntState::ReturningToNest;
                        trace!(ant_id = ?ant_entity, "Picked up food, state -> ReturningToNest");
                    }
                }
            }
            AntState::ReturningToNest => {
                // Check distance to nest
                let dist_to_nest_sq = distance_squared(ant_pos, &nest_position);
                if dist_to_nest_sq <= INTERACTION_RADIUS_SQ {
                    // Change ant state back to foraging
                    *ant_state = AntState::Foraging;
                     trace!(ant_id = ?ant_entity, "Reached nest, state -> Foraging");
                }
            }
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
