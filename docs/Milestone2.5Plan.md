# Milestone 2.5 Plan: True Parallel Execution & Ergonomics

**Goal:** Refactor the core ECS (`World`, `System` trait) and the `#[system]` macro to enable true parallel execution of systems within stages, leveraging `SystemParam` for ergonomic and safe data access. Introduce basic benchmarking.

**Prerequisite:** Milestone 2.4 (Parallel Scheduling Foundation) completed.

---

## Task 1: Define `SystemParam` Trait & Basic Implementations

**Goal:** Create the foundation for abstracting system data access.

**Subtasks:**

1.  **Define `SystemParam` Trait:**
    *   Location: `eusociety-core/src/ecs/system.rs` (or a new `system_param.rs`).
    *   Define the trait with necessary associated types and methods. Key elements:
        *   `type Item<'w, 's>`: The type the system function actually receives (e.g., `Res<'w, T>`, `Query<'w, 's, ...>`).
        *   `fn fetch(world: &'w World, system_state: &'s mut SystemState) -> Self::Item<'w, 's>`: Method to get the data from the world. `'w` is world lifetime, `'s` is system state lifetime.
        *   `fn access() -> SystemAccess`: Static method to declare data dependencies.
        *   `type State: Send + Sync + 'static`: Associated type for any system-local state needed (e.g., for change detection in queries).
        *   `fn init_state(world: &mut World) -> Self::State`: Initialize the local state.
    *   *Note:* Lifetimes (`'w`, `'s`) are crucial here and need careful consideration.
2.  **Implement `SystemParam` for `Res<T>`:**
    *   Implement the trait for `Res<T>`.
    *   `access()` should return `DataAccess::read` for resource `T`.
    *   `fetch()` should call `world.get_resource::<T>()`.
    *   `State` can likely be `()`.
3.  **Implement `SystemParam` for `ResMut<T>`:**
    *   Implement the trait for `ResMut<T>`.
    *   `access()` should return `DataAccess::write` for resource `T`.
    *   `fetch()` needs mutable world access, which complicates things. This highlights the need for `World` refactoring (Task 2). Initially, might require `unsafe` or careful borrow splitting in `World`.
    *   `State` can likely be `()`.

**Validation:**
*   `SystemParam` trait compiles.
*   Implementations for `Res` and `ResMut` compile (potentially with temporary `todo!()` or `unsafe` blocks for `fetch` pending Task 2).
*   `access()` methods return correct `SystemAccess` information.

---

## Task 2: Refactor `World` for Concurrent Access

**Goal:** Modify `World` internals to safely support concurrent data access required by `SystemParam`s.

**Subtasks:**

1.  **Analyze Storage:** Review `ComponentStorage` and `ResourceStorage`.
2.  **Resource Safety:** Modify `ResourceStorage` to allow concurrent immutable borrows (`Res`) and exclusive mutable borrows (`ResMut`). This likely involves using `RwLock` or similar interior mutability patterns for the resource `HashMap` or individual resource entries. Update `get_resource`/`get_resource_mut`.
3.  **Component Safety (Initial Approach):**
    *   Focus on enabling safe concurrent access for *different* component types first. Accessing the *same* component type mutably from multiple threads still requires archetype-level locking or more complex strategies (defer full archetype refactor if too complex for M2.5).
    *   Modify `ComponentStorage` access methods (`get_component`, `get_component_mut`, `query`, `query_mut`). This might involve:
        *   Using `unsafe` code carefully to provide split borrows when different component types are requested.
        *   Returning iterators or views that manage borrow lifetimes correctly.
    *   *Crucial:* Ensure that obtaining mutable access to one component type doesn't prevent immutable access to another type concurrently.

**Validation:**
*   `World` compiles with internal changes.
*   Existing tests for component/resource access still pass.
*   New unit tests demonstrate safe concurrent access patterns (e.g., getting `Res<A>` while getting `ResMut<B>`, getting `&ComponentA` while getting `&mut ComponentB`).

---

## Task 3: Implement `Query` System Parameter

**Goal:** Create the primary mechanism for accessing component data within systems.

**Subtasks:**

