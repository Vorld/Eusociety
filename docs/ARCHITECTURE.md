# Eusociety Architecture Document

## Project Vision

Eusociety is a high-performance, modular simulation engine built in Rust. The primary design goal is **runtime efficiency**, prioritizing the speed of the simulation loop above startup time. The engine aims to simulate large numbers of entities efficiently, leveraging an Entity-Component-System (ECS) architecture with compile-time optimizations. Runtime flexibility is provided for initial state configuration, transport layer selection, and simulation parameters via a configuration file.

## Core Principles

*   **Performance:** Maximize simulation loop speed through compile-time ECS registration, efficient data storage (target: contiguous/sparse sets), and potential parallelism.
*   **Modularity:** Separate concerns into distinct crates (`core`, `simulation`, `config`, `transport`, `runner`, `macros`) for maintainability and testability.
*   **Extensibility:** Allow users to define custom components and systems easily (target: procedural macros like `#[derive(Component)]`, `#[system]`, defined in `eusociety-macros`).
*   **Configurability:** Enable runtime setup of initial state, transport mechanisms (serialization format, output method), and simulation settings (FPS, threading) via a configuration file (e.g., JSON).

## Crate Structure

1.  **`eusociety-core`**
    *   **Purpose:** The heart of the engine. Manages the ECS (Entities, Components, Systems), World state, and the Scheduler.
    *   **Key Features (Target):**
        *   **Entities:** Lightweight IDs (e.g., `u32`).
        *   **Components:** Plain data structs implementing the `Component` trait, typically via `#[derive(Component)]` (macro provided by `eusociety-macros`). Stored efficiently using `Vec<Option<T>>` per component type.
        *   **Systems:** Logic units operating on components and resources. Definition and dependency declaration are evolving (target: `#[system]` macro or `System` trait implementation).
        *   **World:** Holds component storage (`Vec<Option<T>>` based) and global resources (e.g., `DeltaTime`). Provides accessors for components and resources.
        *   **Scheduler:** Manages system execution order. Currently sequential, but designed to eventually leverage dependency information for parallelism (target: `rayon`).
    *   **Milestone 2.2 State:** Uses `Vec<Option<T>>` for component storage, includes resource management. System definition uses `fn` pointers, but is targeted for refactoring in M2.3. Scheduler remains sequential.

2.  **`eusociety-simulation`** (Optional Crate)
    *   **Purpose:** Provides a library of common, reusable components and systems (e.g., `Position`, `Velocity`, `RandomMovementSystem`).
    *   **Details:** Users can opt-in to use this crate or define everything themselves. Depends on `eusociety-core`.

3.  **`eusociety-transport`**
    *   **Purpose:** Handles serialization and transmission of simulation data to external consumers (e.g., visualizers, RL agents).
    *   **Key Features:**
        *   `Serializer` trait: Defines how to convert world state (or parts) into bytes (Implementations: `BinarySerializer`, `JsonSerializer`).
        *   `Sender` trait: Defines how to transmit serialized bytes (Implementations: `FileSender`, `ConsoleSender`; Target: `WebSocketSender`).
        *   Runtime selection via config using `Box<dyn Trait>`. Factory functions (`create_serializer`, `create_sender`) facilitate this.

4.  **`eusociety-config`**
    *   **Purpose:** Parses the runtime configuration file.
    *   **Key Features:**
        *   Defines Rust structs (`Config`, `TransportConfig`, etc.) mirroring the JSON structure using `serde`.
        *   Loads and validates the configuration file (`config.json`).
        *   Provides typed configuration data to other crates.
        *   Handles basic error reporting for invalid formats or values.

5.  **`eusociety-macros`** (Procedural Macro Crate)
    *   **Purpose:** Defines the procedural macros (`#[derive(Component)]`, target: `#[system]`) used for compile-time code generation and integration with the ECS.
    *   **Details:** Has `proc-macro = true` set. `#[derive(Component)]` is implemented. `#[system]` is planned for M2.3. Depended upon by `eusociety-core` and user crates defining components/systems.

6.  **`eusociety-runner`**
    *   **Purpose:** The main executable crate. Initializes the engine and runs the simulation loop.
    *   **Key Features:**
        *   Orchestrates initialization: Loads config, creates World, Scheduler, Serializer, Sender.
        *   Populates the initial World state based on `config.start_state`.
        *   Registers systems with the Scheduler.
        *   Runs the main loop: calculates delta time (future), runs the scheduler, triggers serialization and sending via the transport layer, implements frame pacing (`spin_sleep`).

## Data Flow & Execution

1.  **Startup:**
    *   `eusociety-runner` starts.
    *   `eusociety-config` loads `config.json`.
    *   `eusociety-runner` initializes `eusociety-core::World` based on `start_state`, including initial components and resources (like `DeltaTime`).
    *   `eusociety-runner` initializes `eusociety-core::Scheduler` and registers systems (currently hardcoded `random_movement_system` using `fn` pointers).
    *   `eusociety-runner` initializes `eusociety-transport::{Serializer, Sender}` based on config using factory functions.
2.  **Runtime Loop:**
    *   Calculate and update frame delta time resource in the `World`.
    *   `Scheduler::run()` executes registered systems (sequentially in M1, potentially parallel later) modifying the `World` state.
    *   `Serializer::serialize()` converts relevant `World` data to bytes.
    *   `Sender::send()` transmits the byte data.
    *   Frame pacing logic ensures the loop runs close to the target FPS.
    *   Loop continues until an exit condition (e.g., max frames, external signal).

## Key Trade-offs & Future Goals

*   **Compile-Time vs. Runtime:** Heavily favors compile-time optimization for the core ECS loop, sacrificing some runtime flexibility in component/system registration (addressed via planned macros).
*   **ECS Implementation:** Milestone 2.2 uses optimized `Vec<Option<T>>` storage and includes resource management. System definition and a dependency-aware parallel scheduler using procedural macros are the next major steps (M2.3, M2.4).
*   **Transport Flexibility:** Uses dynamic dispatch (`Box<dyn Trait>`) for transport, accepting minor overhead for flexibility.
*   **Configuration:** Currently simple JSON. May support more complex scenarios (scripting, external data refs) later.
*   **External API:** An explicit API for external interaction (e.g., RL agents) is a future goal.
