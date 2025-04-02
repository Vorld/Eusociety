# Eusociety Engine Documentation (as of M2.3)

## 1. Overview

Eusociety is a simulation engine built in Rust, based on the Entity Component System (ECS) architecture. It aims to provide a flexible and potentially performant foundation for running simulations where behavior emerges from the interaction of simple rules applied to entities with specific data.

The engine is currently under development and focuses on core ECS principles, configuration-driven setup, and data transport.

## 2. Core Concepts

Eusociety follows the ECS pattern:

*   **Entity:** A unique identifier (currently `usize`) representing an object in the simulation (e.g., an agent, a particle).
*   **Component:** A piece of data associated with an entity (e.g., `Position`, `Velocity`). Components define the *state* of an entity.
*   **System:** A piece of logic that operates on entities possessing specific components and/or accesses global resources (e.g., a movement system acting on entities with `Position`). Systems define the *behavior* of the simulation.
*   **Resource:** Global, shared data accessible by systems (e.g., `DeltaTime` for time-step information, configuration settings).
*   **World:** The central container holding all entities, components, resources, and the system scheduler.

### 2.1. World

The `eusociety_core::World` struct is the main entry point for interacting with the ECS. You can:
*   Create/delete entities.
*   Add/remove components for entities.
*   Insert/access/remove resources.
*   Register systems with the scheduler.
*   Run the simulation loop via the scheduler.

### 2.2. Components

Components are simple Rust structs representing data.

**Defining Components:**
Any struct can be a component. To integrate properly with the ECS, derive the `Component` trait using the provided macro:

```rust
use eusociety_core::Component; // Required when deriving in eusociety-core
use eusociety_macros::Component; // Required when deriving outside eusociety-core
use serde::{Serialize, Deserialize}; // If needed for config/transport

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)] // Simple component without serialization
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}
```
*(Note: The `Component` trait itself is currently empty but used for type registration).*

**Using Components:**
```rust
use eusociety_core::World;
// Assuming Position component is defined as above

let mut world = World::new();
let entity1 = world.create_entity();

// Add a component to an entity
world.add_component(entity1, Position { x: 10.0, y: 5.0 });

// Get a component (immutable access)
if let Some(pos) = world.get_component::<Position>(entity1) {
    println!("Entity 1 Position: {:?}", pos);
}

// Get a component (mutable access)
if let Some(pos) = world.get_component_mut::<Position>(entity1) {
    pos.x += 1.0;
}

// Remove a component
world.remove_component::<Position>(entity1);
```

### 2.3. Resources

Resources represent global state.

**Defining Resources:**
Any struct that implements `Send + Sync + 'static` can be a resource. It must also implement the marker trait `Resource`.

```rust
use eusociety_core::Resource;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct DeltaTime {
    pub delta_seconds: f32,
    // internal duration tracking maybe
}

// Implement the marker trait
impl Resource for DeltaTime {}

impl DeltaTime {
    pub fn new(duration: Duration) -> Self {
        Self { delta_seconds: duration.as_secs_f32() }
    }
    // other methods like update...
}
```

**Using Resources:**
```rust
use eusociety_core::World;
// Assuming DeltaTime resource is defined as above

let mut world = World::new();

// Insert a resource (takes ownership)
world.insert_resource(DeltaTime::default());

// Get a resource (immutable access) - returns Option<&T>
if let Some(dt) = world.get_resource::<DeltaTime>() {
    println!("Delta time: {}", dt.delta_seconds);
}

// Get a resource (mutable access) - returns Option<&mut T>
if let Some(dt) = world.get_resource_mut::<DeltaTime>() {
    dt.delta_seconds = 1.0 / 60.0;
}

// Remove a resource
world.remove_resource::<DeltaTime>();
```

### 2.4. Systems

Systems contain the simulation logic.

**Defining Systems (Manual Implementation):**
Currently, due to borrow-checking complexities with the `#[system]` macro, systems involving mutable access (`&mut T`, `ResMut<T>`) combined with other parameters often require manual implementation of the `System` trait:

```rust
use eusociety_core::{
    System, SystemAccess, AccessType, World, Position, DeltaTime, DataAccess
};
use std::any::TypeId;
use rand::Rng; // Assuming rand is a dependency

pub struct MyMovementSystem;

impl System for MyMovementSystem {
    // Declare data dependencies
    fn access(&self) -> SystemAccess {
        SystemAccess::new()
            .with_component(TypeId::of::<Position>(), AccessType::Write) // Needs write access to Position
            .with_resource(TypeId::of::<DeltaTime>(), AccessType::Read)  // Needs read access to DeltaTime
    }

    // Implement the system's logic
    fn run(&mut self, world: &mut World) {
        // 1. Access resources (usually once per run)
        let dt = match world.get_resource::<DeltaTime>() {
            Some(res) => res.delta_seconds,
            None => { return; } // Or handle error/default
        };

        // 2. Query and iterate entities with required components
        for (_entity, pos) in world.components.query_mut::<Position>() {
            // 3. Apply logic
            pos.x += 1.0 * dt; // Example: Move right based on delta time
        }
    }
}
```

**Defining Systems (`#[system]` Macro):**
For simpler systems, the `#[system]` macro can reduce boilerplate. It automatically generates the `struct`, `impl System`, `access` method, and the `run` method's data fetching logic.

*Supported Patterns:*
    *   Only resources: `fn sys(res: Res<R>, mut res_mut: ResMut<RM>)`
    *   Only immutable components/resources: `fn sys(c1: &C1, c2: &C2, res: Res<R>)`
    *   Exactly one mutable component, nothing else: `fn sys(c: &mut C)`

