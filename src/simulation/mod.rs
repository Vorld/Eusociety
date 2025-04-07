//! Manages the core simulation loop and integrates Bevy ECS.
//!
//! This module defines the main `SimulationApp` struct which orchestrates
//! the setup, execution, and teardown of the particle simulation using
//! Bevy's Entity Component System (ECS) framework.

pub mod components;
pub mod resources;
pub mod systems;

use std::time::{Duration, Instant};
use std::thread::sleep;
use tracing::{info, error, debug, trace, warn}; 

// Removed unused imports: bevy_ecs::prelude::*, rand
// `bevy_ecs::prelude::*` is imported again below, keeping that one.

use bevy_ecs::prelude::*; // Ensure ResMut, Res etc are available
use crate::config::Config;
use crate::transport::TransportController; // Keep this import
use self::resources::{Time, FrameCounter, SimulationConfigResource, TransportConfigResource, CurrentSimulationState};
use self::systems::{
    move_particles, randomize_velocities, handle_boundaries,
    update_current_simulation_state_resource, // Keep this import
    send_simulation_data_system, // Import the new system
    spawn_particles, // Import the setup system
    // Removed: extract_and_send, flush_transport, SimulationTimer, SimulationTransport
};
// Removed: use crate::simulation::systems::state_export::update_current_simulation_state_resource; // No longer needed as it's imported above

/// The main simulation application struct.
///
/// Encapsulates the Bevy ECS `World`, startup and update `Schedule`s,
/// and manages the simulation run loop.
pub struct SimulationApp {
    /// The Bevy ECS world containing all entities, components, and resources.
    world: World,
    /// Schedule for systems that run once at the beginning (e.g., spawning particles).
    startup_schedule: Schedule, 
    /// Schedule for systems that run every frame during the simulation update loop.
    update_schedule: Schedule,  
    /// Flag indicating whether the simulation loop is currently running.
    running: bool,
    /// A copy of the initial configuration used for setup and potentially during the run loop.
    config: Config, 
}

impl SimulationApp {
    /// Creates a new `SimulationApp` instance.
    ///
    /// Initializes the Bevy `World`, sets up resources (configuration, time, state, transport),
    /// creates the startup and update schedules, and adds the necessary systems.
    ///
    /// # Arguments
    ///
    /// * `config` - The simulation configuration loaded from `config.json`.
    pub fn new(config: Config) -> Self {
        let mut world = World::new();
        
        info!("Initializing simulation resources...");
        // Add configuration as resources
        world.insert_resource(SimulationConfigResource(config.simulation.clone()));
        world.insert_resource(TransportConfigResource(config.transport.clone()));
        
        // Add simulation resources
        world.insert_resource(Time::default());
        world.insert_resource(FrameCounter::default());
        // Removed: world.insert_resource(SimulationTimer::default());
        world.init_resource::<CurrentSimulationState>(); // Initialize new state resource

        // Create transport controller and insert as resource
        match TransportController::from_config(&config.transport) {
            Ok(controller) => {
                world.insert_resource(controller); // Insert as resource
            }
            Err(err) => {
                // Log error, but continue without transport if it fails
                error!("Failed to create transport controller: {}. Transport will be disabled.", err);
                // Optionally insert a default/null controller or handle differently
            }
        };

        // --- Create Schedules ---
        // Startup schedule for one-time setup systems
        let mut startup_schedule = Schedule::default();
        startup_schedule.add_systems(spawn_particles); // Add particle spawning system

        // Update schedule for systems that run every frame
        let mut update_schedule = Schedule::default();
        update_schedule.add_systems((
            move_particles,
            randomize_velocities,
            handle_boundaries,
            // Add the state export system to run after simulation logic
            update_current_simulation_state_resource.after(handle_boundaries),
            // Add the new transport system to run after state export
            send_simulation_data_system.after(update_current_simulation_state_resource),
        ));
        // --- End Schedule Creation ---

        // Create instance using the new schedules
        let app = Self {
            world,
            startup_schedule, // Use startup schedule
            update_schedule,  // Use update schedule
            // transport_controller, // Field removed
            running: false,
            config, // Keep config if needed elsewhere, e.g., in run loop
        };

        // No need to manually spawn particles here, startup schedule handles it.
        info!("SimulationApp created. Startup systems will run on first execution.");

        app
    }

    /// Runs the systems in the `update_schedule` exactly once.
    ///
    /// This is primarily intended for benchmarking specific systems or for
    /// step-by-step debugging or analysis of the simulation state.
    pub fn run_schedule_once(&mut self) {
        self.update_schedule.run(&mut self.world); 
    }

