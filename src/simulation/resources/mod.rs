use bevy_ecs::system::Resource;

/// Simulation frame counter
#[derive(Resource, Debug, Default)]
pub struct FrameCounter {
    pub count: u64,
    pub timestamp: f64,
}

/// Delta time for simulation updates
#[derive(Resource, Debug)]
pub struct Time {
    pub delta_seconds: f32,
    pub elapsed_seconds: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta_seconds: 1.0 / 60.0, // Default 60 FPS
            elapsed_seconds: 0.0,
        }
    }
}

/// SimulationConfig as a resource for systems
#[derive(Resource)]
pub struct SimulationConfigResource(pub crate::config::SimulationConfig);

/// TransportConfig as a resource for systems
#[derive(Resource)]
pub struct TransportConfigResource(pub crate::config::TransportConfig);