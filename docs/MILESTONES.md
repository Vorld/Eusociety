# Eusociety Milestones

## Milestone 1: Basic Engine Setup [COMPLETED]

**Goal:** Establish a minimal, working simulation engine with a simplified ECS and config-driven runtime flexibility.

**Achievements:**

*   **Project Structure:** Workspace setup with `core`, `simulation`, `config`, `transport`, and `runner` crates.
*   **Simplified Core ECS:** Implemented `Entity`, `Position` component, `World` (using `HashMap`), and a sequential `Scheduler` (using `fn` pointers).
*   **Simulation System:** Created `random_movement_system`.
*   **Configuration:** Implemented loading/parsing of `config.json` via `serde` for start state (10 entities), transport (binary/JSON serializer, file/console sender), and simulation parameters (FPS).
*   **Transport Layer:** Defined `Serializer`/`Sender` traits with `BinarySerializer`, `JsonSerializer`, `FileSender`, `ConsoleSender` implementations and factory functions.
*   **Runner:** Implemented the main loop orchestrating initialization, system execution, transport calls, and frame pacing (`spin_sleep`).
*   **Validation:** Successfully compiled and ran the simulation, outputting serialized data (binary or JSON) to a file (`output.bin`/`output.json`).

---

## Milestone 2: Enhanced ECS Core, Parallelism & Websocket Support [IN PROGRESS]

**Goal:** Refactor the core ECS for better performance and extensibility, introducing compile-time registration, parallel execution foundation, and websocket transport.

### Milestone 2.1: Core Component System Refactoring [COMPLETED]

**Goal:** Improve the foundational data structures while maintaining compatibility with existing systems.

**Tasks:**
- Create `eusociety-macros` crate with basic setup
- Implement `#[derive(Component)]` procedural macro
- Define `Component` trait in core
- Implement optimized `Vec<Option<T>>` storage
- Update `World` to use generic component storage
- Update existing code to use the new component system

**Validation:** Run existing simulation with new component system - output matched previous results.

### Milestone 2.2: Resource Management [COMPLETED]

**Goal:** Add the ability to store and access shared resources.

**Tasks:**
- Define `Resource` trait
- Implement `DeltaTime` and other basic resources
- Add resource storage to `World`
- Create accessor methods for resources
- Update configuration to populate initial resources

**Validation:** Enhanced simulation to use `DeltaTime` resource, verified system access to global resources.

### Milestone 2.3: System Registration and Dependencies [COMPLETED]

**Goal:** Formalize how systems declare their component needs.

**Tasks:**
- Define system access patterns (read/write) (`AccessType`, `DataAccess`, `SystemAccess`)
- Implement `System` trait with dependency declarations (`access` method)
- Create `#[system]` procedural macro (with limitations due to borrow checking)
- Refactor `random_movement_system` to use manual `impl System` (as macro doesn't support its signature)
- Create registry (`SystemRegistry`) and scheduler (`SystemScheduler`) storing systems and access info
- Implement basic conflict detection in registry

**Validation:** Ran simulation with registered systems, verified correct execution order based on manual `impl System` dependencies. Macro compiles and works for simple cases but errors correctly for unsupported complex signatures.

### Milestone 2.4: Parallel Scheduling Foundation [COMPLETED]

**Goal:** Implement the core logic for a parallel-capable scheduler by analyzing system dependencies, calculating execution stages, and introducing `rayon`. Focus on correct sequential staging based on dependencies.

**Tasks:**
- Defined `DependencyGraph` type alias and implemented `build_dependency_graph` function in `scheduler.rs` to construct a graph based on `SystemAccess` conflicts (Read/Write on components/resources).
- Implemented `calculate_execution_stages` using Kahn's algorithm (topological sort) with cycle detection.
- Added `rayon` dependency to `eusociety-core` and the workspace.
- Updated `SystemScheduler::run` to dynamically build the dependency graph and calculate stages on each run.
- Modified `SystemScheduler::run` to execute systems sequentially based on the calculated stages, ensuring dependencies are respected regardless of registration order.
- Added unit tests for graph building, stage calculation (including cycle detection), and an integration test (`test_scheduler_execution_order`) verifying correct execution order.
- Updated documentation (`scheduler.rs` doc comments and `docs/Documentation.md`) to reflect the new dependency-aware scheduling and its current limitations (sequential execution within stages).

**Validation:** Simulation runs correctly with the new scheduler. Systems are executed in an order that respects their data dependencies, grouped into stages. All tests in `eusociety-core` pass, including specific tests for graph building, stage calculation, cycle detection, and execution order.

### Milestone 2.5: True Parallel Execution & Ergonomics [TODO]

**Goal:** Enable true parallel system execution and improve system definition ergonomics.

**Tasks:**
- Define and implement `SystemParam` trait for `Query`, `Res`, `ResMut`, `Commands`, etc.
- Refactor `System` trait to use `SystemParam` instead of `&mut World`.
- Refactor `World` internals to support safe concurrent access (likely involves `unsafe`).
- Update `#[system]` macro to work with `SystemParam`, removing borrow check limitations.
- Update `SystemScheduler` to use `rayon` to execute systems *within* stages in parallel.
- Set up benchmarking crate (`criterion`) and create benchmarks for sequential vs. parallel execution.
- Document performance findings and improved macro usage.

**Validation:** Compare execution of sequential vs. parallel scheduler with varying system counts, demonstrating performance scaling. Verify macro works for previously unsupported signatures.

### Milestone 2.6: WebSocket Transport [TODO]

**Goal:** Enable real-time visualization.

**Tasks:**
- Implement WebSocket sender (`WebSocketSender`) using a suitable crate (e.g., `tungstenite`).
- Integrate with existing serializers.
- Update configuration to support WebSocket settings (address, port).
- Create basic HTML/JS client for visualization (e.g., using `three.js` or simple canvas).

**Validation:** Connect web browser to simulation and verify real-time data reception and basic visualization.
