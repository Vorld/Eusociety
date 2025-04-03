use bevy_ecs::prelude::*;
use crate::simulation::components::{Position, Velocity};
use crate::simulation::resources::Time;

/// System for updating particle positions based on their velocities
pub fn move_particles(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    for (mut position, velocity) in query.iter_mut() {
        position.x += velocity.dx * time.delta_seconds;
        position.y += velocity.dy * time.delta_seconds;
    }
}