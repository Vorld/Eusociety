pub mod components;
pub mod resources;
pub mod systems;

use std::time::{Duration, Instant};
use std::thread::sleep;

use bevy_ecs::prelude::*;
use rand;

use crate::config::Config;
use crate::transport::{TransportController, SimulationState, ParticleState};
use self::resources::{Time, FrameCounter, SimulationConfigResource, TransportConfigResource};
use self::systems::{spawn_particles, move_particles, randomize_velocities, handle_boundaries};

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
        
        // Create schedule with systems
        let mut schedule = Schedule::default();
        
        // Add systems to schedule
        schedule.add_systems((
            move_particles,
            randomize_velocities,
            handle_boundaries,
        ));
        
        // Create transport controller if needed
        let transport = match TransportController::from_config(&config.transport) {
            Ok(controller) => Some(controller),
            Err(err) => {
                eprintln!("Failed to create transport controller: {}", err);
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
        println!("Initializing simulation with {} particles...", particle_count);
        app.spawn_initial_particles();
        println!("Initialization complete.");
        
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
        let output_frequency = self.config.transport.output_frequency as u64;
        
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
            
            // We don't need RunState anymore since we're spawning particles manually
            
            // Run systems
            self.schedule.run(&mut self.world);
            
            // Send state to transport layer if configured and frame matches output frequency
            frame_counter += 1;
            
            // Add debug output every 100 frames
            if frame_counter % 100 == 0 {
                println!("Simulation frame: {}, timestamp: {:.2}s", frame_counter, elapsed_seconds);
                // Print a sample of particle positions
                let mut count = 0;
                for (id, pos) in self.world.query::<(&components::ParticleId, &components::Position)>().iter(&self.world).take(5) {
                    println!("  Particle {}: position ({:.2}, {:.2})", id.0, pos.x, pos.y);
                    count += 1;
                }
                println!("  ... and {} more particles", self.world.query::<&components::ParticleId>().iter(&self.world).count() - count);
            }
            
            // Collect state data if needed for transport (before borrowing transport)
            let state_data = if frame_counter % output_frequency == 0 && self.transport.is_some() {
                // Get frame info
                let frame_count = self.world.resource::<FrameCounter>().count;
                let timestamp = self.world.resource::<FrameCounter>().timestamp;
                
                // Collect particle data
                let mut particles = Vec::new();
                for (id, pos) in self.world.query::<(&components::ParticleId, &components::Position)>().iter(&self.world) {
                    particles.push(ParticleState {
                        id: id.0,
                        position: [pos.x, pos.y],
                    });
                }
                
                Some(SimulationState {
                    frame: frame_count,
                    timestamp,
                    particles,
                })
            } else {
                None
            };
            
            // Send state if we collected it
            if let Some(state) = state_data {
                if let Some(ref transport) = self.transport {
                    if let Err(err) = transport.send_state(&state) {
                        eprintln!("Failed to send state: {}", err);
                    }
                }
            }
            
            // Sleep to maintain frame rate
            let elapsed = now.elapsed();
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