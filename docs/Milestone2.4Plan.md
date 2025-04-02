# Milestone 2.4 Plan: Parallel Scheduling Foundation

**Goal:** Implement the core logic for a parallel-capable scheduler by analyzing system dependencies, calculating execution stages, and introducing `rayon` for future parallel execution patterns. Due to the current `System::run(&mut World)` signature, true parallel *system execution* is limited, so this milestone focuses on correct sequential staging based on dependencies.

---

## Task 1: Dependency Graph Representation

**Goal:** Define data structures to represent the dependencies between systems.

**Subtasks:**

1.  **Choose Graph Representation:**
    *   Decide on a suitable graph structure (e.g., adjacency list, adjacency matrix) to store systems as nodes and dependencies as edges. An adjacency list is likely simpler.
    *   Location: `eusociety-core/src/ecs/scheduler.rs` (or a new `dependency_graph.rs` module).
2.  **Define Node/Edge Types:**
    *   Nodes will represent system indices (referring to the order in the `SystemRegistry`).
    *   Edges will represent a "must run before" relationship due to data conflicts (Write-Read, Write-Write, Read-Write).
3.  **Implement Graph Construction Logic:**
    *   Create a function `build_dependency_graph(access_patterns: &[SystemAccess]) -> DependencyGraph`.
    *   This function iterates through all pairs of systems.
    *   For each pair, it uses the existing `SystemAccess::conflicts_with` method (or similar logic) to check for conflicts.
    *   If system `j` must run after system `i` due to a conflict (e.g., `j` reads what `i` writes), add a directed edge from `i` to `j`.

**Validation:**
*   Graph data structures compile.
*   Unit tests for `build_dependency_graph` with various conflicting and non-conflicting `SystemAccess` patterns produce the correct graph structure (correct nodes and edges).

---

## Task 2: Execution Stage Calculation

**Goal:** Implement an algorithm to group systems into sequential stages based on the dependency graph, where systems within a stage have no dependencies on each other.

**Subtasks:**

