use eusociety::config::{Config, ConfigLoader, ConfigError};
use eusociety::simulation::SimulationApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ConfigLoader::from_file("config.json")?;
    
    // Validate configuration
    ConfigLoader::validate(&config)?;
    
    // Initialize and run simulation
    let mut app = SimulationApp::new(config);
    app.run();
    
    println!("Simulation completed successfully");
    Ok(())
}