    /// Provides mutable access to the simulation's Bevy ECS `World`.
    ///
    /// This allows for direct manipulation or inspection of the world's state,
    /// typically used for advanced integration, testing, or benchmarking purposes.
    pub fn get_world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    
    // Removed the manual spawn_initial_particles method (now handled by startup schedule)
    
    /// Starts and runs the main simulation loop.
    ///
    /// This method first executes the `startup_schedule` once, then enters a loop
    /// that continues as long as the `running` flag is true. Inside the loop, it:
    /// 1. Calculates delta time.
    /// 2. Updates time and frame count resources.
    /// 3. Runs the `update_schedule`.
    /// 4. Performs periodic logging and debug output.
    /// 5. Sleeps to maintain the target frame rate defined in the configuration.
    pub fn run(&mut self) {
        if self.running {
            warn!("Simulation run() called while already running.");
            return;
        }
        self.running = true;
        info!("Simulation run loop starting...");
        // Run startup systems once
        info!("Running startup schedule...");
        self.startup_schedule.run(&mut self.world);
        info!("Startup schedule complete.");
        
        // Main simulation loop (using update schedule)
        let mut last_time = Instant::now();
        let frame_duration = Duration::from_secs_f64(1.0 / self.config.simulation.frame_rate as f64);
        let mut frame_counter = 0;
        
        // The new transport system handles sending data via the extract_and_send system,
        // so we don't need the manual collection logic anymore.
        
        while self.running {
            // Calculate delta time
            let now = Instant::now();
            let delta = now.duration_since(last_time);
            last_time = now;
            
            // Update simulation time and get elapsed time
            let elapsed_seconds = {
                let mut time = self.world.resource_mut::<Time>();
                time.delta_seconds = delta.as_secs_f32();
                time.elapsed_seconds += delta.as_secs_f64();
                time.elapsed_seconds
            };
            
            // Update frame counter with the elapsed time we just calculated
            {
                let mut frame_count = self.world.resource_mut::<FrameCounter>();
                frame_count.count += 1;
                frame_count.timestamp = elapsed_seconds;
            }
            
            // Run simulation systems (including the new transport system) via the update schedule
            self.update_schedule.run(&mut self.world);

            // --- Transport Logic Removed (Now handled by send_simulation_data_system) ---

            // Increment frame counter for debugging
            frame_counter += 1;
            
            // Add debug output every 100 frames
            if frame_counter % 100 == 0 {
                debug!(frame = frame_counter, timestamp = elapsed_seconds, "Simulation frame update");
                // Trace a sample of particle positions (lower level detail)
                let mut count = 0;
                for (id, pos) in self.world.query::<(&components::ParticleId, &components::Position)>().iter(&self.world).take(5) {
                    trace!(particle_id = id.0, x = pos.x, y = pos.y, "Particle position sample");
                    count += 1;
                }
                let total_particles = self.world.query::<&components::ParticleId>().iter(&self.world).count();
                if total_particles > count {
                    trace!(remaining = total_particles - count, "More particles exist");
                }

                // Debug connected WebSocket clients if using WebSocket transport
                // Access controller via world resource now
                if let Some(controller) = self.world.get_resource::<TransportController>() {
                    if let Some(ws_sender) = controller.get_websocket_sender() {
                        debug!(clients = ws_sender.client_count(), "WebSocket clients connected");
                    }
                } else {
                    // Optional: Log if controller resource is missing (e.g., due to init failure)
                    // trace!("TransportController resource not found for client count debug.");
                }
            }

            // Check for frame lag before sleeping
            let elapsed = now.elapsed();
            if elapsed > frame_duration {
                warn!(
                    target_duration_ms = frame_duration.as_millis(),
                    actual_duration_ms = elapsed.as_millis(),
                    lag_ms = (elapsed - frame_duration).as_millis(),
                    "Frame lag detected!"
                );
            }

            // Sleep to maintain frame rate (if needed)
            if elapsed < frame_duration {
                sleep(frame_duration - elapsed);
            }
        }
        info!("Simulation run loop finished.");
    }
    
    /// Stops the simulation run loop.
    ///
    /// Sets the `running` flag to false, causing the `run` method's loop
    /// to terminate after the current frame completes.
    pub fn stop(&mut self) {
        info!("Stopping simulation run loop...");
        self.running = false;
    }
}
