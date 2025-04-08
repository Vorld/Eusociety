//! Contains the implementation for a custom Quadtree spatial partitioning structure.

use bevy_ecs::prelude::*;
use tracing::info; // For logging during build

// Note: Removed VecDeque as Vec is sufficient for now. Can reconsider if needed.
// use std::collections::VecDeque;

use crate::simulation::components::{Position, FoodSource}; // Added FoodSource
// Removed unused import: SimulationConfigResource
// use crate::simulation::resources::SimulationConfigResource;

// --- Configuration ---

/// Maximum number of points a leaf node can hold before subdividing.
const QUADTREE_CAPACITY: usize = 4;
/// Maximum depth of the tree to prevent infinite recursion.
const QUADTREE_MAX_DEPTH: usize = 8;

// --- Data Structures ---

/// Represents an axis-aligned bounding box (AABB).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl Rect {
    /// Creates a new Rect.
    pub fn new(x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> Self {
        // Ensure min <= max
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);
        Self { x_min, y_min, x_max, y_max }
    }

    /// Returns the center x coordinate.
    #[inline]
    pub fn center_x(&self) -> f32 {
        (self.x_min + self.x_max) / 2.0
    }

    /// Returns the center y coordinate.
    #[inline]
    pub fn center_y(&self) -> f32 {
        (self.y_min + self.y_max) / 2.0
    }

    /// Checks if this Rect contains a given Position.
    /// Note: Uses inclusive min and inclusive max now for consistency with boundary handling.
    #[inline]
    pub fn contains(&self, point: &Position) -> bool {
        point.x >= self.x_min
            && point.x <= self.x_max // Changed to <=
            && point.y >= self.y_min
            && point.y <= self.y_max // Changed to <=
    }

    /// Checks if this Rect intersects with another Rect.
    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        // Check for no overlap (easier)
        !(other.x_min >= self.x_max
            || other.x_max <= self.x_min
            || other.y_min >= self.y_max
            || other.y_max <= self.y_min)
    }

    /// Subdivides this Rect into four equal quadrants.
    pub fn subdivide(&self) -> [Rect; 4] {
        let center_x = self.center_x();
        let center_y = self.center_y();
        [
            // North-West (NW) [0]
            Rect::new(self.x_min, center_y, center_x, self.y_max),
            // North-East (NE) [1]
            Rect::new(center_x, center_y, self.x_max, self.y_max),
            // South-West (SW) [2]
            Rect::new(self.x_min, self.y_min, center_x, center_y),
            // South-East (SE) [3]
            Rect::new(center_x, self.y_min, self.x_max, center_y),
        ]
    }
}

/// Represents a node in the Quadtree.
#[derive(Debug)]
pub enum QuadTreeNode {
    Leaf {
        boundary: Rect,
        // Store Entity and Position together for easier access/removal
        points: Vec<(Entity, Position)>,
    },
    Internal {
        boundary: Rect,
        // Order: NW[0], NE[1], SW[2], SE[3]
        children: [Box<QuadTreeNode>; 4],
    },
}

impl QuadTreeNode {
    /// Creates a new leaf node.
    fn new_leaf(boundary: Rect) -> Self {
        QuadTreeNode::Leaf {
            boundary,
            points: Vec::with_capacity(QUADTREE_CAPACITY), // Pre-allocate slightly
        }
    }

    /// Returns the boundary of this node.
    fn boundary(&self) -> Rect {
        match self {
            QuadTreeNode::Leaf { boundary, .. } => *boundary,
            QuadTreeNode::Internal { boundary, .. } => *boundary,
        }
    }