*Unsupported Patterns (Require Manual Implementation):*
    *   `&mut C` with any other component or resource (`&C2`, `Res<R>`, `ResMut<RM>`).
    *   `ResMut<RM>` with any component (`&C`, `&mut C`).

*Example (Supported):*
```rust
use eusociety_macros::system;
use eusociety_core::resources::Res;
// Assuming DeltaTime resource exists

#[system]
fn print_time_system(time: Res<DeltaTime>) {
    println!("Current delta: {}", time.delta_seconds);
}
```
This generates `struct PrintTimeSystemSystem` which implements `System`.

### 2.5. Scheduler

The `eusociety_core::SystemScheduler` is responsible for running registered systems.

*   **Registration:** Systems are added using `scheduler.add_system(MySystem)`. The scheduler checks for immediate conflicts (e.g., two systems writing to the same resource) based on the `access()` declaration and returns `false` if found. `add_system_unchecked` bypasses this check.
*   **Execution:** Calling `scheduler.run(&mut world)` executes the systems. Currently (M2.3), systems are run **sequentially** in the order they were added (if no conflicts were detected by `add_system`) or in the order added via `add_system_unchecked`. Dependency analysis for ordering is planned for M2.4.

```rust
use eusociety_core::{SystemScheduler, World};
// Assuming MyMovementSystem and PrintTimeSystemSystem structs exist

let mut world = World::new();
// ... initialize world ...

let mut scheduler = SystemScheduler::new();

// Register systems (struct instances)
scheduler.add_system(MyMovementSystem);
scheduler.add_system(PrintTimeSystemSystem); // From #[system] macro

// In the main loop:
scheduler.run(&mut world);
```

## 3. Project Structure

The engine is organized into several crates within a Cargo workspace:

*   `eusociety-core`: Contains the fundamental ECS types (`World`, `Component`, `Resource`, `System`, `SystemScheduler`, etc.).
*   `eusociety-macros`: Defines procedural macros like `#[derive(Component)]` and `#[system]`.
*   `eusociety-simulation`: Holds example components and systems for the simulation (e.g., `Position`, `DeltaTime`, `RandomMovementSystem`). Users would typically create their own simulation crate.
*   `eusociety-config`: Handles loading and parsing the `config.json` file.
*   `eusociety-transport`: Defines traits (`Serializer`, `Sender`) and implementations for sending simulation state elsewhere (e.g., to a file, console, or potentially network).
*   `eusociety-runner`: The main executable that parses config, sets up the `World`, registers systems, runs the simulation loop, and uses the transport layer.

## 4. Configuration (`config.json`)

The `eusociety-runner` reads a configuration file (e.g., `config.json` or `stress_config.json`) to set up the simulation. Key sections:

*   `simulation`:
    *   `fps`: Target frames per second for the simulation loop pacing.
    *   `threads`: (Currently unused) Planned for configuring parallel execution.
*   `start_state`:
    *   `entities`: An array defining initial entities and their components. Components are specified as key-value pairs matching component struct names and their JSON representation.
        ```json
        "entities": [
          { "id": 0, "components": { "Position": {"x": 10.0, "y": -5.0} } },
          { "id": 1, "components": { "Position": {"x": 0.0, "y": 0.0} } }
        ]
        ```
*   `initial_resources`: (Optional) Defines initial values for resources. Resource names must match the struct names.
    ```json
    "initial_resources": {
      "DeltaTime": { "delta_seconds": 0.016 }
    }
    ```
*   `transport`: Configures how simulation data is outputted.
    *   `serializer`: `type_` ("json" or "binary").
    *   `sender`: `type_` ("file" or "console") and `options` (e.g., `"path": "output.bin"` for file sender).

## 5. Running the Simulation

Build and run the `eusociety-runner` executable:

```bash
# From the workspace root directory
cargo run --bin eusociety-runner
```
This will typically look for `config.json` in the root, initialize the world, run the systems sequentially, and output data according to the transport configuration.

## 6. Transport Layer

The transport layer allows decoupling the simulation state from how it's outputted or consumed.

*   **`Serializer` Trait:** Defines how to serialize the `World` state (currently serializes all components). Implementations: `JsonSerializer`, `BinarySerializer` (using `bincode`).
*   **`Sender` Trait:** Defines how to send the serialized data. Implementations: `FileSender`, `ConsoleSender`.

These are configured in `config.json` and instantiated by the runner.

## 7. Example Usage (Summary)

1.  **Define Components:** Create structs, derive `Component`, `Serialize`, `Deserialize`.
2.  **Define Resources:** Create structs, implement `Resource`.
3.  **Define Systems:**
    *   For simple cases (see rules above), use `#[system]` on a function.
    *   For complex cases (mixing `&mut T` with other params, or `ResMut<T>` with components), create a struct and manually implement the `System` trait (`access` and `run` methods).
4.  **Configure `config.json`:** Set up initial entities, resources, simulation parameters, and transport.
5.  **Update Runner (if needed):** Import and register your systems in `eusociety-runner/src/main.rs`'s `run_simulation` function using `scheduler.add_system(...)`.
6.  **Run:** Execute `cargo run --bin eusociety-runner`.

This documentation reflects the state after Milestone 2.3. Future milestones will introduce parallel scheduling (M2.4), improved system ergonomics (M2.5), and WebSocket transport (M2.6).
