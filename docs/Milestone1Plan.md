# Milestone 1: Particle Simulation Implementation Plan

## 1. Project Setup & Infrastructure
*Workspace structure is already set up with crates. Focus on Milestone 1 requirements.*

### Dependencies Review
- [ ] Update workspace dependencies in `Cargo.toml`:
  - Add Bevy (`bevy = "0.12"`) to appropriate crates
  - Verify serde, rand, and other dependencies are properly configured

### Module Organization
- [ ] Ensure each crate has proper module organization:
  ```rust
  // Example structure for eusociety-simulation
  mod components;    // Position, Velocity components
  mod resources;     // Simulation parameters as resources
  mod systems;       // Movement and randomization systems
  pub mod prelude;   // Re-exports of commonly used items
  ```

## 2. Configuration Module Implementation (`eusociety-config`)

### Define Configuration Schema
- [ ] Create `SimulationConfig` struct with:
  ```rust
  pub struct SimulationConfig {
      pub particle_count: usize,        // Set default to 100
      pub world_dimensions: (f32, f32), // World bounds
      pub max_initial_velocity: f32,    // For random velocity generation
      pub velocity_randomization_factor: f32, // For smooth random changes
      pub boundary_behavior: BoundaryBehavior, // Enum: Wrap or Bounce
      pub frame_rate: u32,              // Target frames per second
  }
  
  pub enum BoundaryBehavior {
      Wrap,
      Bounce,
  }
  ```

- [ ] Create `TransportConfig` struct with:
  ```rust
  pub struct TransportConfig {
      pub serializer_type: SerializerType,
      pub output_path: String,          // File path for output
      pub output_frequency: u32,        // How often to output data (frames)
  }
  
  pub enum SerializerType {
      Json,
      Binary,
  }
  ```

- [ ] Create root `Config` struct:
  ```rust
  pub struct Config {
      pub simulation: SimulationConfig,
      pub transport: TransportConfig,
  }
  ```

### Create Default Configuration File
- [ ] Create `config.json` with 100 particles:
  ```json
  {
    "simulation": {
      "particle_count": 100,
      "world_dimensions": [1000.0, 1000.0],
      "max_initial_velocity": 5.0,
      "velocity_randomization_factor": 0.1,
      "boundary_behavior": "Wrap",
      "frame_rate": 60
    },
    "transport": {
      "serializer_type": "Json",
      "output_path": "output.json",
      "output_frequency": 10
    }
  }
  ```

### Implement Config Loading
- [ ] Create `ConfigLoader` with these methods:
  ```rust
  impl ConfigLoader {
      pub fn from_file(path: &str) -> Result<Config, ConfigError> {
          // Read file and deserialize JSON
      }
      
      pub fn validate(config: &Config) -> Result<(), ConfigError> {
          // Validate config values (e.g., particle_count > 0)
      }
  }
  ```

- [ ] Add detailed error handling with `thiserror`:
  ```rust
  #[derive(thiserror::Error, Debug)]
  pub enum ConfigError {
      #[error("Failed to read config file: {0}")]
      FileReadError(std::io::Error),
      
      #[error("Failed to parse JSON: {0}")]
      JsonParseError(serde_json::Error),
      
      #[error("Invalid configuration: {0}")]
      ValidationError(String),
  }
  ```

## 3. ECS Simulation Module (`eusociety-simulation`)

### Define Core Components
- [ ] Create component structs in `components.rs`:
  ```rust
  #[derive(Component, Debug, Clone, Copy, serde::Serialize)]
  pub struct Position {
      pub x: f32,
      pub y: f32,
  }
  
  #[derive(Component, Debug, Clone, Copy)]
  pub struct Velocity {
      pub dx: f32,
      pub dy: f32,
  }
  
  #[derive(Component, Debug, Clone, Copy, serde::Serialize)]
  pub struct ParticleId(pub usize);
  ```

### Create Simulation Systems
- [ ] Implement movement system in `systems/movement.rs`:
  ```rust
  pub fn move_particles(
      time: Res<Time>,
      mut query: Query<(&mut Position, &Velocity)>,
  ) {
      for (mut position, velocity) in query.iter_mut() {
          position.x += velocity.dx * time.delta_seconds();
          position.y += velocity.dy * time.delta_seconds();
      }
  }
  ```

- [ ] Implement randomization system in `systems/randomization.rs`:
  ```rust
  pub fn randomize_velocities(
      mut query: Query<&mut Velocity>,
      simulation_config: Res<SimulationConfig>,
      time: Res<Time>,
  ) {
      let factor = simulation_config.velocity_randomization_factor;
      for mut velocity in query.iter_mut() {
          // Add small random changes for smooth movement
          velocity.dx += (random::<f32>() - 0.5) * factor * time.delta_seconds();
          velocity.dy += (random::<f32>() - 0.5) * factor * time.delta_seconds();
      }
  }
  ```

- [ ] Implement boundary system in `systems/boundary.rs`:
  ```rust
  pub fn handle_boundaries(
      mut query: Query<(&mut Position, &mut Velocity)>,
      simulation_config: Res<SimulationConfig>,
  ) {
      let (width, height) = simulation_config.world_dimensions;
      
      for (mut pos, mut vel) in query.iter_mut() {
          match simulation_config.boundary_behavior {
              BoundaryBehavior::Wrap => {
                  // Wrap around logic
                  pos.x = (pos.x + width) % width;
                  pos.y = (pos.y + height) % height;
              },
              BoundaryBehavior::Bounce => {
                  // Bounce logic
                  if pos.x < 0.0 || pos.x > width {
                      vel.dx = -vel.dx;
                      pos.x = pos.x.clamp(0.0, width);
                  }
                  if pos.y < 0.0 || pos.y > height {
                      vel.dy = -vel.dy;
                      pos.y = pos.y.clamp(0.0, height);
                  }
              }
          }
      }
  }
  ```