1.  **Define `Query<'w, 's, F>` Struct:**
    *   `F` represents the fetched component data filter/tuple (e.g., `&Position`, `(&Position, &mut Velocity)`).
    *   Store necessary state (e.g., references to `World`'s component storage, potentially state for change detection).
2.  **Implement `SystemParam` for `Query`:**
    *   `access()`: Analyze the type `F` to determine component `TypeId`s and required `AccessType` (Read/Write) for each.
    *   `fetch()`: Use the refactored `World` methods to obtain safe access to the required component storages based on `F`. Return an iterator-like object that yields the requested component tuples for matching entities.
    *   `State`: Might be needed for query caching or change detection (start with `()` if not implementing change detection yet).
3.  **Handle Tuples:** Ensure `Query` works with tuples of components (e.g., `Query<(&Position, &mut Velocity)>`).

**Validation:**
*   `Query` struct and `SystemParam` implementation compile.
*   `access()` correctly reports dependencies for various `Query` types.
*   Unit tests demonstrate fetching data via `Query` for single components, tuples, mutable/immutable combinations.

---

## Task 4: Refactor `System` Trait and Invocation

**Goal:** Adapt the core `System` definition to use `SystemParam`.

**Subtasks:**

1.  **Define `SystemFunction<Marker, F>`:** Create a generic struct or trait that wraps the user's system function `F`. `Marker` might hold parameter types.
2.  **Implement `System` for `SystemFunction`:**
    *   The `access` method will collect access information from all `SystemParam`s defined by `F`'s arguments.
    *   The `run` method will:
        *   Fetch the required data for each parameter using `SystemParam::fetch`.
        *   Call the user's function `F` with the fetched data.
3.  **Update `SystemScheduler`/`SystemRegistry`:** Modify how systems are stored and invoked to accommodate the new `SystemFunction` wrapper and its `run` logic.

**Validation:**
*   New system structures compile.
*   Scheduler can register and invoke systems defined via the new mechanism (tested with simple mock systems).

---

## Task 5: Update `#[system]` Macro

**Goal:** Make the macro ergonomic and leverage `SystemParam`.

**Subtasks:**

1.  **Parameter Analysis:** Modify the macro to identify function parameters whose types implement `SystemParam` (e.g., `Query<...>`, `Res<...>`, `ResMut<...>`).
2.  **Code Generation:**
    *   Generate the `SystemFunction` wrapper struct for the user's function.
    *   Generate the `impl System` block for the wrapper. The `access` and `run` implementations will delegate to the `SystemParam` methods of the function's parameters.
3.  **Remove Limitations:** The previous borrow-checking error messages for unsupported combinations should no longer be necessary, as `SystemParam` and the refactored `World` handle safety.

**Validation:**
*   Macro compiles.
*   Applying `#[system]` to functions with various `SystemParam` combinations (including previously unsupported ones like `Query<&mut Pos>, Res<Time>`) generates correct code.
*   Generated code compiles and integrates with the scheduler.

---

## Task 6: Enable Parallel Execution in Scheduler

**Goal:** Use `rayon` to execute systems within stages concurrently.

**Subtasks:**

1.  **Modify `SystemScheduler::run`:**
    *   Keep the stage calculation logic from M2.4.
    *   Within the loop iterating through stages:
        *   Use `rayon::scope` or `stage.par_iter()` (from the `rayon::prelude::*` trait) to iterate over the system indices in the current stage.
        *   Inside the parallel execution block, fetch the corresponding system runner (`SystemFunction`) and call its `run` method. This is now safe because `run` takes `SystemParam`s, not `&mut World`.

**Validation:**
*   Scheduler compiles with parallel execution logic.
*   Integration tests confirm that systems within the same stage are potentially executed concurrently (verification might involve timing or observing interleaved logging, though true parallelism depends on hardware and workload).

---

## Task 7: Refactor Example Systems & Runner

**Goal:** Update existing examples to use the new ergonomic systems.

**Subtasks:**

1.  **Update `eusociety-simulation`:**
    *   Modify `RandomMovementSystem` and `ResourceUsingSystem` (or their equivalents) to be functions annotated with `#[system]`.
    *   Use `Query<&mut Position>` and `Res<DeltaTime>` as parameters.
    *   Remove the manual `impl System` blocks.
2.  **Update `eusociety-runner`:**
    *   Ensure the runner correctly imports and registers the macro-generated system structs (e.g., `RandomMovementSystem`).

**Validation:**
*   Simulation crate compiles with macro-based systems.
*   Runner compiles and registers the new systems correctly.
*   The simulation runs as expected using the parallel scheduler and macro-generated systems.

---

## Task 8: Benchmarking Setup & Initial Benchmarks

**Goal:** Establish a baseline for performance measurement.

**Subtasks:**

1.  **Add `criterion` Dependency:** Add to workspace and relevant crate (`eusociety-core` or a new `benches` crate).
2.  **Create Benchmark Suite:** Set up `benches/my_benchmark.rs`.
3.  **Implement Basic Scheduler Benchmark:**
    *   Create a benchmark function that sets up a `World` and `SystemScheduler`.
    *   Register a set of simple systems, including some that conflict and some that don't.
    *   Benchmark the execution time of `scheduler.run(&mut world)` using `criterion`.
    *   Compare sequential execution (force single stage or use M2.3 scheduler) vs. parallel execution (M2.5 scheduler).

**Validation:**
*   Benchmarks compile and run via `cargo bench`.
*   Initial results show a performance difference (hopefully improvement) between sequential and parallel scheduling for the test case.

---

## Task 9: Documentation

**Goal:** Update documentation for the new architecture.

**Subtasks:**

1.  **Update `Documentation.md`:** Explain `SystemParam`, how to define systems using the improved macro, and the parallel execution capabilities. Mention clearly the system was inspired by Bevy.
2.  **Update Code Docs:** Document the `SystemParam` trait, implementations (`Query`, `Res`, `ResMut`), the refactored `System` trait, and the parallel scheduler logic.

**Validation:** Documentation is clear, accurate, and reflects the new architecture.

---

**Overall Validation for M2.5:** The simulation runs correctly using the parallel scheduler. Systems defined using `#[system]` with various `SystemParam` combinations compile and execute correctly, potentially in parallel. Benchmarks show a performance difference compared to purely sequential execution. The macro is significantly more ergonomic.
