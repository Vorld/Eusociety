//! Defines Bevy ECS resources used throughout the simulation.
//! Resources are globally unique data structures accessible by systems.

use bevy_ecs::system::Resource;
use crate::transport::SimulationState; // Added import

/// Resource storing the current simulation frame number and the total elapsed time.
#[derive(Resource, Debug, Default)]
pub struct FrameCounter {
    /// The current simulation frame number, incremented each update cycle.
    pub count: u64,
    /// The total simulation time elapsed when this frame started, in seconds.
    pub timestamp: f64,
}

/// Resource storing simulation timing information.
#[derive(Resource, Debug)]
pub struct Time {
    /// Time elapsed since the previous frame's update, in seconds.
    pub delta_seconds: f32,
    /// Total time elapsed since the simulation started, in seconds.
    pub elapsed_seconds: f64,
}

impl Default for Time {
    /// Provides a default `Time` resource, assuming a 60 FPS target initially.
    fn default() -> Self {
        Self {
            delta_seconds: 1.0 / 60.0, // Default 60 FPS
            elapsed_seconds: 0.0,
        }
    }
}

/// Resource wrapper for the simulation-specific configuration (`SimulationConfig`).
/// Allows systems to access simulation parameters.
#[derive(Resource)]
pub struct SimulationConfigResource(
    /// The wrapped `SimulationConfig`.
    pub crate::config::SimulationConfig
);

/// Resource wrapper for the transport-specific configuration (`TransportConfig`).
/// Allows systems (especially transport-related ones) to access transport parameters.
#[derive(Resource)]
pub struct TransportConfigResource(
    /// The wrapped `TransportConfig`.
    pub crate::config::TransportConfig
);

/// Resource holding the latest snapshot of the complete simulation state (`SimulationState`),
/// intended to be serialized and sent by the transport layer.
/// This is typically updated once per frame after all simulation logic has run.
#[derive(Resource, Default, Clone, Debug)] // Added Debug derive for potential logging
pub struct CurrentSimulationState(
    /// The wrapped `SimulationState`.
    pub SimulationState
);
