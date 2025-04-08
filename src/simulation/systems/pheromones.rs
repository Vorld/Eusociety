//! Systems responsible for pheromone logic: deposit, decay, and following.

use bevy_ecs::prelude::*;
use glam::Vec2;
use rand::{thread_rng, Rng}; // For slight deposit randomization
use tracing::{warn, trace}; // Removed unused 'error' import

// Import simulation components including our custom Timer
use crate::simulation::components::{Position, Ant, AntState, Pheromone, PheromoneType, PheromoneInfluence, Timer};
// Import Time resource
use crate::simulation::spatial::{PheromoneQuadtree, Rect};

// Constants for Pheromone behavior
// TODO: Load these from config later
const PHEROMONE_DEPOSIT_INTERVAL_SECS: f32 = 1.0; // How often ants *can* deposit
const PHEROMONE_DEPOSIT_PROBABILITY: f64 = 0.9; // Chance to deposit each interval check
const PHEROMONE_INITIAL_STRENGTH: f32 = 1.0;
const PHEROMONE_DECAY_DURATION_SECS: f32 = 15.0; // Seconds until pheromone fully decays
const PHEROMONE_SENSE_RADIUS: f32 = 30.0; // How far ants can sense pheromones
// const PHEROMONE_INFLUENCE_WEIGHT: f32 = 1.5; // Weight applied in movement system

// --- Helper Resource for Deposit Timing ---

// Use a resource for the deposit interval timer
#[derive(Resource)]
pub struct PheromoneDepositTimer(Timer); // Use our Timer (Made public)

impl Default for PheromoneDepositTimer {
    fn default() -> Self {
        // Use the repeating constructor from our Timer
        Self(crate::simulation::components::Timer::from_seconds_repeating(PHEROMONE_DEPOSIT_INTERVAL_SECS))
    }
}

// --- Systems ---

/// Initializes the PheromoneDepositTimer resource.
pub fn setup_pheromone_timer(mut commands: Commands) {
    commands.init_resource::<PheromoneDepositTimer>();
}


/// System to handle ants depositing pheromones periodically.
pub fn pheromone_deposit_system(
    mut commands: Commands,
    ant_query: Query<(&Position, &AntState), With<Ant>>,
    mut pheromone_quadtree: ResMut<PheromoneQuadtree>,
    time: Res<crate::simulation::resources::Time>,
    mut deposit_timer: ResMut<PheromoneDepositTimer>,
    // config parameter removed
) {
    // world_bounds calculation removed
    deposit_timer.0.tick(time.delta_seconds); // Access field directly

    if deposit_timer.0.just_finished() {
        let mut rng = thread_rng();
        for (position, state) in ant_query.iter() {
            // Boundary check removed here - Quadtree insert handles it now with inclusive bounds.

            // Random chance to deposit to avoid perfect lines
            if rng.gen_bool(PHEROMONE_DEPOSIT_PROBABILITY) {
                let pheromone_type = match state {
                    AntState::Foraging => PheromoneType::HomeTrail, // Foraging ants leave trail TO home
                    AntState::ReturningToNest => PheromoneType::FoodTrail, // Returning ants leave trail TO food
                };

                // Spawn the pheromone entity
                let pheromone_entity = commands.spawn((
                    Pheromone {
                        type_: pheromone_type,
                        strength: PHEROMONE_INITIAL_STRENGTH,
                    },
                    *position, // Copy the ant's position
                    // Use the non-repeating constructor from our Timer
                    crate::simulation::components::Timer::from_seconds(PHEROMONE_DECAY_DURATION_SECS),
                )).id(); // Get the entity ID

                // Insert into quadtree - insert doesn't return bool, internal logic handles warnings
                pheromone_quadtree.insert(pheromone_entity, *position);
                trace!(entity = ?pheromone_entity, ?position, ?pheromone_type, "Deposited pheromone.");
            }
        }
    }
}

