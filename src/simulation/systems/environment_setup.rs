//! System responsible for setting up the initial simulation environment.
//! Spawns the nest and initial food sources.

use bevy_ecs::prelude::*;
use crate::simulation::components::{Nest, FoodSource, Position};
use crate::simulation::resources::SimulationConfigResource;
use rand::{thread_rng, Rng}; // Import rand for random positions

/// System that runs once at startup to create the nest and food sources.
pub fn setup_environment_system(
    mut commands: Commands,
    simulation_config: Res<SimulationConfigResource>,
) {
    // Get world dimensions and food count from config
    let (world_width, world_height) = simulation_config.0.world_dimensions;
    let food_count = simulation_config.0.food_sources_count;
    
    // Spawn the Nest at the center
    commands.spawn((
        Nest,
        Position { x: 25.0, y: 25.0 },
    ));

    // Calculate safe spawn area (80% of world size to keep food away from edges)
    let safe_min_width = world_width * 0.8;
    let safe_min_height = world_height * 0.8;
    let safe_max_width = world_width * 0.9;
    let safe_max_height = world_height * 0.9;
    
    // Spawn initial Food Sources randomly within world boundaries
    let mut rng = thread_rng();
    for _ in 0..food_count {
        let x = rng.gen_range(safe_min_width..=safe_max_width);
        let y = rng.gen_range(safe_min_height..=safe_max_height);
        commands.spawn((
            FoodSource,
            Position { x, y },
        ));
    }
}
