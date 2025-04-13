# Polygon Wall Implementation Plan

This document outlines the plan to implement polygon-based walls that ants will bounce off of within the simulation.

## Phase 1: Configuration and Data Structures

1.  **Define Polygon Structure:**
    *   **File:** `src/config/types.rs`
    *   **Action:** Add `Point` and `PolygonWall` structs. Update `SimulationConfig` to include `walls: Vec<PolygonWall>`. Use `serde` attributes.
    *   **Rationale:** Defines the Rust data structures for walls and integrates them into the main configuration, allowing loading from JSON. `#[serde(default)]` makes it backward compatible.
    *   **Code:**
        ```rust
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct Point {
            pub x: f32,
            pub y: f32,
        }

        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct PolygonWall {
            pub vertices: Vec<Point>,
        }

        // ... existing SimulationConfig struct ...
        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct SimulationConfig {
            // ... other fields ...
            pub world_dimensions: (f32, f32),
            pub boundary_behavior: BoundaryBehavior,
            #[serde(default)] // Make walls optional
            pub walls: Vec<PolygonWall>,
            // ... other fields ...
        }
        ```

2.  **Update `config.json`:**
    *   **File:** `config.json`
    *   **Action:** Add a `walls` array with example polygon definitions.
    *   **Rationale:** Provides sample data for testing and initial use.
    *   **Code:**
        ```json
        {
          // ... other config ...
          "world_dimensions": [800.0, 600.0],
          "boundary_behavior": "Bounce",
          "walls": [
            {
              "vertices": [
                { "x": 100.0, "y": 100.0 },
                { "x": 200.0, "y": 100.0 },
                { "x": 150.0, "y": 200.0 }
              ]
            },
            {
              "vertices": [
                { "x": 400.0, "y": 300.0 },
                { "x": 500.0, "y": 300.0 },
                { "x": 500.0, "y": 400.0 },
                { "x": 400.0, "y": 400.0 }
              ]
            }
          ],
          // ... other config ...
        }
        ```

## Phase 2: Simulation Integration

3.  **Create Wall Resource:**
    *   **File:** `src/simulation/resources/mod.rs` (or new `walls.rs`)
    *   **Action:** Define a Bevy resource `WallGeometry` holding `Vec<PolygonWall>`.
    *   **Rationale:** Makes wall data accessible globally to Bevy systems.
    *   **Code:**
        ```rust
        use bevy_ecs::prelude::*;
        use crate::config::PolygonWall;

        #[derive(Resource, Debug, Clone)]
        pub struct WallGeometry {
            pub polygons: Vec<PolygonWall>,
        }
        ```

4.  **Load Walls into Resource:**
    *   **File:** `src/simulation/systems/setup.rs` (or `environment_setup.rs`)
    *   **Action:** Modify the setup system to read walls from `SimulationConfigResource` and insert the `WallGeometry` resource.
    *   **Rationale:** Populates the resource at simulation startup.
    *   **Code:**
        ```rust
        // Inside the relevant setup system function:
        fn setup_environment(
            mut commands: Commands,
            config_res: Res<SimulationConfigResource>,
            // ... other params ...
        ) {
            // ... existing setup code ...

            // Load walls
            let wall_geometry = WallGeometry {
                polygons: config_res.0.walls.clone(),
            };
            commands.insert_resource(wall_geometry);

            // ... rest of setup code ...
        }
        ```

## Phase 3: Collision Logic

5.  **Create Wall Collision System:**
    *   **File:** `src/simulation/systems/wall_collision.rs` (New file)
    *   **Action:** Create the system `handle_wall_collisions`. Implement line segment-polygon intersection logic. On collision, calculate the reflection normal, adjust velocity (reflect & dampen), and move the ant to the exact collision point.
    *   **Rationale:** Core logic for detecting and handling wall bounces. Requires careful geometric calculations.

6.  **Integrate New System:**
    *   **File:** `src/simulation/systems/mod.rs`
    *   **Action:** Add `pub mod wall_collision;`.
    *   **File:** `src/main.rs` (or schedule definition)
    *   **Action:** Add `handle_wall_collisions` to the `Update` schedule, likely after movement calculation and before boundary handling, using `.chain()` if necessary for ordering.
    *   **Rationale:** Plugs the new system into the simulation loop in the correct order.
    *   **Code:**
        ```rust
        // Example schedule modification
        app.add_systems(Update,
            (
                // ... other systems ...
                ant_movement::move_ants,
                wall_collision::handle_wall_collisions,
                boundary::handle_boundaries,
                // ... systems that finalize position ...
            ).chain()
        );
        ```

## Phase 4: Visualization (Optional)

7.  **Expose Wall Data:**
    *   **File:** `src/simulation/systems/state_export.rs`
    *   **Action:** Modify the state export system to include `WallGeometry` data.
    *   **Rationale:** Makes wall data available to the frontend.

8.  **Draw Walls in Frontend:**
    *   **File:** `frontend/main.js`
    *   **Action:** Update frontend code to receive wall data and draw the polygons using Canvas/WebGL.
    *   **Rationale:** Visual confirmation of wall placement and behavior.

## Diagram

```mermaid
graph TD
    subgraph Setup Phase
        A[Load config.json] --> B(Parse SimulationConfig incl. Walls);
        B --> C[Create WallGeometry Resource];
        C --> D[Insert WallGeometry into Bevy World];
    end

    subgraph Simulation Update Loop
        E[ant_movement System] --> F(Calculate Potential Next Position & Velocity);
        F --> G[handle_wall_collisions System];
        G -- Reads --> D;
        G --> H{Collision with Wall?};
        H -- Yes --> I[Calculate Reflection, Adjust Velocity, Set Position to Collision Point];
        H -- No --> J[Keep Calculated Position/Velocity];
        I --> K[Adjusted Position/Velocity];
        J --> K;
        K --> L[handle_boundaries System];
        L --> M[Finalize Frame Update];
    end

    subgraph Data Export / Visualization (Optional)
        N[state_export System] -- Reads --> D;
        N --> O[Serialize State (incl. Walls)];
        O --> P[Send State to Frontend];
        P --> Q[Frontend main.js];
        Q --> R[Draw Ants];
        Q --> S[Draw Walls];
    end