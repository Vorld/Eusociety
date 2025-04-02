# Milestone 2.3 Plan: System Registration and Dependencies

**Goal:** Formalize how systems declare their component and resource needs, enabling compile-time dependency analysis and paving the way for parallel execution. This involves defining access patterns, creating a `System` trait and a procedural macro (`#[system]`) for easier definition, and building a registry to store this information.

---

## Task 1: Define System Access Patterns

**Goal:** Establish clear types to represent how systems interact with components and resources.

**Subtasks:**

1.  **Define `AccessType` Enum:**
    *   Location: `eusociety-core/src/ecs/system.rs` (or similar)
    *   Variants: `Read`, `Write`
2.  **Define `DataAccess` Struct/Tuple:**
    *   Location: `eusociety-core/src/ecs/system.rs`
    *   Fields: `TypeId` (for component or resource), `AccessType`
    *   Purpose: Represent a single dependency (e.g., read `Position`, write `Velocity`).
3.  **Define `SystemAccess` Struct:**
    *   Location: `eusociety-core/src/ecs/system.rs`
    *   Fields: `component_access: Vec<DataAccess>`, `resource_access: Vec<DataAccess>`
    *   Purpose: Aggregate all dependencies for a single system.
    *   *Note:* Consider potential future expansion (e.g., `ReadWrite` access type for single-pass operations), but stick to `Read` and `Write` for M2.3 simplicity as they map more directly to parallel execution safety checks.

**Validation:** Code compiles, types are clearly defined and documented.

---

## Task 2: Implement `System` Trait

**Goal:** Create a trait that all systems must implement, providing metadata about their dependencies and the execution logic.

**Subtasks:**

1.  **Define `System` Trait:**
    *   Location: `eusociety-core/src/ecs/system.rs`
    *   Methods:
        *   `access(&self) -> SystemAccess`: Returns the system's component and resource dependencies.
        *   `run(&mut self, world: &mut World)`: Executes the system's logic. Systems will access data directly via `world` methods (e.g., `world.query()`, `world.resource()`) within this method.
2.  **`SystemParam` Approach (Deferred):**
    *   The approach of using a `SystemParam` trait (similar to Bevy) to automatically inject parameters into the `run` function signature was considered but **deferred** beyond M2.3 to reduce complexity. This could be revisited in future refactoring efforts if the direct `world` access proves cumbersome.

**Validation:** Trait compiles, methods are defined.

---

## Task 3: Create `#[system]` Procedural Macro

**Goal:** Provide a convenient way for users to define systems and automatically generate the `System` trait implementation and dependency information.

**Subtasks:**

1.  **Setup Macro Crate:** Ensure `eusociety-macros` is correctly set up (`proc-macro = true`, necessary dependencies like `syn`, `quote`).
2.  **Define Macro Input Structure:**
    *   Target syntax: `#[system] fn my_system(pos: &mut Position, time: &DeltaTime) { ... }`
    *   Parse function signature using `syn`.
3.  **Implement Dependency Extraction:**
    *   Analyze function parameters (e.g., `Query<&mut Position>`, `Res<&DeltaTime>`, `&mut Velocity`, `&Time`).
    *   Map parameter types and mutability (`&` vs `&mut`, or specific wrapper types like `Query`/`Res`) to `DataAccess` (Read/Write for components/resources). Ensure both component and resource access patterns are handled smoothly.
    *   Requires mapping type names (e.g., `Position`) to `TypeId`. This might involve looking up registered components/resources or using type information available to the macro.
4.  **Generate `System` Trait Implementation:**
    *   Use `quote` to generate the `impl System for MySystemStruct { ... }` block (or similar, depending on whether it wraps a function or struct).
    *   Generate the `access` method based on extracted dependencies.
    *   Generate the `run` method. This is the most complex part: it needs to safely fetch the required data (components via queries, resources) from the `World` based on the declared dependencies and pass them to the original function logic. Consider using helper traits like `SystemParam` if adopted.
5.  **Handle Struct Systems (Optional):** Consider if the macro should apply to functions directly or require wrapping them in a struct first. Start with functions for simplicity.
6.  **Iterative Development:** Acknowledge the complexity. Start with a simple version handling basic component access (`&T`, `&mut T`) and resource access (`Res<T>`), then expand capabilities.

**Validation:**
*   Macro compiles.
*   Applying `#[system]` to a simple function generates correct `impl System` code (inspect with `cargo expand`).
*   Generated code compiles successfully.

---

## Task 4: Refactor `random_movement_system`

**Goal:** Update the existing example system to use the new `#[system]` macro or implement the `System` trait directly.

**Subtasks:**

1.  **Apply `#[system]` Macro:**
    *   Modify `eusociety-simulation/src/lib.rs`.
    *   Annotate `random_movement_system` with `#[system]`.
    *   Adjust function signature if needed to match macro expectations (e.g., parameter types for component/resource access).
2.  **Update Runner:**
    *   Modify `eusociety-runner/src/main.rs`.
    *   Change how `random_movement_system` is registered with the (yet to be updated) scheduler/registry. Instead of a function pointer, it will now be an instance of the generated system struct or a type implementing `System`.

**Validation:** Simulation compiles and runs with the refactored system, producing expected output (or similar behavior, accounting for potential changes in access patterns).

---

## Task 5: Create System Registry

**Goal:** Implement a central place to store system instances and their dependency information, used by the scheduler.

**Subtasks:**

1.  **Define `SystemRegistry` Struct:**
    *   Location: `eusociety-core/src/ecs/scheduler.rs` (or a new `registry.rs`)
    *   Fields: `systems: Vec<Box<dyn System>>`, potentially storing `SystemAccess` separately or alongside.
2.  **Implement `add_system` Method:**
    *   Takes `impl System + 'static` as input.
    *   Stores the system instance (likely boxed).
    *   Retrieves and stores its `SystemAccess` metadata.
3.  **Integrate with Runner:**
    *   Modify `eusociety-runner/src/main.rs`.
    *   Instantiate the `SystemRegistry`.
    *   Register the refactored `random_movement_system` using `add_system`.
4.  **Update Scheduler (Basic):**
    *   Modify `eusociety-core/src/ecs/scheduler.rs`.
    *   The `Scheduler` should now hold or access the `SystemRegistry`.
    *   The `run` method should iterate through the registered systems and call their `run` methods sequentially (for now).
    *   *Note on Ordering:* For M2.3, the registry primarily enables dependency *analysis*. Explicit ordering constraints between systems are not a primary goal yet but might be considered later if complex workflows require it. Sequential execution based on registration order is the default.
5.  **Add Conflict Detection Tests:**
    *   Implement unit tests that attempt to register systems with conflicting dependencies (e.g., two systems writing to the same component type).
    *   Verify that the registry/scheduler can detect or handle these conflicts appropriately (e.g., log a warning, potentially prevent parallel execution later).

**Validation:**
*   Runner compiles and registers the system.
*   Scheduler can access and iterate through registered systems.
*   Simulation runs sequentially using the registry.
*   Conflict detection tests pass.

---

**Overall Validation for M2.3:** The simulation runs correctly using the new system definition (`#[system]` macro or direct `System` trait implementation) and the system registry, with systems declaring their dependencies explicitly, even though execution remains sequential for this milestone. Optional: Perform basic benchmarking before/after this milestone to measure any overhead introduced by the new registration/scheduling mechanism compared to the simple `fn` pointer approach (major performance analysis is deferred to M2.5).
