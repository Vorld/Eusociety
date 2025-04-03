use bevy_ecs::component::Component;
use serde::Serialize;

/// Position component for particles in 2D space
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// Velocity component for particles in 2D space
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

/// Unique identifier for particles
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct ParticleId(pub usize);