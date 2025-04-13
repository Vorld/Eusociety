//! Defines the Bevy ECS components used in the simulation.
//! Components represent the data associated with each simulated entity (particle).

use bevy_ecs::component::Component;
use glam::Vec2; // Added for PheromoneInfluence
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

impl Position {
    /// Converts the Position component to a glam::Vec2.
    #[inline]
    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
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

/// Component identifying an entity as an Ant and tracking its state relevant to pheromone deposition.
#[derive(Component, Debug, Clone, Copy)]
pub struct Ant {
    /// Time elapsed (in seconds) since the ant last visited its relevant source
    /// (Nest for Foraging state, FoodSource for ReturningToNest state).
    pub time_since_last_source: f32,
}

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


// --- Pheromone Components ---

/// The type of pheromone trail.
/// Derives Serialize for sending state to the frontend.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PheromoneType {
    /// Trail leading towards food, dropped by ants returning to nest.
    FoodTrail,
    /// Trail leading towards the nest, dropped by foraging ants.
    HomeTrail,
}

/// Represents a pheromone deposit in the environment.
/// Entities with this component also need `Position` and `Timer`.
/// Derives Serialize for sending state to the frontend.
#[derive(Component, Debug, Clone, Copy, Serialize)]
pub struct Pheromone {
    pub type_: PheromoneType,
    pub strength: f32,
}

/// Stores the calculated influence vector from nearby pheromones on an ant.
/// This is used by the movement system to adjust the ant's direction.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PheromoneInfluence {
    pub vector: Vec2,
}

// --- Utility Components ---

/// A simple timer component for tracking durations.
#[derive(Component, Debug, Clone, Copy)]
pub struct Timer {
    /// Total duration of the timer in seconds.
    duration: f32,
    /// Time elapsed since the timer started or was last reset, in seconds.
    elapsed: f32,
    /// Whether the timer repeats after finishing.
    repeating: bool,
    /// Whether the timer has finished during the last tick.
    just_finished: bool,
}

impl Timer {
    /// Creates a new timer that runs once.
    pub fn from_seconds(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            repeating: false,
            just_finished: false,
        }
    }

    /// Creates a new timer that repeats.
    pub fn from_seconds_repeating(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            repeating: true,
            just_finished: false,
        }
    }

    /// Ticks the timer by the given delta time (in seconds).
    pub fn tick(&mut self, delta_seconds: f32) {
        self.elapsed += delta_seconds;
        self.just_finished = false; // Reset flag

        if self.elapsed >= self.duration {
            self.just_finished = true;
            if self.repeating {
                // Calculate how many times the timer finished and wrap around
                let overshoot = self.elapsed - self.duration;
                self.elapsed = overshoot % self.duration;
            } else {
                // Clamp elapsed time to duration if not repeating
                self.elapsed = self.duration;
            }
        }
    }

    /// Returns `true` if the timer finished during the last call to `tick`.
    pub fn just_finished(&self) -> bool {
        self.just_finished
    }

    /// Returns `true` if the timer has finished (elapsed >= duration).
    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Returns the fraction of the timer's duration that has elapsed (0.0 to 1.0).
    pub fn fraction(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Returns the remaining time in seconds.
    pub fn remaining(&self) -> f32 {
        (self.duration - self.elapsed).max(0.0)
    }
}
