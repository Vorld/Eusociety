use bevy_ecs::prelude::*;
use rand::random;
use crate::simulation::components::{Position, Velocity, ParticleId};
use crate::simulation::resources::SimulationConfigResource;

/// System for spawning initial particles according to configuration
pub fn spawn_particles(
    mut commands: Commands,
    simulation_config: Res<SimulationConfigResource>,
) {
    let (width, height) = simulation_config.0.world_dimensions;
    let max_vel = simulation_config.0.max_initial_velocity;
    
    println!("Spawning {} particles...", simulation_config.0.particle_count);
    
    for i in 0..simulation_config.0.particle_count {
        commands.spawn((
            ParticleId(i),
            Position {
                x: random::<f32>() * width,
                y: random::<f32>() * height,
            },
            Velocity {
                dx: (random::<f32>() - 0.5) * max_vel * 2.0,
                dy: (random::<f32>() - 0.5) * max_vel * 2.0,
            },
        ));
    }
    
    println!("Particles spawned successfully");
}