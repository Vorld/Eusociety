use clap::Parser;
use eusociety_config::{load_config, Config, ConfigError, BehaviorType, SerializerType, SenderType};
use eusociety_core::{World, scheduler::Scheduler}; // Import Scheduler
use eusociety_simulation::{Position, Velocity, DeltaTime, WorldBounds, SpatialGridSystem, RandomMovementSystem, FlockingSystem}; // Import systems and resources
use eusociety_transport::{JsonSerializer, BinarySerializer, StdioSender, Serializer, Sender};

#[cfg(feature = "websocket")]
use eusociety_transport::WebSocketSender;

use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::process;
use rand::Rng;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the simulation configuration file
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
}

fn main() {
    println!("Eusociety Simulation Runner");
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Load configuration
    let config = match load_config(&args.config) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load config: {:?}", e);
            process::exit(1);
        }
    };

    println!("Using configuration from {}", args.config.display());
    
    // Initialize the world
    let mut world = World::new();

    // --- Add Core Resources ---
    let bounds = WorldBounds {
        min_x: -config.world_settings.width / 2.0,
        max_x: config.world_settings.width / 2.0,
        min_y: -config.world_settings.height / 2.0,
        max_y: config.world_settings.height / 2.0,
    };
    world.add_resource(bounds.clone()); // Add bounds resource

    // Calculate frame duration from config framerate
    let frame_duration = Duration::from_secs_f64(1.0 / config.framerate as f64);
    let initial_dt = frame_duration.as_secs_f32();
    world.add_resource(DeltaTime(initial_dt)); // Add initial DeltaTime

    // Initialize entities (remains similar)
    initialize_world(&mut world, &config);

    // --- Setup Scheduler and Systems ---
    let mut scheduler = Scheduler::new();
    scheduler.with_fixed_timestep(frame_duration.as_millis() as u64); // Use fixed timestep

    // Register SpatialGrid system (needed for flocking)
    scheduler.add_system(SpatialGridSystem::default());

    // Register behavior system based on first config entry (M1 simplification)
    if let Some(first_entity_config) = config.initial_state.first() {
        match first_entity_config.behavior {
            BehaviorType::Random => {
                println!("Using RandomMovementSystem");
                scheduler.add_system(RandomMovementSystem::default());
            }
            BehaviorType::Flocking => {
                println!("Using FlockingSystem");
                // Use defaults if flocking settings are missing in config
                let flock_settings = first_entity_config.get_flocking_settings().unwrap_or_else(|| {
                    println!("Flocking settings not found in config, using defaults.");
                    // Manually create default FlockingSettings if needed, assuming eusociety_config provides defaults
                    // If not, define them here or ensure eusociety_config has a Default impl
                     eusociety_config::FlockingSettings {
                         perception_radius: 10.0, // Example default
                         separation_weight: 0.5, // Example default
                         alignment_weight: 0.3, // Example default
                         cohesion_weight: 0.2, // Example default
                     }
                });
                 scheduler.add_system(FlockingSystem::new(
                    flock_settings.perception_radius,
                    flock_settings.separation_weight,
                    flock_settings.alignment_weight,
                    flock_settings.cohesion_weight,
                ));
            }
            _ => { // Default to random
                 println!("Defaulting to RandomMovementSystem for behavior type: {:?}", first_entity_config.behavior);
                 scheduler.add_system(RandomMovementSystem::default());
            }
        }
    } else {
        println!("No initial state defined, adding RandomMovementSystem by default.");
        scheduler.add_system(RandomMovementSystem::default());
    }

    // --- Setup Transport (remains similar) ---
    // Create the appropriate serializer based on config
    let serializer: Box<dyn Serializer> = create_serializer(&config);
    
    // Create the appropriate sender based on config
    let mut sender = create_sender(&config);

    println!("Running simulation at {} FPS...", config.framerate);
    match config.transport.sender.sender_type {
        SenderType::WebSocket => {
            #[cfg(feature = "websocket")]
            {
                let ws_options = config.transport.sender.get_websocket_options();
                println!("WebSocket server listening on ws://{}:{}", 
                    ws_options.host, ws_options.port);
                println!("Open the frontend HTML page to visualize the simulation");
            }
            #[cfg(not(feature = "websocket"))]
            {
                eprintln!("WebSocket sender configured but websocket feature is not enabled!");
                process::exit(1);
            }
        },
        SenderType::Stdio => println!("Sending simulation data to standard output"),
    }
    
    // Simulation loop
    loop {
        let frame_start = Instant::now();

        // Calculate actual delta time for this frame (using fixed duration for now)
        let delta_seconds = frame_duration.as_secs_f32();
        world.add_resource(DeltaTime(delta_seconds)); // Update DeltaTime resource

        // --- Execute Systems ---
        scheduler.execute_once(&mut world); // Run registered systems

        // --- Serialize and Send (remains similar) ---
        if let Ok(data) = serializer.serialize(&world) {
            // Send data
            if let Err(e) = sender.send(data.as_bytes()) {
                eprintln!("Error sending data: {:?}", e);
                // Consider breaking the loop or handling differently if send fails repeatedly
            }
        }

        // --- Frame Rate Control (remains similar) ---
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            sleep(frame_duration - elapsed);
        } else if config.framerate > 10 {
            // Only show warning if target framerate is high enough to matter
            eprintln!("Frame time exceeded budget: {:?} > {:?}", elapsed, frame_duration);
        }
    }
}

