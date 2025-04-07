# Eusociety Simulation Documentation

## Overview

Eusociety is a Rust application that simulates the movement of particles within a defined 2D world. It features configurable parameters for simulation behavior (particle count, velocity, boundary interactions) and data transport (serialization format, output method). The simulation core uses the Bevy ECS framework for efficient state management and system execution. Data can be output to a file or streamed over WebSockets for real-time visualization or analysis.

## Features

*   **Particle Simulation:** Simulates N particles moving in a 2D space.
*   **Configurable Behavior:**
    *   Particle count.
    *   World dimensions.
    *   Maximum particle velocity.
    *   Velocity randomization factor.
    *   Velocity damping factor (friction/drag).
    *   Boundary behavior (Wrap or Bounce).
    *   Target frame rate.
*   **Bevy ECS Integration:** Utilizes Bevy ECS for managing entities, components, resources, and systems.
*   **Configurable Transport:**
    *   **Serialization:** JSON, Bincode Binary, Optimized Binary (with optional delta compression and parallelism).
    *   **Sending:** Output to File, WebSocket server broadcast, Null (no output).
*   **Optimizations:**
    *   Delta compression for WebSocket transport (sends only changed particle data).
    *   Parallel serialization for large particle counts using Rayon.
*   **Logging:** Uses the `tracing` library for detailed logging, configurable via the `RUST_LOG` environment variable.

## Installation & Usage