    /// Attempts to insert a point into this node or its children.
    /// Returns true if insertion was successful, false otherwise (e.g., point outside boundary).
    fn insert(&mut self, entity: Entity, position: Position, current_depth: usize) -> bool {
        // Check if the point is within the node's boundary
        // Use >= min and <= max for contains check (consistent with Rect::contains)
        if !(position.x >= self.boundary().x_min && position.x <= self.boundary().x_max && // Changed to <=
             position.y >= self.boundary().y_min && position.y <= self.boundary().y_max) { // Changed to <=
             // Optional: Add a specific warning if it's *exactly* on the boundary but outside the root?
             // This case should be rare if the root boundary is correct.
            return false; // Point is outside this node's area
        }

        match self {
            QuadTreeNode::Leaf { boundary, points, .. } => {
                // If it's a leaf node:
                if points.len() < QUADTREE_CAPACITY || current_depth >= QUADTREE_MAX_DEPTH {
                    // If capacity not reached or max depth hit, add the point here
                    points.push((entity, position));
                    true
                } else {
                    // If capacity reached and depth allows, subdivide and then insert
                    // Need to temporarily take ownership of points to redistribute
                    let current_points = std::mem::take(points);
                    let children_boundaries = boundary.subdivide();
                    // Remove 'mut' as children array is moved immediately
                    let children = [
                        Box::new(QuadTreeNode::new_leaf(children_boundaries[0])), // NW
                        Box::new(QuadTreeNode::new_leaf(children_boundaries[1])), // NE
                        Box::new(QuadTreeNode::new_leaf(children_boundaries[2])), // SW
                        Box::new(QuadTreeNode::new_leaf(children_boundaries[3])), // SE
                    ];

                    // Transition self to an Internal node BEFORE redistributing points
                    *self = QuadTreeNode::Internal {
                        boundary: *boundary,
                        children,
                    };

                    // Redistribute existing points into the new children
                    // We know 'self' is now Internal, so call insert on the new self
                    for (e, p) in current_points {
                        // It's safe to unwrap here because we just made it Internal
                        if let QuadTreeNode::Internal { children, .. } = self {
                             Self::insert_into_children(children, e, p, current_depth + 1);
                        }
                    }

                    // Finally, insert the new point into the appropriate child
                    // It's safe to unwrap here because we just made it Internal
                     if let QuadTreeNode::Internal { children, .. } = self {
                         Self::insert_into_children(children, entity, position, current_depth + 1)
                     } else {
                         unreachable!("Node should be Internal after subdivision");
                     }
                }
            }
            QuadTreeNode::Internal { children, .. } => {
                // If it's an internal node, determine which child to insert into
                Self::insert_into_children(children, entity, position, current_depth + 1)
            }
        }
    }

    /// Helper function to insert into the correct child of an Internal node.
    fn insert_into_children(children: &mut [Box<QuadTreeNode>; 4], entity: Entity, position: Position, current_depth: usize) -> bool {
        // Determine which child quadrant the point belongs to based on the parent's center
        // Assumes children order: NW[0], NE[1], SW[2], SE[3]
        // We need the parent's center, which is the corner point for the children.
        // Child 0 (NW) boundary gives us the center point.
        let center_x = children[0].boundary().x_max;
        let center_y = children[0].boundary().y_min; // NW's min_y is the center_y

        let child_index = if position.y >= center_y { // North
            if position.x < center_x { 0 } else { 1 } // NW or NE
        } else { // South
            if position.x < center_x { 2 } else { 3 } // SW or SE
        };

        children[child_index].insert(entity, position, current_depth) // Pass depth along
    }

    /// Recursively queries the node and its children for points within the given range.
    fn query_range<'a>(&'a self, range: &Rect, found: &mut Vec<&'a (Entity, Position)>) {
        // If the query range doesn't intersect this node's boundary, prune this branch
        if !self.boundary().intersects(range) {
            return;
        }

        match self {
            QuadTreeNode::Leaf { points, .. } => {
                // If it's a leaf, check each point within this node
                for point_data @ (_, point_pos) in points.iter() {
                    // Check if the point's position is within the query range Rect
                    if range.contains(point_pos) {
                        found.push(point_data);
                    }
                }
            }
            QuadTreeNode::Internal { children, .. } => {
                // If it's internal, recursively query children
                for child in children.iter() {
                    child.query_range(range, found);
                }
            }
        }
    }

    /// Attempts to remove a specific entity at a given position from this node or its children.
    /// Returns true if the entity was found and removed, false otherwise.
    /// Note: Does not currently implement node merging after removal.
    fn remove(&mut self, entity_to_remove: Entity, position: &Position) -> bool {
         // If the point is outside this node's boundary, it cannot be here
        if !self.boundary().contains(position) {
            return false;
        }

        match self {
            QuadTreeNode::Leaf { points, .. } => {
                // If it's a leaf, find and remove the point
                let initial_len = points.len();
                // Remove the point if the entity matches
                points.retain(|(entity, _)| *entity != entity_to_remove);
                // Return true if an element was removed
                points.len() < initial_len
            }
            QuadTreeNode::Internal { children, .. } => {
                 // If it's internal, determine which child the point *should* be in
                let center_x = children[0].boundary().x_max;
                let center_y = children[0].boundary().y_min;

                let child_index = if position.y >= center_y { // North
                    if position.x < center_x { 0 } else { 1 } // NW or NE
                } else { // South
                    if position.x < center_x { 2 } else { 3 } // SW or SE
                };

                // Recursively call remove on the appropriate child
                children[child_index].remove(entity_to_remove, position)
            }
        }
    }
}


