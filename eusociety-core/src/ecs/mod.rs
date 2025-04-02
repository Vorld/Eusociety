// Expose the ECS module structure
pub mod system;
pub mod scheduler;

// Re-export key types for convenience
pub use system::{System, SystemAccess, DataAccess, AccessType};
pub use scheduler::{SystemRegistry, SystemScheduler};