# Plan: Phase 2 - Pheromone Trails (Inverted Vector Model with Quadtree)

**Goal:** Implement `FoodTrail` and `HomeTrail` pheromones that influence ant movement. Ants returning to the nest will follow `HomeTrail` pheromones using an inverted vector approach (moving *away* from the trail's source direction), while foraging ants will follow `FoodTrail` pheromones normally. Pheromone lookup will leverage a dedicated Quadtree for efficient spatial querying.

## Core Concepts

*   **Pheromones:** Entities with a type (`FoodTrail`, `HomeTrail`), strength (decaying over time), and position.
*   **Quadtree (`PheromoneQuadtree`):** A spatial index storing `(Entity, Position)` of active pheromones for fast proximity queries. This will be separate from the `FoodQuadtree`.
*   **Inverted Vector Model:** When returning to the nest, ants calculate the resultant vector from nearby `HomeTrail` pheromones and then *invert* it to guide their movement back towards the nest (away from the direction other ants came *from* the nest).
*   **Influence Blending:** The calculated pheromone influence vector will be blended with the ant's existing random walk behavior.

## Implementation Steps

1.  **Quadtree Adaptation (`src/simulation/spatial.rs` or new file):**
    *   **Goal:** Create a `PheromoneQuadtree` resource analogous to `FoodQuadtree`.
    *   **Tasks:**
        *   Duplicate or generalize the `QuadTreeNode` and related `Rect` logic from `spatial.rs`.
        *   Define `PheromoneQuadtree` as a new Bevy `Resource`.
        *   Ensure it stores `(Entity, Position)` pairs.
        *   Implement `insert(Entity, Position)`, `query_range(&Rect) -> Vec<&(Entity, Position)>`, and `remove(Entity, &Position) -> bool` methods.
        *   Initialize and register the `PheromoneQuadtree` resource in `main.rs` or `simulation/mod.rs` app setup (likely with the world boundary, initially empty).

2.  **Component Definitions (`src/simulation/components/mod.rs`):**
    *   **Goal:** Define the necessary data components for pheromones and their influence.
    *   **Tasks:**
        *   Define `PheromoneType` enum: `FoodTrail`, `HomeTrail`.
        *   Define `Pheromone` component: `{ type: PheromoneType, strength: f32 }`. Add `#[derive(Component)]`.
        *   Define `PheromoneInfluence` component: `{ vector: Vec2 }` (using `glam::Vec2`). This will store the calculated influence on an ant. Add `#[derive(Component, Default)]`. Initialize ants with this component.
        *   Ensure pheromone entities will also have a `Position` component and a `Timer` component (from `bevy::core::Timer`) for decay.

3.  **Pheromone Systems (`src/simulation/systems/pheromones.rs` - New File):**
    *   **Goal:** Implement the core logic for pheromone deposit, decay, and following.
    *   **Tasks:**
        *   **`pheromone_deposit_system`:**
            *   **Query:** `Query<(&Position, &AntState), With<Ant>>`.
            *   **Resources:** `Commands`, `ResMut<PheromoneQuadtree>`.
            *   **Logic:**
                *   Periodically (e.g., using a local timer or checking simulation time), ants spawn new pheromone entities.
                *   `ReturningToNest` ants spawn `FoodTrail` pheromones at their current `Position`.
                *   `Foraging` ants spawn `HomeTrail` pheromones at their current `Position`.
                *   Each spawned pheromone needs: `Pheromone` component (with initial strength), `Position`, and a `Timer` (set to the desired decay duration).
                *   Use `commands.spawn(...)` to create the entity.
                *   **Crucially:** After spawning, immediately insert the new pheromone's `(Entity, Position)` into the `PheromoneQuadtree`.
        *   **`pheromone_decay_system`:**
            *   **Query:** `Query<(Entity, &mut Pheromone, &mut Timer, &Position)>`.
            *   **Resources:** `Commands`, `ResMut<PheromoneQuadtree>`, `Res<Time>`.
            *   **Logic:**
                *   Use `par_iter_mut()` for parallel processing.
                *   Tick the `Timer` for each pheromone using `time.delta()`.
                *   Decrease `Pheromone.strength` based on the timer's progress (e.g., linearly or exponentially).
                *   If `timer.finished()`:
                    *   Get the pheromone's `Position`.
                    *   Remove the pheromone from the `PheromoneQuadtree` using `pheromone_quadtree.remove(entity, position)`. Handle potential errors if removal fails (e.g., log a warning).
                    *   Despawn the pheromone entity using `commands.entity(entity).despawn()`.
        *   **`pheromone_follow_system`:**
            *   **Query:** `Query<(&Position, &AntState, &mut PheromoneInfluence), With<Ant>>`.
            *   **Resources:** `Res<PheromoneQuadtree>`.
            *   **Other Queries:** `Query<&Pheromone>` (to get pheromone data after lookup).
            *   **Logic:**
                *   For each ant:
                    *   Reset `PheromoneInfluence.vector` to `Vec2::ZERO`.
                    *   Define a query `Rect` around the ant's `Position` (using a configured `pheromone_sense_radius`).
                    *   Query the `PheromoneQuadtree` using `pheromone_quadtree.query_range(&query_rect)`.
                    *   Initialize a resultant vector `resultant = Vec2::ZERO`.
                    *   Iterate through the found `(pheromone_entity, pheromone_pos)` pairs:
                        *   Get the `Pheromone` component data for `pheromone_entity` using `pheromones_query.get(pheromone_entity)`.
                        *   Determine the relevant `PheromoneType` based on the ant's `AntState`:
                            *   `Foraging` ant -> interested in `FoodTrail`.
                            *   `ReturningToNest` ant -> interested in `HomeTrail`.
                        *   If the pheromone is of the relevant type:
                            *   Calculate the vector *from* the ant *to* the pheromone: `direction = pheromone_pos - ant_pos`.
                            *   Weight the vector by the `pheromone.strength` and potentially inverse distance squared.
                            *   Add the weighted vector to `resultant`.
                    *   Normalize the `resultant` vector if its length is greater than zero.
                    *   **Inversion Logic:** If the ant's state is `ReturningToNest`, invert the vector: `resultant = -resultant`.
                    *   Store the final vector in the ant's `PheromoneInfluence.vector`.

4.  **Integration (`src/simulation/systems/ant_movement.rs`):**
    *   **Goal:** Blend the pheromone influence with existing movement logic.
    *   **Tasks:**
        *   Modify `ant_movement_system` query to include `&PheromoneInfluence`.
        *   In the movement calculation, retrieve the `influence.vector` from the `PheromoneInfluence` component.
        *   Combine the `influence.vector` (weighted by some factor) with the existing random walk vector before normalizing and applying to velocity. Example: `final_direction = random_walk_vector + influence.vector * PHEROMONE_WEIGHT`.

5.  **Scheduling (`src/simulation/mod.rs` or `main.rs`):**
    *   **Goal:** Ensure systems run in the correct order within the `Update` schedule.
    *   **Tasks:**
        *   Add the new systems to the Bevy app schedule.
        *   Define explicit ordering using `.before()` / `.after()` or system sets. Recommended order:
            1.  `pheromone_deposit_system`
            2.  `pheromone_decay_system`
            3.  `pheromone_follow_system`
            4.  `ant_movement_system` (or the system that uses `PheromoneInfluence`)
            5.  (Other systems like `apply_velocity`, `boundary_check`)

6.  **Frontend Visualization (`frontend/main.js`):**
    *   **Goal:** Display pheromones visually.
    *   **Tasks:**
        *   Modify the state export (`state_export.rs`) to include pheromone data (Position, Type, Strength). Consider delta compression if state becomes large.
        *   Update the WebSocket message parsing in `main.js` to handle pheromone data.
        *   Implement rendering logic (e.g., using small, semi-transparent dots or heat map approach) for `FoodTrail` and `HomeTrail` pheromones, potentially using different colors. The intensity/opacity could represent `strength`.

## Mermaid Diagram (Phase 2 Flow)

```mermaid
graph TD
    subgraph Update Loop
        A(Start Tick) --> B(pheromone_deposit_system);
        B --> C{Ants Spawn Pheromones};
        C --> D{Add Pheromones to Quadtree};
        D --> E(pheromone_decay_system);
        E --> F{Tick Timers, Reduce Strength};
        F --> G{Pheromone Expired?};
        G -- Yes --> H{Remove Pheromone from Quadtree};
        H --> I{Despawn Pheromone Entity};
        G -- No --> J;
        I --> J;
        J(pheromone_follow_system) --> K{Ant Queries Quadtree};
        K --> L{Calculate Resultant Vector};
        L --> M{Ant State?};
        M -- ReturningToNest --> N{Invert Vector};
        M -- Foraging --> O;
        N --> O{Store in PheromoneInfluence};
        O --> P(ant_movement_system);
        P --> Q{Combine Random Walk + PheromoneInfluence};
        Q --> R(apply_velocity_system);
        R --> S(boundary_check_system);
        S --> T(state_export_system);
        T --> U{Send State (incl. Pheromones) to Frontend};
        U --> V(End Tick);
    end

    subgraph Frontend
        W(Receive WebSocket Data) --> X{Parse Pheromone Data};
        X --> Y{Render Pheromones (Dots/Heatmap)};
    end

    V --> A;
    U --> W;
```

## File Structure Changes

*   **New:** `src/simulation/systems/pheromones.rs`
*   **New/Modified:** `src/simulation/spatial.rs` (or a new file like `src/simulation/pheromone_spatial.rs` if separating logic)
*   **Modified:** `src/simulation/components/mod.rs` (add Pheromone components)
*   **Modified:** `src/simulation/systems/ant_movement.rs` (integrate influence)
*   **Modified:** `src/simulation/mod.rs` or `main.rs` (add resources, systems, scheduling)
*   **Modified:** `src/simulation/systems/state_export.rs` (export pheromone data)
*   **Modified:** `frontend/main.js` (parse and render pheromones)