/// The Bevy resource holding the Quadtree root and configuration.
#[derive(Resource, Debug)]
pub struct FoodQuadtree {
    root: QuadTreeNode,
    // Store max depth and capacity for reference if needed
    // max_depth: usize,
    // capacity: usize,
}

impl FoodQuadtree {
    /// Creates a new, empty FoodQuadtree for the given world boundary.
    pub fn new(world_boundary: Rect) -> Self {
        info!("Creating new FoodQuadtree with boundary: {:?}", world_boundary);
        Self {
            root: QuadTreeNode::new_leaf(world_boundary),
            // max_depth: QUADTREE_MAX_DEPTH,
            // capacity: QUADTREE_CAPACITY,
        }
    }

    /// Inserts an entity with its position into the Quadtree.
    pub fn insert(&mut self, entity: Entity, position: Position) {
        // Start insertion from the root node at depth 0
        if !self.root.insert(entity, position, 0) {
            // Optional: Log or handle cases where the point is outside the root boundary
            tracing::warn!(?entity, ?position, "Attempted to insert point outside Quadtree root boundary");
        }
    }

    /// Queries the Quadtree for all points within the given rectangular range.
    /// Returns a Vec containing references to the (Entity, Position) tuples found.
    pub fn query_range<'a>(&'a self, range: &Rect) -> Vec<&'a (Entity, Position)> {
        let mut found = Vec::new();
        self.root.query_range(range, &mut found);
        found
    }

    /// Removes a specific entity at a given position from the Quadtree.
    /// Returns true if the entity was found and removed, false otherwise.
    pub fn remove(&mut self, entity: Entity, position: &Position) -> bool {
        self.root.remove(entity, position)
    }

    /// Clears all points from the Quadtree, resetting it to an empty leaf node.
    /// Useful for rebuilding the tree if needed.
    pub fn clear(&mut self) {
        info!("Clearing FoodQuadtree");
        self.root = QuadTreeNode::new_leaf(self.root.boundary());
    }
}


// --- Pheromone Quadtree ---

/// The Bevy resource holding the Pheromone Quadtree root.
#[derive(Resource, Debug)]
pub struct PheromoneQuadtree {
    root: QuadTreeNode,
}

impl PheromoneQuadtree {
    /// Creates a new, empty PheromoneQuadtree for the given world boundary.
    pub fn new(world_boundary: Rect) -> Self {
        info!("Creating new PheromoneQuadtree with boundary: {:?}", world_boundary);
        Self {
            root: QuadTreeNode::new_leaf(world_boundary),
        }
    }

    /// Inserts an entity with its position into the Quadtree.
    pub fn insert(&mut self, entity: Entity, position: Position) {
        if !self.root.insert(entity, position, 0) {
            tracing::warn!(?entity, ?position, "Attempted to insert pheromone outside Quadtree root boundary");
        }
    }

    /// Queries the Quadtree for all points within the given rectangular range.
    /// Returns a Vec containing references to the (Entity, Position) tuples found.
    pub fn query_range<'a>(&'a self, range: &Rect) -> Vec<&'a (Entity, Position)> {
        let mut found = Vec::new();
        self.root.query_range(range, &mut found);
        found
    }

    /// Removes a specific entity at a given position from the Quadtree.
    /// Returns true if the entity was found and removed, false otherwise.
    pub fn remove(&mut self, entity: Entity, position: &Position) -> bool {
        self.root.remove(entity, position)
    }

    /// Clears all points from the Quadtree, resetting it to an empty leaf node.
    pub fn clear(&mut self) {
        info!("Clearing PheromoneQuadtree");
        self.root = QuadTreeNode::new_leaf(self.root.boundary());
    }
}


// --- Systems ---

/// System that runs once at startup to build the initial FoodQuadtree.
pub fn build_food_quadtree_system(
    mut quadtree: ResMut<FoodQuadtree>,
    food_query: Query<(Entity, &Position), With<FoodSource>>,
) {
    info!("Building FoodQuadtree...");
    quadtree.clear(); // Clear any previous state (though it should be new)
    let mut count = 0;
    for (entity, position) in food_query.iter() {
        quadtree.insert(entity, *position); // Dereference position
        count += 1;
    }
    info!("Inserted {} food items into FoodQuadtree.", count);
}


// TODO: Add unit tests for Rect and Quadtree logic (insert, query_range, remove)