# Pheromone System Refactor Plan

This document outlines the plan to refactor the pheromone system based on the following requirements:

1.  Pheromone secretion strength decreases linearly as the time since the ant was at the nest/food source increases.
2.  Pheromone strength decays exponentially (faster decay at higher strength).
3.  Ants follow the gradient of pheromone strength within their sample radius, without distance scaling.

## Phase 1: Data Structure & State Management Updates

1.  **Modify `Ant` Component:** (`src/simulation/components/mod.rs`)
    *   Add `pub time_since_last_source: f32` field to the `Ant` struct.
    *   Update ant spawning logic (e.g., `ant_setup.rs`) to initialize `time_since_last_source: 0.0`.
    *   Update queries using `With<Ant>` to query `&Ant` or `&mut Ant`.

2.  **Update Ant State Logic:** (`src/simulation/systems/ant_logic.rs`)
    *   Modify `ant_state_machine_system` query `query_ants` to include `&mut Ant`.
    *   Reset `ant.time_since_last_source = 0.0;` when state changes to `ReturningToNest` or `Foraging`.
    *   Create a *new system* (`update_ant_timers_system`) to run *before* `pheromone_deposit_system`. This system increments `ant.time_since_last_source += time.delta_seconds;` for every ant. Add this system to the simulation schedule.

## Phase 2: Pheromone System Implementation

3.  **Implement Time-Based Secretion Strength (Linear):** (`src/simulation/systems/pheromones.rs`)
    *   Update `pheromone_deposit_system`'s `ant_query` to include `&Ant`.
    *   Define new constants: `MAX_STRENGTH`, `MIN_STRENGTH`, `MAX_TIME_AWAY` (configurable).
    *   Calculate strength linearly: `strength = (MAX_STRENGTH - MIN_STRENGTH) * (1.0 - (ant.time_since_last_source / MAX_TIME_AWAY).clamp(0.0, 1.0)) + MIN_STRENGTH`.
    *   Use this calculated `strength` when spawning `Pheromone` entities.

4.  **Implement Strength-Based Decay (Exponential):** (`src/simulation/systems/pheromones.rs`)
    *   Remove the `Timer` component from `Pheromone` entities (in deposit system) and the `pheromone_decay_system` query.
    *   Define new constants: `STRENGTH_DECAY_RATE`, `MIN_STRENGTH_THRESHOLD` (configurable).
    *   In `pheromone_decay_system`, update strength: `pheromone.strength -= STRENGTH_DECAY_RATE * pheromone.strength * time.delta_seconds;`. Clamp strength at 0.
    *   Change the despawn condition to `pheromone.strength < MIN_STRENGTH_THRESHOLD`.

5.  **Implement Gradient Following (No Distance Scaling):** (`src/simulation/systems/pheromones.rs`)
    *   In `pheromone_follow_system`:
        *   Keep the check for the correct `target_pheromone_type`.
        *   Calculate `direction = pheromone_pos.as_vec2() - ant_pos.as_vec2();`.
        *   Remove distance calculations and weighting.
        *   Calculate contribution: `contribution = direction.normalize_or_zero() * pheromone.strength;`.
        *   Sum contributions into `resultant_vector`.
        *   Remove the inversion logic for `Foraging` state.
        *   Assign `influence.vector = resultant_vector;`.

## Phase 3: Configuration and Tuning

6.  **Configuration:**
    *   Move new constants (`MAX_STRENGTH`, `MIN_STRENGTH`, `MAX_TIME_AWAY`, `STRENGTH_DECAY_RATE`, `MIN_STRENGTH_THRESHOLD`) into a configuration file/struct.
    *   Update systems to read these values from the config resource.

7.  **Testing & Tuning:**
    *   Run the simulation, observe behavior, and verify the new mechanics.
    *   Tune configuration values to achieve desired results.

## Logic Flow Diagram

```mermaid
graph TD
    subgraph Ant Logic Update [ant_logic.rs]
        A[Ant Interacts: Nest or Food] --> B{State Change?};
        B -- Yes --> C[Reset ant.time_since_last_source = 0.0];
        B -- No --> D[State Unchanged];
        C --> E[Update AntState Component];
        D --> E;
    end

    subgraph Ant Timer Update [new system]
        F[For Each Ant] --> G[ant.time_since_last_source += delta_time];
    end

    subgraph Pheromone Deposit [pheromones.rs]
        H[Ant Deposit Check: Timer & Probability] --> I{Deposit Allowed?};
        I -- Yes --> J[Read ant.time_since_last_source];
        J --> K[Calculate Strength = linear_f(time_since_last_source)];
        K --> L[Spawn Pheromone Entity with calculated Strength];
        I -- No --> M[No Deposit];
    end

    subgraph Pheromone Decay [pheromones.rs]
        N[For Each Pheromone] --> O[Read pheromone.strength];
        O --> P[Calculate Decay = RATE * strength];
        P --> Q[pheromone.strength -= Decay * delta_time];
        Q --> R{strength < Threshold?};
        R -- Yes --> S[Despawn Pheromone];
        R -- No --> T[Keep Pheromone];
    end

    subgraph Pheromone Following [pheromones.rs]
        U[For Each Ant] --> V[Find Nearby Relevant Pheromones];
        V --> W[Initialize Influence Vector = Zero];
        W --> X[For Each Nearby Pheromone];
        X --> Y[Calculate Direction = PheromonePos - AntPos];
        Y --> Z[Weight = pheromone.strength];
        Z --> AA[Contribution = Direction.normalize() * Weight];
        AA --> BB[Add Contribution to Influence Vector];
        X -- More Pheromones --> X;
        X -- No More Pheromones --> CC[Set Ant's PheromoneInfluence Component];
    end

    E --> F;
    G --> H;
    CC --> AntMovementSystem[ant_movement.rs uses Influence];

    style K fill:#f9f,stroke:#333,stroke-width:2px
    style P fill:#f9f,stroke:#333,stroke-width:2px
    style Z fill:#f9f,stroke:#333,stroke-width:2px