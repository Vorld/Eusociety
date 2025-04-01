// eusociety-core: Core ECS Implementation

pub mod component;
pub mod entity;
pub mod world;
pub mod system;
pub mod scheduler; // Basic single-threaded scheduler for M1

// Re-export key types
pub use component::Component;
pub use entity::Entity;
pub use entity::EntityManager;
pub use world::World;
pub use system::System;

// Placeholder for now
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
