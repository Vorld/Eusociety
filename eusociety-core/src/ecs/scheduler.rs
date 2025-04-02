use crate::ecs::system::{System, SystemAccess};

/// Registry for storing and managing systems
#[derive(Default)]
pub struct SystemRegistry {
    /// Systems stored in the registry
    systems: Vec<Box<dyn System>>,
    /// Access patterns for each system
    access_patterns: Vec<SystemAccess>,
}

impl SystemRegistry {
    /// Creates a new empty system registry
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Adds a system to the registry
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    /// 
    /// # Returns
    /// 
    /// True if the system was added successfully, false if it conflicts with existing systems
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> bool {
        let system_access = system.access();
        
        // Check for conflicts with existing systems
        for existing_access in &self.access_patterns {
            if system_access.conflicts_with(existing_access) {
                // Found a conflict, don't add the system
                return false;
            }
        }
        
        // No conflicts, add the system
        self.access_patterns.push(system_access);
        self.systems.push(Box::new(system));
        
        true
    }
    
    /// Forcefully adds a system regardless of conflicts
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    pub fn add_system_unchecked<S: System + 'static>(&mut self, system: S) {
        let system_access = system.access();
        self.access_patterns.push(system_access);
        self.systems.push(Box::new(system));
    }
    
    /// Runs all systems in the registry
    /// 
    /// # Arguments
    /// 
    /// * `world` - The world to run the systems on
    pub fn run_systems(&mut self, world: &mut crate::World) {
        for system in &mut self.systems {
            system.run(world);
        }
    }
    
    /// Returns the number of systems in the registry
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
    
    /// Checks if two systems have conflicting access patterns
    pub fn systems_conflict(system1: &dyn System, system2: &dyn System) -> bool {
        let access1 = system1.access();
        let access2 = system2.access();
        
        access1.conflicts_with(&access2)
    }
}

/// Enhanced scheduler that uses the system registry
#[derive(Default)]
pub struct SystemScheduler {
    /// Registry storing all systems
    registry: SystemRegistry,
}

impl SystemScheduler {
    /// Creates a new empty scheduler
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Adds a system to the scheduler if it doesn't conflict with existing systems
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    /// 
    /// # Returns
    /// 
    /// True if the system was added, false if it conflicts
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> bool {
        self.registry.add_system(system)
    }
    
    /// Forcefully adds a system regardless of conflicts
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    pub fn add_system_unchecked<S: System + 'static>(&mut self, system: S) {
        self.registry.add_system_unchecked(system)
    }
    
    /// Runs all systems in the scheduler
    /// 
    /// # Arguments
    /// 
    /// * `world` - The world to run the systems on
    pub fn run(&mut self, world: &mut crate::World) {
        self.registry.run_systems(world);
    }
    
    /// Returns the number of systems in the scheduler
    pub fn system_count(&self) -> usize {
        self.registry.system_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::system::{System, SystemAccess, DataAccess, AccessType};
    use crate::{Position, World};
    use std::any::TypeId;

    // Mock system that reads Position
    struct PositionReaderSystem;
    
    impl System for PositionReaderSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new()
                .with_component(TypeId::of::<Position>(), AccessType::Read)
        }
        
        fn run(&mut self, world: &mut World) {
            // Just read positions
            for (_, pos) in world.components.query::<Position>() {
                let _ = pos.x;
            }
        }
    }
    
    // Mock system that writes Position
    struct PositionWriterSystem;
    
    impl System for PositionWriterSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new()
                .with_component(TypeId::of::<Position>(), AccessType::Write)
        }
        
        fn run(&mut self, world: &mut World) {
            // Modify positions
            for (_, pos) in world.components.query_mut::<Position>() {
                pos.x += 1.0;
            }
        }
    }
    
    #[test]
    fn test_system_conflict_detection() {
        let reader = PositionReaderSystem;
        let writer = PositionWriterSystem;
        let writer2 = PositionWriterSystem;
        
        // A reader and writer should conflict
        assert!(SystemRegistry::systems_conflict(&reader, &writer));
        
        // Two writers should conflict
        assert!(SystemRegistry::systems_conflict(&writer, &writer2));
        
        // Two readers should not conflict
        assert!(!SystemRegistry::systems_conflict(&reader, &reader));
    }
    
    #[test]
    fn test_registry_conflict_prevention() {
        let mut registry = SystemRegistry::new();
        
        // First system should be added successfully
        assert!(registry.add_system(PositionWriterSystem));
        assert_eq!(registry.system_count(), 1);
        
        // Second conflicting system should fail to be added
        assert!(!registry.add_system(PositionWriterSystem));
        assert_eq!(registry.system_count(), 1);
        
        // Non-conflicting system should be added
        // (in reality, this would be a different type of system)
    }
    
    #[test]
    fn test_scheduler() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position { x: 0.0, y: 0.0 });
        
        let mut scheduler = SystemScheduler::new();
        scheduler.add_system(PositionWriterSystem);
        
        scheduler.run(&mut world);
        
        // Position should be modified
        assert_eq!(world.get_component::<Position>(entity).unwrap().x, 1.0);
    }
}