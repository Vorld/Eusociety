//! Main executable entry point for the Eusociety simulation.
//!
//! This binary initializes logging, loads configuration, validates it,
//! and then starts the main simulation application loop.

use eusociety::config::ConfigLoader; // Use the loader for config operations
use eusociety::simulation::SimulationApp;
use tracing::info; // Use info macro directly
use tracing_subscriber::{fmt, EnvFilter};

/// Main function to run the simulation.
///
/// # Errors
///
/// Returns an error if:
/// - Logging setup fails.
/// - Configuration file (`config.json`) cannot be read or parsed.
/// - Configuration validation fails.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    // Reads log level directives from RUST_LOG environment variable
    // Defaults to `info` level for the `eusociety` crate if RUST_LOG is not set.
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("eusociety=info".parse()?)) 
        .init();

    info!("Starting Eusociety simulation...");

    // Load configuration from the specified file
    info!("Loading configuration from config.json...");
    let config = ConfigLoader::from_file("config.json")?;
    info!("Configuration loaded successfully.");
    
    // Validate the loaded configuration
    info!("Validating configuration...");
    ConfigLoader::validate(&config)?;
    info!("Configuration validated successfully.");
    
    // Initialize the simulation application with the loaded config
    info!("Initializing SimulationApp...");
    let mut app = SimulationApp::new(config);
    info!("SimulationApp initialized.");

    // Run the main simulation loop
    info!("Starting simulation run loop...");
    app.run(); // This function will block until the simulation stops or exits

    info!("Simulation run loop finished."); // This might not be reached if run indefinitely
    Ok(())
}