1.  **Prerequisites:** Ensure you have Rust and Cargo installed ([https://rustup.rs/](https://rustup.rs/)).
2.  **Build:** Navigate to the project root directory and run:
    ```bash
    cargo build --release
    ```
3.  **Configure:** Edit the `config.json` file in the project root to set simulation and transport parameters (see Configuration section below).
4.  **Run:** Execute the simulation from the project root:
    ```bash
    # Example: Set log level to debug for eusociety crate
    RUST_LOG=eusociety=debug cargo run --release 
    # Or simply run with default info level
    cargo run --release
    ```
    The simulation will run according to the settings in `config.json`. Press Ctrl+C to stop it.

## Configuration (`config.json`)

The simulation is configured via the `config.json` file located in the project root. Below is an explanation of the structure and fields:

```json
{
  "simulation": {
    "particle_count": 10000,
    "world_dimensions": [800.0, 600.0],
    "max_velocity": 100.0,
    "velocity_randomization_factor": 50.0,
    "velocity_damping_factor": 0.99,
    "boundary_behavior": "Bounce", // or "Wrap"
    "frame_rate": 60
  },
  "transport": {
    "serializer": {
      "serializer_type": "Binary", // "Json", "Binary", "Null"
      "options": {} // Specific options for the chosen serializer (currently none)
    },
    "sender": {
      "sender_type": "WebSocket", // "File", "WebSocket", "Null"
      "options": {
        // --- Options for "WebSocket" ---
        "websocket_address": "127.0.0.1:9001",
        "update_frequency": 1 // Send every frame
        // --- Options for "File" ---
        // "output_path": "output.bin", // Or output.json if using Json serializer
        // "output_frequency": 1 // Write every frame
        // --- Options for "Null" ---
        // (No options needed)
      }
    },
    "delta_compression": true, // Optional: Enable delta compression (default: false) - Primarily for OptimizedBinarySerializer/WebSocket
    "delta_threshold": 0.1,   // Optional: Movement threshold for delta compression (default: 0.1)
    "parallel_serialization": { // Optional: Configure parallel serialization - Primarily for OptimizedBinarySerializer
      "enabled": true,          // Optional: Enable/disable (default: true)
      "threshold": 50000,       // Optional: Particle count threshold to enable parallelism (default: 50000)
      "thread_count": 0         // Optional: Number of threads (0 = auto, default: 0)
    },
    "log_frequency": 60 // Optional: Log transport performance every N frames (0=every frame, null/absent=never, default: null)
  }
}
```

**`simulation` Section:**

*   `particle_count` (integer): Number of particles to simulate. Must be > 0.
*   `world_dimensions` (array `[width, height]`): Floating-point width and height of the simulation area. Must be > 0.
*   `max_velocity` (float): Maximum speed any particle can attain.
*   `velocity_randomization_factor` (float): Controls how much random fluctuation is added to velocity each frame. Higher values mean more erratic movement.
*   `velocity_damping_factor` (float): Factor applied to velocity each frame (0.0 to 1.0). Simulates drag/friction. 1.0 means no damping.
*   `boundary_behavior` (string): How particles interact with world edges.
    *   `"Wrap"`: Particles exiting one side reappear on the opposite side.
    *   `"Bounce"`: Particles reflect off the boundaries.
*   `frame_rate` (integer): Target frames per second for the simulation loop. Must be > 0.

**`transport` Section:**

*   `serializer`: Defines how simulation state is converted to bytes.
    *   `serializer_type` (string): `"Json"`, `"Binary"`, or `"Null"`.
    *   `options` (object): Placeholder for future serializer-specific options.
*   `sender`: Defines where the serialized data is sent.
    *   `sender_type` (string): `"File"`, `"WebSocket"`, or `"Null"`.
    *   `options` (object): Sender-specific options:
        *   **For `"File"`:**
            *   `output_path` (string): Path to the output file (e.g., "output.bin", "output.json").
            *   `output_frequency` (integer): How often (in frames) to write to the file. Must be > 0.
        *   **For `"WebSocket"`:**
            *   `websocket_address` (string): IP address and port for the WebSocket server to listen on (e.g., "127.0.0.1:9001").
            *   `update_frequency` (integer): How often (in frames) to send updates to clients. Must be > 0.
        *   **For `"Null"`:** No options needed.
*   `delta_compression` (boolean, optional): If `true`, enables sending only particle data that changed significantly since the last send (requires `OptimizedBinarySerializer`, typically used with `WebSocketSender`). Defaults to `false`.
*   `delta_threshold` (float, optional): The minimum distance a particle must move to be included when `delta_compression` is enabled. Defaults to `0.1`.
*   `parallel_serialization` (object, optional): Configures parallel processing for serialization (requires `OptimizedBinarySerializer`).
    *   `enabled` (boolean, optional): Enable/disable parallelism. Defaults to `true`.
    *   `threshold` (integer, optional): Minimum particle count to activate parallelism. Defaults to `50000`.
    *   `thread_count` (integer, optional): Number of threads for Rayon pool (0 = auto). Defaults to `0`.
*   `log_frequency` (integer, optional): How often (in frames) to log transport performance metrics. `0` logs every frame. If absent or `null`, logging is disabled.

## Architecture

The project is structured into several Rust modules within the `src` directory:

*   **`main.rs`:** Executable entry point. Handles initialization (logging, config) and starts the `SimulationApp`.
*   **`lib.rs`:** Library root, declares modules and provides a `prelude`.
*   **`config/`:** Handles configuration loading (`mod.rs`) and defines configuration data structures (`types.rs`).
*   **`simulation/`:** Contains the core simulation logic using Bevy ECS.
    *   `mod.rs`: Defines `SimulationApp` which manages the Bevy `World` and `Schedule`s.
    *   `components/`: Defines ECS components (`Position`, `Velocity`, `ParticleId`).
    *   `resources/`: Defines ECS resources (`Time`, `FrameCounter`, configuration wrappers, `CurrentSimulationState`).
    *   `systems/`: Defines ECS systems that implement simulation logic.
        *   `mod.rs`: Declares and re-exports system modules.
        *   `setup.rs`: System for spawning initial particles.
        *   `movement.rs`: System for updating particle positions.
        *   `randomization.rs`: System for applying velocity damping and randomization.
        *   `boundary.rs`: System for handling world boundary interactions.
        *   `state_export.rs`: System to capture the current simulation state into a resource.
        *   `transport_integration.rs`: System to trigger sending the captured state via the `TransportController`.
*   **`transport/`:** Handles data serialization and sending.
    *   `mod.rs`: Defines core transport traits, data structures (`ParticleState`, `SimulationState`), and the `TransportController`. Declares and re-exports submodules.
    *   `serializer.rs`: Defines `Serializer` trait and implementations (`JsonSerializer`, `BinarySerializer`, `NullSerializer`, `OptimizedBinarySerializer`).
    *   `sender.rs`: Defines `Sender` trait and implementations (`FileSender`, `NullSender`).
    *   `delta_compression.rs`: Implements `DeltaCompressor` logic used by `OptimizedBinarySerializer`.
    *   `websocket.rs`: Implements `WebSocketSender` logic, including the async server.

## Frontend Integration (WebSocket)

If using the `WebSocketSender`, a frontend application can connect to the specified `websocket_address`. The server sends simulation state data as **binary WebSocket messages**.

The binary format (when using `BinarySerializer` or `OptimizedBinarySerializer`) follows the `bincode` serialization of the `SimulationState` struct:

1.  **Frame Number:** `u64` (little-endian)
2.  **Timestamp:** `f64` (little-endian)
3.  **Particle Count:** `u64` (little-endian) - Number of particles *in this message* (relevant for delta compression).
4.  **Particle Data:** Repeated `Particle Count` times:
    *   **ID:** `u32` (little-endian)
    *   **X Position:** `f32` (little-endian)
    *   **Y Position:** `f32` (little-endian)

A frontend needs to parse this binary structure accordingly. If using JSON serialization, the messages will be standard JSON strings representing the `SimulationState`.
