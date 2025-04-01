// filepath: /home/ubuntu/projects/Eusociety/eusociety-core/src/scheduler.rs
use crate::system::System;
use crate::World;
use std::time::{Duration, Instant};

/// Simple scheduler for executing systems with basic timing and performance metrics
pub struct Scheduler {
    systems: Vec<Box<dyn System>>,
    fixed_timestep: Option<Duration>, // Optional fixed timestep for deterministic simulation
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            fixed_timestep: None,
        }
    }

    /// Add a system to the scheduler
    pub fn add_system<T: System + 'static>(&mut self, system: T) {
        self.systems.push(Box::new(system));
    }

    /// Set a fixed timestep for deterministic simulation
    pub fn with_fixed_timestep(&mut self, timestep_ms: u64) -> &mut Self {
        self.fixed_timestep = Some(Duration::from_millis(timestep_ms));
        self
    }

    /// Execute all systems once
    pub fn execute_once(&mut self, world: &mut World) {
        let start = Instant::now();
        
        for system in &mut self.systems {
            let system_start = Instant::now();
            system.run(world);
            let system_duration = system_start.elapsed();
            
            // In a more advanced version, we could log performance metrics
            // or dynamically reorder systems based on execution time
        }
        
        let total_duration = start.elapsed();
        
        // If using fixed timestep, sleep if we finished early
        if let Some(target_duration) = self.fixed_timestep {
            if total_duration < target_duration {
                std::thread::sleep(target_duration - total_duration);
            }
        }
    }

    /// Run systems in a loop until stopped
    pub fn run_loop(&mut self, world: &mut World) {
        let mut running = true;
        
        while running {
            self.execute_once(world);
            
            // In a more complex version, we might check for stop signals
            // or implement a proper game loop with input handling
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}