fn initialize_world(world: &mut World, config: &Config) {
    let mut rng = rand::thread_rng();

    // Get world bounds resource and clone values to release the borrow
    let (min_x, max_x, min_y, max_y) = {
        let bounds = world.get_resource::<WorldBounds>().expect("WorldBounds resource missing during initialization");
        (bounds.min_x, bounds.max_x, bounds.min_y, bounds.max_y)
    }; // Immutable borrow of world ends here

    // Initialize entities based on config
    for entity_config in &config.initial_state {
        println!("Creating {} '{}' entities", entity_config.count, entity_config.entity_type);
        
        for _ in 0..entity_config.count {
            let entity = world.create_entity();
            world.spawn(entity); // <--- Add this line to register the entity in the world's main map
            // Initialize with random position using cloned bounds
            let x = rng.gen_range(min_x..max_x);
            let y = rng.gen_range(min_y..max_y);
            world.add_component(entity, Position { x, y }); // Mutable borrow of world is now fine

            // Initialize with random velocity
            let speed = match entity_config.behavior {
                BehaviorType::Random => 5.0 + rng.gen::<f32>() * 5.0,
                BehaviorType::Flocking => {
                    if let Some(settings) = entity_config.get_flocking_settings() {
                        2.0 + rng.gen::<f32>() * 3.0
                    } else {
                        3.0
                    }
                },
                _ => 3.0,
            };
            
            let angle = rng.gen::<f32>() * std::f32::consts::TAU;
            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;

            world.add_component(entity, Velocity { x: vx, y: vy }); // Mutable borrow of world is also fine here
        }
    }
}

// Removed the old update_entities function and its helper

fn create_serializer(config: &Config) -> Box<dyn Serializer> {
    match config.transport.serializer.serializer_type {
        SerializerType::Json => Box::new(JsonSerializer),
        SerializerType::Binary => Box::new(BinarySerializer),
    }
}

#[allow(unused_variables)]
fn create_sender(config: &Config) -> Box<dyn Sender> {
    match config.transport.sender.sender_type {
        SenderType::Stdio => Box::new(StdioSender::new()),
        SenderType::WebSocket => {
            #[cfg(feature = "websocket")]
            {
                // Get WebSocket options from config
                let options = config.transport.sender.get_websocket_options();
                let mut ws_sender = WebSocketSender::new(&options.host, options.port);
                
                // Start the WebSocket server
                if let Err(e) = ws_sender.start() {
                    eprintln!("Failed to start WebSocket server: {:?}", e);
                    process::exit(1);
                }
                
                return Box::new(ws_sender);
            }
            
            #[cfg(not(feature = "websocket"))]
            {
                eprintln!("WebSocket sender configured but websocket feature is not enabled!");
                process::exit(1);
            }
        }
    }
}
