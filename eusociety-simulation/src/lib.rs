use eusociety_core::{Component, System, World, impl_component}; // Added impl_component macro
use rand::Rng;
use serde::Serialize;
use std::any::Any;
use std::collections::HashMap;

// --- Components ---

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
impl_component!(Position);

#[derive(Debug, Clone, Serialize)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}
impl_component!(Velocity);

// --- Resources ---
// Resources are typically singleton data stored in the World

#[derive(Debug, Clone)]
pub struct DeltaTime(pub f32);

#[derive(Debug, Clone)]
pub struct WorldBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

// --- Spatial Partitioning ---
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    cell_size: f32,
    grid: HashMap<(i32, i32), Vec<eusociety_core::Entity>>,
    bounds: WorldBounds,
}

impl SpatialGrid {
    pub fn new(cell_size: f32, bounds: WorldBounds) -> Self {
        SpatialGrid {
            cell_size,
            grid: HashMap::new(),
            bounds,
        }
    }
    
    /// Get the grid cell coordinates for a position
    fn get_cell(&self, pos: &Position) -> (i32, i32) {
        let x = ((pos.x - self.bounds.min_x) / self.cell_size).floor() as i32;
        let y = ((pos.y - self.bounds.min_y) / self.cell_size).floor() as i32;
        
        // Safety check - ensure coordinates are within a reasonable range
        // to prevent excessive memory usage from invalid coordinates
        let x_cells = ((self.bounds.max_x - self.bounds.min_x) / self.cell_size).ceil() as i32;
        let y_cells = ((self.bounds.max_y - self.bounds.min_y) / self.cell_size).ceil() as i32;
        
        // Clamp to valid range
        let x = x.max(0).min(x_cells - 1);
        let y = y.max(0).min(y_cells - 1);
        
        (x, y)
    }
    
    /// Update the grid with all entities that have positions
    pub fn update(&mut self, world: &World) {
        self.grid.clear();
        
        let positions = world.query::<Position>();
        for (entity, pos) in &positions {
            let cell = self.get_cell(pos);
            self.grid.entry(cell).or_insert_with(Vec::new).push(**entity);
        }
    }
    
    /// Find all entities within a radius of a position
    pub fn query_radius(&self, center: &Position, radius: f32) -> Vec<eusociety_core::Entity> {
        let mut result = Vec::new();
        let radius_in_cells = (radius / self.cell_size).ceil() as i32;
        
        let center_cell = self.get_cell(center);
        
        // Check all cells that could contain entities within the radius
        for x_offset in -radius_in_cells..=radius_in_cells {
            for y_offset in -radius_in_cells..=radius_in_cells {
                let cell = (center_cell.0 + x_offset, center_cell.1 + y_offset);
                
                if let Some(entities) = self.grid.get(&cell) {
                    for entity in entities {
                        result.push(*entity);
                    }
                }
            }
        }
        
        result
    }
}

// --- Systems ---

// System to update the spatial grid
#[derive(Default)]
pub struct SpatialGridSystem;

impl System for SpatialGridSystem {
    fn run(&mut self, world: &mut World) {
        // Get world bounds or use defaults
        let bounds = world.get_resource::<WorldBounds>()
            .cloned()
            .unwrap_or_else(|| WorldBounds { 
                min_x: -50.0, max_x: 50.0, 
                min_y: -50.0, max_y: 50.0 
            });
        
        // Create or update the spatial grid
        let mut grid = world.get_resource::<SpatialGrid>()
            .cloned()
            .unwrap_or_else(|| SpatialGrid::new(5.0, bounds.clone()));
        
        // Update the grid with current positions
        grid.update(world);
        
        // Store the updated grid as a resource
        world.add_resource(grid);
    }
}

#[derive(Default)]
pub struct RandomMovementSystem;

