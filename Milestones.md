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

## Milestone 2: Enhanced ECS Core, Basic Parallelism & Websocket Support [TODO]

**Goal:** Refactor the core ECS for better performance and extensibility, introducing compile-time registration, basic parallel execution and websocket transport.

### Milestone 2.1: Core Component System Refactoring [TODO]

**Goal:** Improve the foundational data structures while maintaining compatibility with existing systems.

**Tasks:**
- Create `eusociety-macros` crate with basic setup
- Implement `#[derive(Component)]` procedural macro
- Define `Component` trait in core
- Implement optimized `Vec<Option<T>>` storage
- Update `World` to use generic component storage
- Update existing code to use the new component system

**Validation:** Run existing simulation with new component system - output should match previous results.

### Milestone 2.2: Resource Management [TODO]

**Goal:** Add the ability to store and access shared resources.

**Tasks:**
- Define `Resource` trait
- Implement `DeltaTime` and other basic resources
- Add resource storage to `World`
- Create accessor methods for resources
- Update configuration to populate initial resources

**Validation:** Enhance simulation to use `DeltaTime` resource, verify system access to global resources.

### Milestone 2.3: System Registration and Dependencies [TODO]

**Goal:** Formalize how systems declare their component needs.

**Tasks:**
- Define system access patterns (read/write)
- Implement `System` trait with dependency declarations
- Create `#[system]` procedural macro
- Refactor `random_movement_system` to use new approach
- Create registry for systems with dependency information

**Validation:** Run simulation with registered systems, verify correct execution with explicit dependencies.

### Milestone 2.4: Parallel Execution [TODO]

**Goal:** Enable parallel system execution.

**Tasks:**
- Add Rayon dependency
- Implement dependency analysis algorithm
- Create parallel scheduler
- Integrate with configuration (thread count)
- Update runner to use parallel execution when configured

**Validation:** Compare execution of sequential vs. parallel scheduler with varying system counts.

### Milestone 2.5: Benchmarking [TODO]

**Goal:** Measure performance improvements.

**Tasks:**
- Set up benchmarking crate with Criterion
- Create benchmarks for component storage (old vs. new)
- Benchmark sequential vs. parallel execution
- Implement end-to-end simulation benchmarks
- Document performance findings

**Validation:** Run benchmarks and verify improved performance.

### Milestone 2.6: WebSocket Transport [TODO]

**Goal:** Enable real-time visualization.

**Tasks:**
- Implement WebSocket sender
- Integrate with existing serializers
- Update configuration to support WebSocket settings
- Create basic HTML/JS client for visualization

**Validation:** Connect web browser to simulation and verify data reception.