### Setup Simulation Initialization
- [ ] Create particle spawning system in `systems/setup.rs`:
  ```rust
  pub fn spawn_particles(
      mut commands: Commands,
      simulation_config: Res<SimulationConfig>,
  ) {
      let (width, height) = simulation_config.world_dimensions;
      let max_vel = simulation_config.max_initial_velocity;
      
      for i in 0..simulation_config.particle_count {
          commands.spawn((
              ParticleId(i),
              Position {
                  x: random::<f32>() * width,
                  y: random::<f32>() * height,
              },
              Velocity {
                  dx: (random::<f32>() - 0.5) * max_vel * 2.0,
                  dy: (random::<f32>() - 0.5) * max_vel * 2.0,
              },
          ));
      }
  }
  ```

- [ ] Create plugin for registering systems in `lib.rs`:
  ```rust
  pub struct ParticleSimulationPlugin;
  
  impl Plugin for ParticleSimulationPlugin {
      fn build(&self, app: &mut App) {
          app
              .init_resource::<SimulationConfig>()
              .add_systems(Startup, spawn_particles)
              .add_systems(Update, (
                  move_particles,
                  randomize_velocities,
                  handle_boundaries,
              ).chain());
      }
  }
  ```

## 4. Transport Module (`eusociety-transport`)

### Implement Serializer Trait
- [ ] Define trait in `serializer.rs`:
  ```rust
  pub trait Serializer: Send + Sync {
      fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>, SerializationError>;
  }
  
  #[derive(thiserror::Error, Debug)]
  pub enum SerializationError {
      #[error("JSON serialization error: {0}")]
      JsonError(#[from] serde_json::Error),
      #[error("Binary serialization error: {0}")]
      BinaryError(#[from] bincode::Error),
  }
  ```

- [ ] Implement JSON serializer:
  ```rust
  pub struct JsonSerializer;
  
  impl Serializer for JsonSerializer {
      fn serialize<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>, SerializationError> {
          let json = serde_json::to_vec(data)?;
          Ok(json)
      }
  }
  ```

### Implement Sender Trait
- [ ] Define trait in `sender.rs`:
  ```rust
  pub trait Sender: Send + Sync {
      fn send(&self, data: &[u8]) -> Result<(), TransportError>;
      fn flush(&self) -> Result<(), TransportError>;
  }
  
  #[derive(thiserror::Error, Debug)]
  pub enum TransportError {
      #[error("I/O error: {0}")]
      IoError(#[from] std::io::Error),
  }
  ```

- [ ] Implement file sender:
  ```rust
  pub struct FileSender {
      file_path: String,
      file: Mutex<std::fs::File>,
  }
  
  impl FileSender {
      pub fn new(file_path: &str) -> Result<Self, TransportError> {
          let file = std::fs::File::create(file_path)?;
          Ok(Self {
              file_path: file_path.to_string(),
              file: Mutex::new(file),
          })
      }
  }
  
  impl Sender for FileSender {
      fn send(&self, data: &[u8]) -> Result<(), TransportError> {
          let mut file = self.file.lock().expect("Failed to lock file mutex");
          file.write_all(data)?;
          file.write_all(b"\n")?;
          Ok(())
      }
      
      fn flush(&self) -> Result<(), TransportError> {
          let mut file = self.file.lock().expect("Failed to lock file mutex");
          file.flush()?;
          Ok(())
      }
  }
  ```

### Create Transport Controller
- [ ] Implement controller in `controller.rs`:
  ```rust
  pub struct TransportController {
      serializer: Box<dyn Serializer>,
      sender: Box<dyn Sender>,
  }
  
  impl TransportController {
      pub fn new(
          serializer: Box<dyn Serializer>,
          sender: Box<dyn Sender>,
      ) -> Self {
          Self { serializer, sender }
      }
      
      pub fn from_config(config: &TransportConfig) -> Result<Self, TransportError> {
          let serializer: Box<dyn Serializer> = match config.serializer_type {
              SerializerType::Json => Box::new(JsonSerializer),
              SerializerType::Binary => Box::new(BinarySerializer),
          };
          
          let sender: Box<dyn Sender> = Box::new(FileSender::new(&config.output_path)?);
          
          Ok(Self::new(serializer, sender))
      }
      
      pub fn send_state<T: serde::Serialize>(&self, state: &T) -> Result<(), TransportError> {
          let data = self.serializer.serialize(state)
              .map_err(|e| TransportError::SerializationError(e))?;
          self.sender.send(&data)?;
          Ok(())
      }
  }
  ```

- [ ] For particle state extraction, create a serializable struct:
  ```rust
  #[derive(serde::Serialize)]
  pub struct ParticleState {
      pub id: usize,
      pub position: [f32; 2],
  }
  
  #[derive(serde::Serialize)]
  pub struct SimulationState {
      pub frame: u64,
      pub timestamp: f64,
      pub particles: Vec<ParticleState>,
  }
  ```

## Implementation Timeline
1. **Week 1:** Config module and basic project structure
2. **Week 2:** Core simulation components and movement system
3. **Week 3:** Transport module implementation
4. **Week 4:** Integration and testing

## Definition of Done
- [x] Configuration loads correctly from JSON file
- [x] 100 particles are simulated with smooth random movement
- [x] Particle state is correctly serialized to JSON file
- [x] All tests pass
- [x] Performance meets target of 60 FPS with 100 particles