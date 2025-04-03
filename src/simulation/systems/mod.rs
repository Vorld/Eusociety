pub mod movement;
pub mod randomization;
pub mod boundary;
pub mod setup;

// Re-export system functions for easier access
pub use movement::move_particles;
pub use randomization::randomize_velocities;
pub use boundary::handle_boundaries;
pub use setup::spawn_particles;