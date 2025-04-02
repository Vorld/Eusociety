# Milestone 2.2: Resource Management - Detailed Plan

**Goal:** Add the ability to store and access shared, global data (resources) within the `World`.

---

## Phase 1: Core Resource Implementation (`eusociety-core`)

1.  **Define `Resource` Trait:**
    *   **Location:** `eusociety-core/src/lib.rs`
    *   **Action:** Define `pub trait Resource: 'static + Send + Sync {}`.
    *   **Impact:** Minimal.
    *   **Documentation Note:** Add clear documentation for the `Resource` trait explaining:
        - The purpose of resources as global shared state
        - When to use resources vs. components (state shared across systems vs. entity-specific data)
        - That resources are singletons (only one instance per type)
        - Common use cases (game time, physics constants, game state, etc.)
        - Document that resources may be accessed concurrently in the future:

2.  **Implement `ResourceStorage`:**
    *   **Location:** `eusociety-core/src/lib.rs`
    *   **Action:** Create `pub struct ResourceStorage` with a `resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>` field. Implement `Default`.
    *   **Impact:** Minimal.

3.  **Implement Accessors for `ResourceStorage`:**
    *   **Location:** `eusociety-core/src/lib.rs` (methods on `ResourceStorage`)
    *   **Action:** Add `insert`, `get`, `get_mut`, `remove`, `contains` methods using `TypeId` and `downcast`.
    *   **Impact:** Minimal.

4.  **Integrate `ResourceStorage` into `World`:**
    *   **Location:** `eusociety-core/src/lib.rs`
    *   **Action:** Add `pub resources: ResourceStorage` field to `World`. Update `Default`/`new`.
    *   **Impact:** Minor Structural Change.

5.  **Add Convenience Accessors to `World`:**
    *   **Location:** `eusociety-core/src/lib.rs` (methods on `World`)
    *   **Action:** Add `insert_resource`, `get_resource`, `get_resource_mut`, etc., methods delegating to `world.resources`.
    *   **Impact:** Minimal.

6.  **Add Unit Tests:**
    *   **Location:** `eusociety-core/src/lib.rs` (`#[cfg(test)]`)
    *   **Action:** Create tests for resource management via `World` methods (insert, get, get_mut, remove, contains, non-existent).
    *   **Impact:** Minimal.

---

## Phase 2: Implement and Use `DeltaTime`

1.  **Implement `DeltaTime` Resource:**
    *   **Location:** `eusociety-core/src/lib.rs`
    *   **Action:** Define `pub struct DeltaTime { pub delta: Duration }` (or `f64`). Implement `Resource` and `Default`. Add `use std::time::Duration;`.
    *   **Impact:** Minimal.

2.  **Initialize & Update `DeltaTime` in Runner:**
    *   **Location:** `eusociety-runner/src/main.rs`
    *   **Action:**
        *   Initialize: `world.insert_resource(DeltaTime::default());` before the loop.
        *   Update: Inside the loop, calculate `elapsed_time` and update the resource via `world.get_resource_mut::<DeltaTime>()`. Handle potential `None` case (e.g., with `warn!`).
    *   **Impact:** Moderate Change to Runner.

3.  **Demonstrate Resource Usage in System:**
    *   **Location:** `eusociety-simulation/src/lib.rs`
    *   **Action:**
        *   Modify `random_movement_system` to fetch `DeltaTime` using `world.get_resource::<DeltaTime>()`.
        *   Log the fetched delta time value (`log::debug!(...)`) inside the system. **Do not change movement logic yet.**
        *   Update the system's unit tests to insert a `DeltaTime` resource during setup.
    *   **Impact:** Minor Change to Simulation System (read-only access for now).

---

## Phase 3: Configuration (Optional but Recommended)

1.  **Update Config Struct:**
    *   **Location:** `eusociety-config/src/lib.rs`
    *   **Action:** Add `pub initial_resources: Option<HashMap<String, serde_json::Value>>` to `Config` or a sub-struct. Use `#[serde(default)]`.
    *   **Impact:** Minor Change to Config.

2.  **Update Runner Initialization:**
    *   **Location:** `eusociety-runner/src/main.rs`
    *   **Action:** If `config.initial_resources` is `Some`, iterate, deserialize values based on keys (e.g., `"DeltaTime"`), and insert into `world`. Handle errors.
    *   **Impact:** Moderate Change to Runner.

---

## Validation

*   Run unit tests in `eusociety-core` and `eusociety-simulation` (`cargo test`).
*   Run the main simulation (`cargo run` in `eusociety-runner`).
*   Check logs for `DeltaTime` debug messages.
*   Verify simulation runs correctly and outputs data as expected.

---

## Key Considerations

*   **Impact:** Changes are introduced incrementally, starting with core structures and tests.
*   **Future Refactoring (M2.3):** Milestone 2.3 will introduce a `System` trait with formal dependency declarations, changing resource/component access. This plan is compatible with the current `fn(&mut World)` system signature.
