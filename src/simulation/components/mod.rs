//! Defines the Bevy ECS components used in the simulation.
//! Components represent the data associated with each simulated entity (particle).

use bevy_ecs::component::Component;
use serde::Serialize;

/// Component representing the 2D position (x, y coordinates) of a particle.
/// Automatically derives `Serialize` for transport purposes.
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Position {
    /// The x-coordinate.
    pub x: f32,
    /// The y-coordinate.
    pub y: f32,
}

/// Component representing the 2D velocity (dx, dy) of a particle.
/// `dx` is the change in x per second, `dy` is the change in y per second.
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    /// The velocity component along the x-axis.
    pub dx: f32,
    /// The velocity component along the y-axis.
    pub dy: f32,
}

/// Component representing a unique identifier for each particle.
/// Uses a `usize` internally but might be serialized as `u32` for transport.
/// Automatically derives `Serialize` for transport purposes.
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct ParticleId(
    /// The unique ID value.
    pub usize
);