1.  **Algorithm Selection:** Choose an algorithm for topological sorting or stage calculation. Kahn's algorithm (using in-degrees) is a common choice for finding parallelizable stages.
2.  **Implement Stage Calculation Function:**
    *   Create a function `calculate_execution_stages(graph: &DependencyGraph) -> Vec<Vec<usize>>`. `usize` represents the system index.
    *   Implement the chosen algorithm (e.g., Kahn's):
        *   Calculate in-degrees for all nodes (systems).
        *   Initialize a queue with nodes having an in-degree of 0.
        *   While the queue is not empty:
            *   Dequeue all nodes currently in the queue – these form the *current stage*.
            *   Add the current stage to the list of stages.
            *   For each node `u` in the current stage:
                *   For each neighbor `v` of `u`:
                    *   Decrement the in-degree of `v`.
                    *   If the in-degree of `v` becomes 0, enqueue `v`.
    *   Handle cycles (optional but recommended): If the algorithm finishes but not all systems are staged, a dependency cycle exists. The function should detect this and return an error or panic.
3.  **Data Structure for Stages:** The result `Vec<Vec<usize>>` represents the stages, where each inner `Vec` contains the indices of systems that can run concurrently within that stage.

**Validation:**
*   Stage calculation function compiles.
*   Unit tests with different dependency graphs (linear dependencies, parallel branches, complex graphs) produce the correct execution stages.
*   Unit tests verify cycle detection.

---

## Task 3: Integrate `rayon` Dependency

**Goal:** Add `rayon` to the project for parallel iteration capabilities.

**Subtasks:**

1.  **Add `rayon` to `Cargo.toml`:**
    *   Add `rayon` to the `[dependencies]` section of `eusociety-core/Cargo.toml`. Use workspace inheritance if applicable (`rayon.workspace = true`).
    *   Update the root `Cargo.toml`'s `[workspace.dependencies]` if necessary.
2.  **Verify Dependency:** Run `cargo check` or `cargo build` to ensure the dependency is correctly resolved.

**Validation:** Project compiles with the added `rayon` dependency.

---

## Task 4: Update Scheduler to Build Graph & Stages

**Goal:** Modify the `SystemScheduler` to utilize the dependency graph and stage calculation logic.

**Subtasks:**

1.  **Store Stages:**
    *   Modify `SystemScheduler` (or `SystemRegistry`) to store the calculated execution stages (`Vec<Vec<usize>>`).
    *   Alternatively, calculate stages dynamically each time `run` is called (simpler for now, but less efficient if systems don't change often). Let's start with dynamic calculation within the `run` method.
2.  **Modify `add_system` (Optional):**
    *   The current `add_system` checks for *any* conflict. This might be too strict for staging. We might remove the conflict check here, as the staging calculation will handle ordering. The `add_system_unchecked` becomes the default way to add systems.
    *   *Decision:* Keep the conflict check in `add_system` for now as an immediate feedback mechanism, but rely on staging for execution order. The runner currently uses `add_system_unchecked` anyway when conflicts are detected.

**Validation:**
*   Scheduler code compiles.
*   Registration logic remains functional.

---

## Task 5: Update Scheduler `run` Method

**Goal:** Modify the scheduler's `run` method to execute systems stage by stage, potentially using `rayon` for intra-system parallelism if systems are designed for it.

**Subtasks:**

1.  **Calculate Graph and Stages:** Inside the `run` method, call `build_dependency_graph` and `calculate_execution_stages` using the registered systems' access patterns.
2.  **Iterate Through Stages:** Loop through the calculated `stages: Vec<Vec<usize>>`.
3.  **Execute Systems within a Stage:**
    *   For each stage (a `Vec<usize>` of system indices):
        *   **Sequential Execution (M2.4 Focus):** Iterate through the system indices in the current stage and call `system.run(&mut world)` for each one sequentially. The `&mut World` borrow prevents parallel execution of the `run` methods themselves across different systems within the stage.
        *   **Future Parallelism Hook (Optional Prep for M2.5):** Although we can't run `system.run(&mut world)` in parallel *between* systems easily, we *could* use `rayon`'s `par_iter()` on the system indices *within* a stage if the `run` method itself was internally parallel and didn't require `&mut World`. This requires changing the `System` trait and `World` significantly, so we **defer** this. The current sequential execution per stage is the main goal for M2.4.
4.  **Update `SystemRegistry::run_systems`:** Modify this method (or remove it if logic moves entirely to `SystemScheduler::run`) to accept the calculated stages or handle the stage-by-stage execution. It's likely cleaner to move the core execution logic to `SystemScheduler::run`.

**Validation:**
*   Scheduler `run` method compiles.
*   Simulation runs correctly, executing systems in an order consistent with their dependencies (verifiable through logging or specific system interactions). Systems that *could* run in parallel (based on dependencies) are grouped into the same stage, even if executed sequentially within that stage for now.

---

## Task 6: Add/Update Tests for Scheduling Logic

**Goal:** Ensure the new scheduling logic based on dependencies and stages works correctly.

**Subtasks:**

1.  **Create Test Systems:** Define several mock systems with varying dependencies (read/write on different components/resources). Include systems that can run in parallel and systems that must run sequentially.
2.  **Test Stage Calculation:** Write unit tests in `scheduler.rs` that register these test systems, build the graph, calculate stages, and assert that the resulting stages are correct and respect dependencies.
3.  **Test Execution Order:** Write integration-style tests (potentially in the runner or using the scheduler directly) that register test systems designed to modify data in a specific order. Verify that after `scheduler.run` is called, the final state of the `World` reflects the correct execution order imposed by the dependencies and stages.

**Validation:** All new and existing tests pass.

---

## Task 7: Documentation Update

**Goal:** Document the new scheduling mechanism and its current limitations.

**Subtasks:**

1.  **Update `scheduler.rs` Docs:** Explain the dependency graph, stage calculation, and the sequential execution of stages.
2.  **Update `System` Trait Docs:** Mention that dependencies declared via `access()` are now used for scheduling order.
3.  **Update Architecture Docs (Optional):** Briefly describe the new dependency-aware scheduling approach in `docs/ARCHITECTURE.md`.
4.  **Document Limitations:** Clearly state that true parallel system execution is currently limited by the `System::run(&mut World)` signature and that complex `World` refactoring is needed for further parallelism (mentioning M2.5).

**Validation:** Documentation is clear, accurate, and reflects the implemented changes and limitations.

---

**Overall Validation for M2.4:** The simulation runs correctly with the new scheduler. Systems are executed in an order that respects their data dependencies, grouped into stages. Although stages are run sequentially, the foundation for parallel execution is in place via `rayon` and stage calculation. Conflict detection during registration might be relaxed or adjusted depending on final implementation choices.