/// System to handle pheromone decay and despawning.
pub fn pheromone_decay_system(
    mut commands: Commands,
    // Fully qualify Timer component in Query
    mut query: Query<(Entity, &mut Pheromone, &mut crate::simulation::components::Timer, &Position)>,
    mut pheromone_quadtree: ResMut<PheromoneQuadtree>,
    time: Res<crate::simulation::resources::Time>, // Fully qualify Time resource
) {
    // Use parallel iterator for potentially many pheromones
    // query.par_iter_mut().for_each(|(entity, mut pheromone, mut timer, position)| {
    //     timer.tick(time.delta());

    //     // Decrease strength based on timer progress (linear decay)
    //     pheromone.strength = PHEROMONE_INITIAL_STRENGTH * (1.0 - timer.fraction());

    //     if timer.finished() {
    //         // Use Commands to safely despawn and remove from quadtree
    //         // We need to collect removals first because Commands execution is deferred
    //         // However, direct despawn and quadtree removal *might* be okay if done carefully.
    //         // Let's try direct removal first for simplicity, but be aware of potential issues.

    //         // Remove from quadtree *before* despawning
    //         if !pheromone_quadtree.remove(entity, position) {
    //             // This might happen if it was already removed or somehow outside bounds
    //             warn!(?entity, ?position, "Pheromone entity not found in quadtree during decay removal.");
    //         } else {
    //              trace!(?entity, ?position, "Removed decayed pheromone from quadtree.");
    //         }

    //         // Despawn the entity - needs access to Commands, cannot do in par_iter_mut directly
    //         // commands.entity(entity).despawn(); // This won't work here

    //         // --- Alternative: Collect entities to despawn ---
    //         // Need a way to communicate back to Commands.
    //         // For now, let's stick to single-threaded iteration for despawning.
    //     }
    // });

    // --- Single-threaded despawn loop ---
    // This is less efficient but safer with Commands.
    let mut entities_to_despawn = Vec::new(); // Collect entities to despawn
    for (entity, mut pheromone, mut timer, position) in query.iter_mut() {
         // Tick timer first
         timer.tick(time.delta_seconds); // Access field directly

         // Update strength based on timer progress (linear decay)
         pheromone.strength = PHEROMONE_INITIAL_STRENGTH * (1.0 - timer.fraction());

         if timer.finished() {
             // Attempt to remove from quadtree. Failure is not critical here, just log.
             if !pheromone_quadtree.remove(entity, position) {
                 warn!(?entity, ?position, "Pheromone entity not found in quadtree during decay removal (single-threaded).");
             } else {
                 trace!(?entity, ?position, "Removed decayed pheromone from quadtree.");
             }
             // Add to list for despawning after the loop
             entities_to_despawn.push(entity);
         }
    }

    // Despawn collected entities
    for entity in entities_to_despawn {
        commands.entity(entity).despawn();
        trace!(?entity, "Despawned decayed pheromone entity.");
    }

     // TODO: Revisit parallelization strategy for decay if performance becomes an issue.
     // Using `Commands.add` with a custom command might be cleaner.
}


/// System for ants to calculate influence from nearby pheromones.
pub fn pheromone_follow_system(
    mut ant_query: Query<(Entity, &Position, &AntState, &mut PheromoneInfluence), With<Ant>>,
    pheromone_query: Query<&Pheromone>, // Query to get pheromone data after lookup
    pheromone_quadtree: Res<PheromoneQuadtree>,
) {
    ant_query.par_iter_mut().for_each(|(ant_entity, ant_pos, ant_state, mut influence)| {
        // 1. Reset influence for this frame
        influence.vector = Vec2::ZERO;

        // 2. Define query range around the ant
        let query_rect = Rect::new(
            ant_pos.x - PHEROMONE_SENSE_RADIUS,
            ant_pos.y - PHEROMONE_SENSE_RADIUS,
            ant_pos.x + PHEROMONE_SENSE_RADIUS,
            ant_pos.y + PHEROMONE_SENSE_RADIUS,
        );

        // 3. Query the PheromoneQuadtree
        let nearby_pheromones = pheromone_quadtree.query_range(&query_rect);

        if nearby_pheromones.is_empty() {
            return; // No pheromones nearby, nothing to do
        }

        // 4. Calculate resultant vector
        let mut resultant_vector = Vec2::ZERO;
        let ant_vec2 = ant_pos.as_vec2(); // Convert ant position once

        for &(pheromone_entity, pheromone_pos) in nearby_pheromones {
             // Avoid self-influence if an ant somehow queried itself (unlikely here)
             // if ant_entity == pheromone_entity { continue; }

            // Get Pheromone component data
            if let Ok(pheromone) = pheromone_query.get(pheromone_entity) {
                // Determine relevant pheromone type based on ant state
                let target_pheromone_type = match ant_state {
                    AntState::Foraging => PheromoneType::FoodTrail, // Foraging ants follow food trails
                    AntState::ReturningToNest => PheromoneType::HomeTrail, // Returning ants follow home trails
                };

                // Check if the pheromone is the type the ant is interested in
                if pheromone.type_ == target_pheromone_type {
                    let pheromone_vec2 = pheromone_pos.as_vec2();
                    let direction = pheromone_vec2 - ant_vec2; // Vector from ant TO pheromone
                    let distance_sq = direction.length_squared();

                    // Avoid division by zero and ignore very close pheromones (optional)
                    if distance_sq > 5.0 {
                        // Weight by strength and inverse distance squared (dampened)
                        // Adding 1.0 to the denominator prevents division by zero and reduces
                        // the overwhelming influence of extremely close pheromones.
                        let weight = pheromone.strength / (distance_sq + 1.0);
                        // Use the *normalized* direction vector scaled by the dampened weight.
                        // Each pheromone contributes direction based on its weighted influence,
                        // preventing close pheromones from dominating solely due to vector magnitude.
                        resultant_vector += direction.normalize_or_zero() * weight;
                    }
                }
            } else {
                // This could happen if a pheromone was despawned between quadtree query and component lookup
                warn!(?pheromone_entity, "Failed to get Pheromone component for entity found in quadtree query.");
            }
        }

        // 5. Normalize the resultant vector (Optional: Removing this makes magnitude depend on strength/density)
        // resultant_vector = resultant_vector.normalize_or_zero();

        // 6. Inversion Logic for ReturningToNest state
        if *ant_state == AntState::ReturningToNest {
            // Ants returning home follow HomeTrail pheromones, but move *away* from their source direction
            // The calculated vector points towards HomeTrail pheromones. Inverting it points away.
            // resultant_vector = -resultant_vector;
        }

        // 7. Store the final influence vector
        influence.vector = resultant_vector;

        if influence.vector != Vec2::ZERO {
             trace!(?ant_entity, state = ?ant_state, influence = ?influence.vector, "Calculated pheromone influence");
        }
    });
}