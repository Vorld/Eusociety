use bevy_ecs::prelude::*;
use crate::simulation::components::{Position, Velocity};
use crate::simulation::resources::Time;

/// System for updating particle positions based on their velocities
pub fn move_particles(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    // Using par_for_each for parallel iteration over all particles
    query.par_iter_mut().for_each(|(mut position, velocity)| {
        position.x += velocity.dx * time.delta_seconds;
        position.y += velocity.dy * time.delta_seconds;
    });
}