impl System for RandomMovementSystem {
    fn run(&mut self, world: &mut World) {
        // Get resources from world or use default values
        let dt = world.get_resource::<DeltaTime>()
            .map_or(1.0 / 60.0, |dt| dt.0);

        let bounds = world.get_resource::<WorldBounds>()
            .cloned()
            .unwrap_or_else(|| WorldBounds { 
                min_x: -50.0, max_x: 50.0, 
                min_y: -50.0, max_y: 50.0 
            });

        // Get all Position components with their entities
        let positions = world.query::<Position>();
        let mut updates = Vec::new();
        
        let mut rng = rand::thread_rng();
        
        // Process each entity with position and velocity
        for (entity, position) in &positions {
            if let Some(velocity) = world.get_component::<Velocity>(**entity) {
                // Create updated position based on velocity
                let mut new_pos = Position {
                    x: position.x + velocity.x * dt,
                    y: position.y + velocity.y * dt,
                };
                
                // Create updated velocity with random perturbation
                let mut new_vel = Velocity {
                    x: velocity.x + rng.gen_range(-0.1..0.1) * dt,
                    y: velocity.y + rng.gen_range(-0.1..0.1) * dt,
                };

                // Clamp velocity magnitude to prevent runaway speeds
                let speed_sq = new_vel.x * new_vel.x + new_vel.y * new_vel.y;
                let max_speed_sq = 5.0 * 5.0; // Max speed of 5.0 units/sec
                if speed_sq > max_speed_sq {
                    let scale = (max_speed_sq / speed_sq).sqrt();
                    new_vel.x *= scale;
                    new_vel.y *= scale;
                }
                
                // Handle world boundaries (wrap around)
                if new_pos.x > bounds.max_x { new_pos.x = bounds.min_x; }
                else if new_pos.x < bounds.min_x { new_pos.x = bounds.max_x; }
                if new_pos.y > bounds.max_y { new_pos.y = bounds.min_y; }
                else if new_pos.y < bounds.min_y { new_pos.y = bounds.max_y; }
                // Store the updates to apply after iteration
                updates.push((**entity, new_pos, new_vel));
            }
        }

        // Apply all updates (to avoid borrowing issues)
        for (entity, new_pos, new_vel) in updates {
            world.add_component(entity, new_pos);
            world.add_component(entity, new_vel);
        }
    }
}

/// Implements flocking behavior (similar to Boids algorithm)
/// Particles will exhibit:
/// 1. Separation - avoid crowding neighbors
/// 2. Alignment - steer towards average heading of neighbors
/// 3. Cohesion - steer towards average position of neighbors
#[derive(Default)]
pub struct FlockingSystem {
    perception_radius: f32,
    separation_weight: f32, 
    alignment_weight: f32,
    cohesion_weight: f32,
}

impl FlockingSystem {
    pub fn new(perception_radius: f32, 
               separation_weight: f32, 
               alignment_weight: f32, 
               cohesion_weight: f32) -> Self {
        Self {
            perception_radius,
            separation_weight,
            alignment_weight,
            cohesion_weight,
        }
    }
}

