# Eusociety Architecture Document (Bevy ECS Branch)

## 1. Overview

**Goal:** Create a simulation engine that:

*   Leverages Bevy ECS for its core simulation state management.
*   Has a pluggable module for serialization (JSON, binary, etc.) and transport (stdio, file, websocket, etc.) to output the ECS state.
*   Uses a JSON-based configuration to set simulation parameters (e.g., framerate, world size, transport settings, initial state).

**Key Design Principles:**

*   **Separation of Concerns:** Each major function (simulation, configuration, serialization, transport) is encapsulated in its own module.
*   **Extensibility:** Use traits and interfaces so that new serializers or transport mechanisms can be added without modifying core logic.
*   **Configurability:** Use a JSON config file parsed via serde to allow easy tweaking of simulation parameters and output settings.
*   **Performance:** While leveraging Bevy ECS, aim for efficient state extraction and non-blocking transport where possible.

## 2. Architecture Components

### A. Configuration Module

*   **Purpose:** Load and parse a JSON configuration file.
*   **Responsibilities:**
    *   Parse simulation parameters such as expected framerate, world size, initial simulation state.
    *   Read transport configuration (which serializer and sender to use, along with their settings).
*   **Implementation Considerations:**
    *   Use `serde` for JSON deserialization.
    *   Define a configuration struct that mirrors the expected JSON schema.
    *   Provide functions to load and parse the JSON file, handling errors appropriately.

### B. ECS Simulation Module

*   **Purpose:** Encapsulate the simulation logic and state management using Bevy ECS.
*   **Responsibilities:**
    *   Define Bevy components, resources, and systems representing the simulation.
    *   Implement a main simulation loop that ticks the Bevy ECS `App` or `World` at the configured framerate.
*   **Implementation Considerations:**
    *   Keep simulation logic independent of the output mechanism.
    *   Leverage Bevy's scheduling and parallelism features.
    *   Optionally design systems to modify the simulation state based on external events (if needed).

### C. Transport Module (Serialization & Sender)

*   **Purpose:** Handle the serialization of ECS state and its transmission via various channels.
*   **Responsibilities:**
    *   **Serializer Interface:** Define a trait (e.g., `Serializer`) with methods like `serialize(&self, world: &bevy::prelude::World) -> Result<Vec<u8>, SerializationError>`. Provide implementations for JSON (`serde_json`) and binary formats (e.g., `bincode`).
    *   **Sender Interface:** Define a trait (e.g., `Sender`) with methods such as `send(&self, data: &[u8]) -> Result<(), TransportError>`. Implement different senders: `StdioSender`, `FileSender`, `WebSocketSender`, etc.
    *   **Transport Controller:** Combine a chosen serializer and sender (determined by configuration) into a single unit responsible for extracting state, serializing it, and sending it.
*   **Implementation Considerations:**
    *   Use the configuration module to determine which implementations to instantiate.
    *   Design with the Strategy pattern (using `Box<dyn Trait>`) to allow dynamic selection of serializer and sender at runtime.

### D. Integration Layer

*   **Purpose:** Integrate the Bevy ECS simulation loop with the transport mechanism.
*   **Responsibilities:**
    *   Within a Bevy system or stage, extract the current ECS state (potentially querying specific components/resources).
    *   Pass the relevant state to the `TransportController` to serialize and send the data at configurable intervals (e.g., every frame, every N frames).
*   **Implementation Considerations:**
    *   Ensure that the ECS state extraction is efficient. Consider how to best query the `World`.
    *   Handle asynchronous sending if required (especially for websocket or file I/O) without blocking the simulation loop (e.g., using `tokio` or `async-std` integrated with Bevy).

## 3. Data Flow & Execution

1.  **Startup:**
    *   The main runner executable starts.
    *   The `Configuration Module` loads and parses `config.json`.
    *   A Bevy `App` is initialized.
    *   Initial simulation state (entities, components, resources) is set up in the Bevy `World` based on the configuration.
    *   Simulation systems are added to the Bevy schedule.
    *   The `Transport Module` initializes the configured `Serializer` and `Sender` based on the config, likely creating a `TransportController`.
    *   An integration system/stage is added to the Bevy schedule to handle state extraction and transport.
2.  **Runtime Loop (Bevy App Run):**
    *   Bevy's scheduler executes systems according to its schedule (potentially in parallel).
    *   Simulation systems update the `World` state (components, resources).
    *   The integration system runs:
        *   It queries the `World` to extract the necessary state snapshot.
        *   It passes the snapshot to the `TransportController`.
        *   The `TransportController` uses the `Serializer` to convert the state to bytes.
        *   The `TransportController` uses the `Sender` to transmit the bytes (potentially asynchronously).
    *   Bevy manages the loop timing based on its configuration (e.g., `WinitSettings::desktop_app()`).
    *   The loop continues until Bevy's exit conditions are met.

## 4. Implementation Considerations and Decision Rationale

*   **Modularity:** By keeping the configuration, simulation (within Bevy), and transport modules separate, components can be evolved or swapped (e.g., adding a new transport mechanism) with minimal impact on the core simulation.
*   **Abstraction via Traits:** Traits for `Serializer` and `Sender` enable using the Strategy pattern. The configuration dictates which implementations are used at runtime, decoupling instantiation from usage.
*   **Leveraging Bevy:** Rely on Bevy ECS for efficient state management, scheduling, and potentially parallelism. Focus application logic on defining components and systems.
*   **Ease of Extension:** New serialization formats or transport mechanisms can be added by implementing the corresponding traits and updating the configuration parsing/handling logic. **Future goals include potentially adding an external API for interaction (e.g., for RL agents) and exploring more advanced configuration options like scripting.**
*   **Performance and Non-blocking Behavior:** The simulation relies on Bevy's performance. Transport operations, especially I/O-bound ones (file, network), should ideally be asynchronous to avoid blocking the main Bevy schedule. Bevy's task pools or integration with async runtimes can facilitate this.
*   **Error Handling:** Each module should implement robust error handling (e.g., config parsing errors, serialization failures, transport connection issues) and provide clear logging.
