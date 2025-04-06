pub mod components;
pub mod resources;
pub mod systems;

use std::time::{Duration, Instant};
use std::thread::sleep;
use tracing::{info, error, debug, trace, warn}; 

use bevy_ecs::prelude::*;
use rand;


use bevy_ecs::prelude::*; // Ensure ResMut, Res etc are available
use crate::config::Config;
use crate::transport::TransportController; // Keep this import
use self::resources::{Time, FrameCounter, SimulationConfigResource, TransportConfigResource, CurrentSimulationState};
use self::systems::{
    move_particles, randomize_velocities, handle_boundaries,
    update_current_simulation_state_resource, // Keep this import
    send_simulation_data_system, // Import the new system
    // Removed: extract_and_send, flush_transport, SimulationTimer, SimulationTransport
};
// Removed: use crate::simulation::systems::state_export::update_current_simulation_state_resource; // No longer needed as it's imported above

/// The main simulation application
pub struct SimulationApp {
    world: World,
    schedule: Schedule,
    // transport_controller: Option<TransportController>, // Removed field
    running: bool,
    config: Config, // Keep config for spawning particles etc.
}

impl SimulationApp {
    /// Create a new simulation app with the provided configuration
    pub fn new(config: Config) -> Self {
        let mut world = World::new();
        
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

        // Create schedule with systems
        let mut schedule = Schedule::default();

        // Add core simulation systems and the new transport system
        schedule.add_systems((
            move_particles,
            randomize_velocities,
            handle_boundaries,
            // Add the state export system to run after simulation logic
            update_current_simulation_state_resource.after(handle_boundaries),
            // Add the new transport system to run after state export
            send_simulation_data_system.after(update_current_simulation_state_resource),
        ));

        // Store particle count for initialization
        let particle_count = config.simulation.particle_count;
        
        // Create instance first
        let mut app = Self {
            world,
            schedule,
            // transport_controller, // Field removed
            running: false,
            config,
        };
        // Manually spawn particles before returning
        info!("Initializing simulation with {} particles...", particle_count);
        app.spawn_initial_particles();
        info!("Initialization complete.");

        app
    }

    /// Runs the simulation schedule once.
    /// Intended for benchmarking or step-by-step execution.
    pub fn run_schedule_once(&mut self) {
        self.schedule.run(&mut self.world);
    }

    /// Provides mutable access to the simulation world.
    /// Intended for benchmarking or advanced integration.
    pub fn get_world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    
    /// Spawn initial particles
    fn spawn_initial_particles(&mut self) {
        let config = &self.config.simulation;
        let (width, height) = config.world_dimensions;
        let max_vel = config.max_velocity;
        
        for i in 0..config.particle_count {
            self.world.spawn((
                components::ParticleId(i),
                components::Position {
                    x: rand::random::<f32>() * width,
                    y: rand::random::<f32>() * height,
                },
                components::Velocity {
                    dx: (rand::random::<f32>() - 0.5) * max_vel * 2.0,
                    dy: (rand::random::<f32>() - 0.5) * max_vel * 2.0,
                },
            ));
        }
    }
    
    /// Run the simulation
    pub fn run(&mut self) {
        self.running = true;
        
        // Main simulation loop
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
            
            // Run simulation systems (including the new transport system)
            self.schedule.run(&mut self.world);

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
    }
    
    /// Stop the simulation
    pub fn stop(&mut self) {
        self.running = false;
    }
}
