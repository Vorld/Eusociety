use eusociety_config::{load_config, parse_position_component, ConfigError}; // Removed unused Config
use eusociety_core::{Scheduler, World};
use eusociety_simulation::random_movement_system;
use eusociety_transport::{create_sender, create_serializer, TransportError}; // Removed unused Sender, Serializer traits (using Box<dyn Trait>)
use log::{error, info, warn}; // Using log crate
use spin_sleep; // For accurate sleeping
use std::error::Error;
use std::process::exit;
// Removed unused std::thread
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
enum RunnerError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    #[error("Transport initialization error: {0}")]
    Transport(#[from] TransportError),
    #[error("Component parsing error: {0}")]
    ComponentParse(String),
    #[error("Runtime transport error: {0}")]
    RuntimeTransport(TransportError), // Separate variant for errors within the loop
}

fn main() {
    // Initialize logger (optional, but helpful)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Eusociety Simulation Runner...");

    if let Err(e) = run_simulation() {
        error!("Simulation failed: {}", e);
        // Print cause chain if available
        let mut source = e.source();
        while let Some(cause) = source {
            error!("  Caused by: {}", cause);
            source = cause.source();
        }
        exit(1);
    }

    info!("Simulation finished successfully.");
}

fn run_simulation() -> Result<(), RunnerError> {
    // 1. Load Configuration
    // TODO: Make config path configurable via command line args
    let config_path = "config-json.json";
    info!("Loading configuration from: {}", config_path);
    let config = load_config(config_path)?;
    info!(
        "Config loaded: FPS={}, Threads={}, Transport={}/{}",
        config.simulation.fps,
        config.simulation.threads, // Note: M1 uses single thread regardless
        config.transport.serializer.type_,
        config.transport.sender.type_
    );

    // 2. Initialize World
    let mut world = World::new();
    info!(
        "Initializing world with {} entities from config...",
        config.start_state.entities.len()
    );
    for entity_config in &config.start_state.entities {
        // For M1, we only expect the 'position' component
        if let Some(pos_value) = entity_config.components.get("position") {
            let position = parse_position_component(pos_value)
                .map_err(|e| RunnerError::ComponentParse(e.to_string()))?;
            world.add_entity_with_position(entity_config.id, position);
        } else {
            warn!(
                "Entity {} in config is missing 'position' component, skipping.",
                entity_config.id
            );
        }
    }
    info!("World initialized.");

    // 3. Initialize Scheduler
    let mut scheduler = Scheduler::new();
    // Add systems defined for the simulation
    scheduler.add_system(random_movement_system);
    info!("Scheduler initialized with systems.");

    // 4. Initialize Transport
    info!("Initializing transport...");
    let serializer = create_serializer(&config.transport.serializer.type_)?;
    let mut sender = create_sender(
        &config.transport.sender.type_,
        &config.transport.sender.options,
    )?;
    info!("Transport initialized.");

    // 5. Simulation Loop
    let target_fps = config.simulation.fps;
    let target_frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
    info!(
        "Starting simulation loop (Target FPS: {}, Target Frame Time: {:?})",
        target_fps, target_frame_duration
    );

    let mut frame_count: u64 = 0;
    let simulation_start_time = Instant::now();
    let mut last_log_time = Instant::now();

    // For simplicity in M1, run for a fixed number of frames or duration
    let max_frames = target_fps * 10; // Run for 10 seconds

    loop {
        let frame_start_time = Instant::now();

        // --- Run Systems ---
        scheduler.run(&mut world);

        // --- Transport Data ---
        let serialized_data = serializer
            .serialize(&world)
            .map_err(RunnerError::RuntimeTransport)?;

        sender
            .send(&serialized_data)
            .map_err(RunnerError::RuntimeTransport)?;

        // --- Frame Pacing ---
        let elapsed_time = frame_start_time.elapsed();

        if let Some(sleep_duration) = target_frame_duration.checked_sub(elapsed_time) {
            if !sleep_duration.is_zero() {
                // Use spin_sleep for potentially more accurate short sleeps
                spin_sleep::sleep(sleep_duration);
            }
        } else {
            warn!(
                "Frame {} took longer than target time: {:?} >= {:?}",
                frame_count, elapsed_time, target_frame_duration
            );
        }

        frame_count += 1;

        // --- Logging & Exit Condition ---
        let now = Instant::now();
        if now.duration_since(last_log_time) >= Duration::from_secs(1) {
            let total_elapsed = simulation_start_time.elapsed().as_secs_f64();
            let avg_fps = frame_count as f64 / total_elapsed;
            info!(
                "Frame: {}, Elapsed Time: {:.2}s, Avg FPS: {:.2}",
                frame_count, total_elapsed, avg_fps
            );
            last_log_time = now;
        }

        // Convert max_frames to u64 for comparison
        if frame_count >= max_frames as u64 {
            info!("Reached max frames ({}), stopping simulation.", max_frames);
            break;
        }

        // Add a small yield to prevent pegging CPU if loop is too fast,
        // though spin_sleep should handle most cases.
        // thread::yield_now();
    }

    Ok(())
}