impl System for FlockingSystem {
    fn run(&mut self, world: &mut World) {
        // Get resources
        let dt = world.get_resource::<DeltaTime>()
            .map_or(1.0 / 60.0, |dt| dt.0);
            
        // Need spatial grid for efficient neighbor queries
        let grid = match world.get_resource::<SpatialGrid>() {
            Some(grid) => grid,
            None => return, // Can't run without spatial grid
        };
        
        // Get world bounds (needed for boundary handling)
        let bounds = world.get_resource::<WorldBounds>()
            .cloned()
            .unwrap_or_else(|| WorldBounds { 
                min_x: -50.0, max_x: 50.0, 
                min_y: -50.0, max_y: 50.0 
            });
        
        // Get all entities with positions and velocities
        let positions = world.query::<Position>();
        let mut updates = Vec::new();
        
        for (entity, pos) in &positions {
            if let Some(vel) = world.get_component::<Velocity>(**entity) {
                // Find neighbors within perception radius
                let neighbors = grid.query_radius(pos, self.perception_radius);
                
                if neighbors.is_empty() {
                    continue;
                }
                
                // Variables for the three flocking rules
                let mut separation = Velocity { x: 0.0, y: 0.0 };
                let mut alignment = Velocity { x: 0.0, y: 0.0 };
                let mut cohesion_pos = Position { x: 0.0, y: 0.0 };
                let mut neighbor_count = 0;
                
                // Process each neighbor
                for neighbor_entity in &neighbors {
                    // Skip self
                    if *neighbor_entity == **entity {
                        continue;
                    }
                    
                    if let Some(neighbor_pos) = world.get_component::<Position>(*neighbor_entity) {
                        // Calculate distance
                        let dx = pos.x - neighbor_pos.x;
                        let dy = pos.y - neighbor_pos.y;
                        let dist_sq = dx * dx + dy * dy;
                        
                        if dist_sq < self.perception_radius * self.perception_radius {
                            // Separation - move away from close neighbors
                            if dist_sq > 0.0001 { // Avoid division by zero
                                let factor = 1.0 / dist_sq.sqrt();
                                separation.x += dx * factor;
                                separation.y += dy * factor;
                            }
                            
                            // Get neighbor velocity for alignment
                            if let Some(neighbor_vel) = world.get_component::<Velocity>(*neighbor_entity) {
                                alignment.x += neighbor_vel.x;
                                alignment.y += neighbor_vel.y;
                            }
                            
                            // Add position for cohesion (average position)
                            cohesion_pos.x += neighbor_pos.x;
                            cohesion_pos.y += neighbor_pos.y;
                            
                            neighbor_count += 1;
                        }
                    }
                }
                
                // Skip if no actual neighbors found
                if neighbor_count == 0 {
                    continue;
                }
                
                // Normalize and apply weights to the three components
                
                // 1. Normalize separation
                let separation_mag_sq = separation.x * separation.x + separation.y * separation.y;
                if separation_mag_sq > 0.0001 {
                    let separation_mag = separation_mag_sq.sqrt();
                    separation.x = separation.x / separation_mag * self.separation_weight;
                    separation.y = separation.y / separation_mag * self.separation_weight;
                }
                
                // 2. Calculate average velocity for alignment
                alignment.x = alignment.x / neighbor_count as f32 * self.alignment_weight;
                alignment.y = alignment.y / neighbor_count as f32 * self.alignment_weight;
                
                // 3. Calculate average position and steer toward it for cohesion
                cohesion_pos.x /= neighbor_count as f32;
                cohesion_pos.y /= neighbor_count as f32;
                
                // Vector pointing to center of mass
                let cohesion_force_x = cohesion_pos.x - pos.x;
                let cohesion_force_y = cohesion_pos.y - pos.y;
                
                // Normalize cohesion
                let cohesion_mag_sq = cohesion_force_x * cohesion_force_x + cohesion_force_y * cohesion_force_y;
                let mut cohesion = Velocity { x: 0.0, y: 0.0 };
                
                if cohesion_mag_sq > 0.0001 {
                    let cohesion_mag = cohesion_mag_sq.sqrt();
                    cohesion.x = cohesion_force_x / cohesion_mag * self.cohesion_weight;
                    cohesion.y = cohesion_force_y / cohesion_mag * self.cohesion_weight;
                }
                
                // Combine all forces to create new velocity
                let new_vel = Velocity {
                    x: vel.x + separation.x + alignment.x + cohesion.x,
                    y: vel.y + separation.y + alignment.y + cohesion.y,
                };
                
                // Calculate new position based on velocity
                let new_pos = Position {
                    x: pos.x + new_vel.x * dt,
                    y: pos.y + new_vel.y * dt,
                };
                
                // Store update for later application
                updates.push((**entity, new_pos, new_vel));
            }
        }
        
        // Apply all updates
        for (entity, mut new_pos, new_vel) in updates {
            // Apply boundary wrapping before setting the final position
            if new_pos.x > bounds.max_x { new_pos.x = bounds.min_x; }
            else if new_pos.x < bounds.min_x { new_pos.x = bounds.max_x; }
            if new_pos.y > bounds.max_y { new_pos.y = bounds.min_y; }
            else if new_pos.y < bounds.min_y { new_pos.y = bounds.max_y; }
            
            world.add_component(entity, new_pos);
            world.add_component(entity, new_vel);
        }
    }
}


#[cfg(test)]
mod tests {
    // Add tests later when core functionality is available
}
