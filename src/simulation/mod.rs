pub mod components;
pub mod resources;
pub mod systems;

use std::time::{Duration, Instant};
use std::thread::sleep;
use tracing::{info, error, debug, trace, warn}; // Added tracing import

use bevy_ecs::prelude::*;
use rand;

use crate::config::Config;
use crate::transport::TransportController; // Removed unused SimulationState, ParticleState
use self::resources::{Time, FrameCounter, SimulationConfigResource, TransportConfigResource};
use self::systems::{
    // Removed unused spawn_particles
    move_particles, randomize_velocities, handle_boundaries, 
    extract_and_send, flush_transport, SimulationTimer, SimulationTransport
};

/// The main simulation application
pub struct SimulationApp {
    world: World,
    schedule: Schedule,
    transport: Option<TransportController>,
    running: bool,
    config: Config,
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
        world.insert_resource(SimulationTimer::default());
        
        // Create schedule with systems
        let mut schedule = Schedule::default();
        
        // Create transport controller
        let transport = match TransportController::from_config(&config.transport) {
            Ok(controller) => {
                // Add transport resource to world if created successfully
                let transport_controller = controller.clone();
                world.insert_resource(SimulationTransport { controller: transport_controller });
                
                // Add transport systems to schedule
                schedule.add_systems((
                    move_particles,
                    randomize_velocities,
                    handle_boundaries,
                    extract_and_send,
                    flush_transport.after(extract_and_send),
                ));
                
                Some(controller)
            },
            Err(err) => {
                error!("Failed to create transport controller: {}", err);

                // Add standard systems without transport
                schedule.add_systems((
                    move_particles,
                    randomize_velocities,
                    handle_boundaries,
                ));
                
                None
            }
        };
        
        // Store particle count for initialization
        let particle_count = config.simulation.particle_count;
        
        // Create instance first
        let mut app = Self {
            world,
            schedule,
            transport,
            running: false,
            config,
        };
        // Manually spawn particles before returning
        info!("Initializing simulation with {} particles...", particle_count);
        app.spawn_initial_particles();
        info!("Initialization complete.");

        app
    }
    
    /// Spawn initial particles
    fn spawn_initial_particles(&mut self) {
        let config = &self.config.simulation;
        let (width, height) = config.world_dimensions;
        let max_vel = config.max_initial_velocity;
        
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
            
            // Run systems (including transport systems if configured)
            self.schedule.run(&mut self.world);
            
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
                if let Some(transport) = &self.world.get_resource::<SimulationTransport>() {
                    if let Some(ws_sender) = transport.controller.get_websocket_sender() {
                        debug!(clients = ws_sender.client_count(), "WebSocket clients connected");
                    }
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

/// Resource to track simulation run state
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
enum RunState {
    Startup,
    Running,
}

impl RunState {
    fn is_startup(&self) -> bool {
        matches!(self, RunState::Startup)
    }
}
