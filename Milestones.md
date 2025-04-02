Architecture Plan: Eusociety
Project Vision & Goals: Remain unchanged – high runtime efficiency via ECS, compile-time optimizations, and targeted runtime flexibility through configuration.

Crate Structure:

eusociety-core

What it does: The heart of the engine—handles the ECS and world management.
Key Features:
Entities: Simple IDs (e.g., u32).
Components: Data structs (e.g., Position, Velocity).
Extensibility: Users define components by deriving a procedural macro: #[derive(Component)]. This macro generates the necessary compile-time registration code within the user's crate, allowing custom components without modifying eusociety-core. The core crate provides the Component trait and the derive macro implementation.
Storage: Stored in contiguous, type-specific storage (e.g., Vec<Option<Position>> or more optimized sparse sets) for cache efficiency.
Systems: Logic units that operate on components.
Extensibility: Users define systems as functions or structs implementing a System trait. A procedural macro #[system] can be used to automatically declare component access (read/write) and handle boilerplate.
Dependency Declaration: Systems explicitly declare their component access needs. For example:
#[system] // Hypothetical macro
fn movement_system(
    #[resource] time: &DeltaTime, // Access shared resources
    #[query] mut positions: Query<&mut Position>, // Write access to Position
    #[query] velocities: Query<&Velocity> // Read access to Velocity
) {
    // ... system logic using positions and velocities ...
}
The #[system] macro or the System trait implementation provides this metadata to the scheduler.
Scheduler: Manages system execution order based on declared dependencies. It identifies systems that can run in parallel (no conflicting component access) and those that must run sequentially. Leverages a thread pool (like rayon) configured via eusociety-config.
World: Global simulation state, configured at startup.
eusociety-simulation (Optional Crate)

What it does: Provides a library of optional, predefined components and systems (e.g., Position, Velocity, RandomMovementSystem).
Details: Users can choose to include this crate or define all their components/systems themselves. Depends on eusociety-core.
eusociety-transport

What it does: Manages serialization and transmission of simulation data.
Key Features:
Serializer Trait: Defines trait Serializer { fn serialize(&self, data: &World) -> Result<Vec<u8>, Error>; }. Implementations like JsonSerializer, BinarySerializer (e.g., using bincode).
Sender Trait: Defines trait Sender { fn send(&mut self, data: &[u8]) -> Result<(), Error>; }. Implementations like WebSocketSender, FileSender.
Runtime Selection: Config file selects implementations at startup using Box<dyn Serializer> and Box<dyn Sender>. We accept the minor dynamic dispatch overhead for flexibility, benchmark later if needed.
Serialization Granularity: (Deferred) Initially, might serialize a predefined subset or all components. A mechanism to configure which components to serialize will be added later if performance requires it.
Format Flexibility: Supports multiple formats (JSON for inspection, binary for performance) as per design.
eusociety-config

What it does: Parses a configuration file (initially JSON) to set up the simulation.
Key Features:
Loads start-state (entities, components, world parameters), transport settings (serializer type/options, sender type/options), and simulation parameters (threads, target FPS).
Format: Sticking with JSON for readability and ease of initial implementation. Advanced features like procedural generation or external data references are deferred.
Provides validated configuration data to other crates.
eusociety-runner

What it does: Initializes the engine and runs the main simulation loop.
Key Features:
Uses eusociety-config to get settings.
Initializes eusociety-core's ECS world and scheduler.
Initializes eusociety-transport's serializer and sender.
Runs the loop:
Calculate delta time since the last frame.
Run systems via the eusociety-core scheduler (passing delta time).
Serialize relevant data using eusociety-transport.
Send data using eusociety-transport.
Frame Pacing: Implement more robust frame pacing. Instead of simple sleep, use a technique that accounts for the time taken by the current frame's work:
Record start_time at the beginning of the loop.
Perform all work (systems, transport).
Record end_time.
Calculate elapsed_time = end_time - start_time.
Calculate target_frame_time = 1.0 / target_fps.
If elapsed_time < target_frame_time, calculate sleep_duration = target_frame_time - elapsed_time. Use a precise sleep function (e.g., from crates like spin_sleep) for sleep_duration. If elapsed_time >= target_frame_time, don't sleep (or maybe log a warning if consistently missing the target). This provides more stable FPS.
How It Runs: The pipeline remains largely the same, but the core ECS is now more extensible via macros, and system execution is explicitly managed by a dependency-aware scheduler. Frame pacing is more precise.

Key Trade-Offs:

Compile-Time vs. Runtime: Still heavily favors compile-time for the core loop, using macros for user extensibility.
Dynamic Dispatch: Accepted for transport flexibility.
Configuration: Simple JSON initially, deferring complexity.
Deferred Goals:

Fine-grained serialization control.
Advanced configuration options (scripting, external data).
Explicit external API for interaction (e.g., for RL).
This revised plan incorporates the procedural macros for extensibility, clarifies system dependencies for scheduling, makes eusociety-simulation optional, and proposes a better frame pacing mechanism, while acknowledging the deferred items.



Milestone 1:
Project Structure: Set up the workspace with crates for core, simulation, config, transport, and runner.
Core ECS (Simplified): Implemented basic Entity, Position component, World with HashMap storage, and a simple sequential Scheduler.
Simulation System: Created the random_movement_system in its own crate.
Configuration: Implemented loading and parsing of config.json defining start state, transport, and simulation parameters. The correct config.json was created.
Transport: Implemented Serializer and Sender traits, along with BinarySerializer and FileSender. Factory functions were added to create instances based on the config.
Runner: Created the main binary that initializes all parts based on the config, runs the simulation loop with frame pacing, executes systems, and uses the transport layer.
Execution: Successfully compiled and ran the simulation, which executed for the configured duration and outputted data.
As per the configuration ("sender": {"type": "file", "options": {"path": "output.bin"}}), the serialized position data for each frame was written to the output.bin file in the project root. This file contains the binary representation of the entity positions over time, confirming the simulation ran and the transport layer functioned.

Milestone 2