use eusociety::config::ConfigLoader; // Removed unused Config, ConfigError
use eusociety::simulation::SimulationApp;
use tracing_subscriber::{fmt, EnvFilter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("eusociety=info".parse()?)) // Default to info level for our crate
        .init();

    // Load configuration
    let config = ConfigLoader::from_file("config.json")?;
    
    // Validate configuration
    ConfigLoader::validate(&config)?;
    
    // Initialize and run simulation
    let mut app = SimulationApp::new(config);
    app.run();

    tracing::info!("Simulation completed successfully");
    Ok(())
}
