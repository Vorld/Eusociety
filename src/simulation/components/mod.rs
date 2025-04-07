//! Defines the Bevy ECS components used in the simulation.
//! Components represent the data associated with each simulated entity (particle).

use bevy_ecs::component::Component;
use serde::Serialize; // Keep Serialize for components that need to be sent

/// Component representing the 2D position (x, y coordinates) of an entity.
/// Automatically derives `Serialize` for transport purposes.
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Position {
    /// The x-coordinate.
    pub x: f32,
    /// The y-coordinate.
    pub y: f32,
}

/// Component representing the 2D velocity (dx, dy) of an entity.
/// `dx` is the change in x per second, `dy` is the change in y per second.
#[derive(Component, Debug, Clone, Copy)] // No Serialize needed for Velocity if not sent directly
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

// --- Ant Simulation Components ---

/// Marker component identifying an entity as an Ant.
#[derive(Component, Debug, Clone, Copy)]
pub struct Ant;

/// Represents the behavioral state of an Ant.
/// Derives Serialize for sending state to the frontend.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AntState {
    /// Searching for food, potentially influenced by FoodTrail pheromones.
    Foraging,
    /// Carrying food back to the nest, potentially influenced by HomeTrail pheromones (or NestScent).
    ReturningToNest,
}

/// Marker component identifying the Nest entity.
/// Typically added to an entity that also has a `Position`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Nest;

/// Marker component identifying a Food Source entity.
/// Typically added to an entity that also has a `Position`.
#[derive(Component, Debug, Clone, Copy)]
pub struct FoodSource;
