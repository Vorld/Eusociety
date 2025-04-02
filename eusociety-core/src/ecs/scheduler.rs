use crate::ecs::system::{System, SystemAccess, AccessType};
use std::collections::{HashMap, HashSet, VecDeque}; // Added VecDeque

/// Registry for storing and managing systems
#[derive(Default)]
pub struct SystemRegistry {
    /// Systems stored in the registry
    systems: Vec<Box<dyn System>>,
    /// Access patterns for each system
    access_patterns: Vec<SystemAccess>,
}

impl SystemRegistry {
    /// Creates a new empty system registry
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Adds a system to the registry
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    /// 
    /// # Returns
    /// 
    /// True if the system was added successfully, false if it conflicts with existing systems
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> bool {
        let system_access = system.access();
        
        // Check for conflicts with existing systems
        for existing_access in &self.access_patterns {
            if system_access.conflicts_with(existing_access) {
                // Found a conflict, don't add the system
                return false;
            }
        }
        
        // No conflicts, add the system
        self.access_patterns.push(system_access);
        self.systems.push(Box::new(system));
        
        true
    }
    
    /// Forcefully adds a system regardless of conflicts
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    pub fn add_system_unchecked<S: System + 'static>(&mut self, system: S) {
        let system_access = system.access();
        self.access_patterns.push(system_access);
        self.systems.push(Box::new(system));
    }
    
    /// Runs all systems in the registry
    /// 
    /// # Arguments
    /// 
    /// * `world` - The world to run the systems on
    pub fn run_systems(&mut self, world: &mut crate::World) {
        for system in &mut self.systems {
            system.run(world);
        }
    }
    
    /// Returns the number of systems in the registry
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
    
    /// Checks if two systems have conflicting access patterns
    pub fn systems_conflict(system1: &dyn System, system2: &dyn System) -> bool {
        let access1 = system1.access();
        let access2 = system2.access();
        
        access1.conflicts_with(&access2)
    }
}

/// Represents dependencies between systems using a reverse adjacency list.
/// Key: System index `j`.
/// Value: A set of system indices `{i, k, ...}` that `j` depends on (must run *after*).
/// Example: `graph[j] = {i}` means there's a dependency `i -> j`.
pub type DependencyGraph = HashMap<usize, HashSet<usize>>;


/// Enhanced scheduler that uses the system registry to run systems
/// based on their data dependencies.
///
/// This scheduler builds a dependency graph from the `SystemAccess` patterns
/// declared by each system. It then calculates execution stages using Kahn's
/// algorithm (topological sort) to determine which systems can run concurrently.
///
/// Currently (M2.4), systems within a stage are still executed sequentially
/// due to the `System::run(&mut World)` signature requiring mutable access
/// to the entire `World`. True parallel system execution will require
/// significant changes to `World` and the `System` trait (planned for M2.5).
#[derive(Default)]
pub struct SystemScheduler {
    /// Registry storing all systems
    registry: SystemRegistry,
}

impl SystemScheduler {
    /// Creates a new empty scheduler
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Adds a system to the scheduler if it doesn't conflict with existing systems
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    /// 
    /// # Returns
    /// 
    /// True if the system was added, false if it conflicts
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> bool {
        self.registry.add_system(system)
    }
    
    /// Forcefully adds a system regardless of conflicts
    /// 
    /// # Arguments
    /// 
    /// * `system` - The system to add
    pub fn add_system_unchecked<S: System + 'static>(&mut self, system: S) {
        self.registry.add_system_unchecked(system)
    }
    
    /// Runs all registered systems according to their dependencies.
    ///
    /// This method performs the following steps:
    /// 1. Builds a dependency graph based on the `SystemAccess` of registered systems.
    /// 2. Calculates execution stages using topological sort (Kahn's algorithm).
    /// 3. Executes systems sequentially within each stage.
    ///
    /// # Arguments
    ///
    /// * `world` - The `World` containing components and resources for the systems.
    ///
    /// # Panics
    ///
    /// Panics if a cycle is detected in the system dependencies, as this indicates
    /// an impossible execution order.
    pub fn run(&mut self, world: &mut crate::World) {
        // 1. Build the dependency graph
        let graph = build_dependency_graph(&self.registry.access_patterns);

        // 2. Calculate execution stages
        let stages = match calculate_execution_stages(&graph) {
            Ok(s) => s,
            Err(e) => {
                // In a real application, you might want better error handling
                // For now, panic if a cycle is detected.
                panic!("Failed to calculate execution stages: {}", e);
            }
        };

        // 3. Execute systems stage by stage
        for stage in stages {
            for system_index in stage {
                // Get the mutable system reference and run it
                // Note: This borrow structure prevents parallel execution within a stage
                // because we need `&mut world` for each system run.
                if let Some(system) = self.registry.systems.get_mut(system_index) {
                    system.run(world);
                } else {
                    // This should ideally not happen if indices are correct
                    eprintln!("Warning: System index {} out of bounds during scheduling.", system_index);
                }
            }
        }
    }
    
    /// Returns the number of systems in the scheduler
    pub fn system_count(&self) -> usize {
        self.registry.system_count()
    }
}

/// Builds a dependency graph based on system access patterns.
/// Edges go from prerequisite system to dependent system (i -> j means i must run before j).
/// The returned graph represents reverse dependencies (j -> {i} means j depends on i).
fn build_dependency_graph(access_patterns: &[SystemAccess]) -> DependencyGraph {
    let num_systems = access_patterns.len();
    // Using reverse adjacency list: j -> {i} means j depends on i (must run after i)
    let mut rev_adj: HashMap<usize, HashSet<usize>> = HashMap::new();

    for i in 0..num_systems {
        rev_adj.entry(i).or_default(); // Ensure all nodes exist
    }

    for i in 0..num_systems {
        for j in (i + 1)..num_systems {
            let access_i = &access_patterns[i];
            let access_j = &access_patterns[j];

            let mut i_must_run_before_j = false; // Corresponds to edge i -> j
            let mut j_must_run_before_i = false; // Corresponds to edge j -> i

            // --- Check Component Conflicts ---
            // Iterate through Vec<DataAccess> for system i
            for access_i_comp in &access_i.component_access {
                // Iterate through Vec<DataAccess> for system j
                for access_j_comp in &access_j.component_access {
                    // Check if they refer to the same component
                    if access_i_comp.type_id == access_j_comp.type_id {
                        match (access_i_comp.access_type, access_j_comp.access_type) {
                            // Write-Read conflict: i writes, j reads -> i must run before j
                            (AccessType::Write, AccessType::Read) => i_must_run_before_j = true,
                            // Read-Write conflict: i reads, j writes -> j must run before i
                            (AccessType::Read, AccessType::Write) => j_must_run_before_i = true,
                            // Write-Write conflict: enforce registration order (i before j) for determinism
                            (AccessType::Write, AccessType::Write) => i_must_run_before_j = true,
                            // Read-Read is fine
                            (AccessType::Read, AccessType::Read) => {}
                        }
                    }
                }
            }

            // --- Check Resource Conflicts ---
             // Iterate through Vec<DataAccess> for system i
             for access_i_res in &access_i.resource_access {
                 // Iterate through Vec<DataAccess> for system j
                 for access_j_res in &access_j.resource_access {
                     // Check if they refer to the same resource
                     if access_i_res.type_id == access_j_res.type_id {
                        match (access_i_res.access_type, access_j_res.access_type) {
                            // Write-Read conflict: i writes, j reads -> i must run before j
                            (AccessType::Write, AccessType::Read) => i_must_run_before_j = true,
                            // Read-Write conflict: i reads, j writes -> j must run before i
                            (AccessType::Read, AccessType::Write) => j_must_run_before_i = true,
                            // Write-Write conflict: enforce registration order (i before j) for determinism
                            (AccessType::Write, AccessType::Write) => i_must_run_before_j = true,
                            // Read-Read is fine
                            (AccessType::Read, AccessType::Read) => {}
                        }
                    }
                }
            }
            // --- Add Edges to Reverse Adjacency List ---
            // If conflicting dependencies exist (e.g., i needs to run before j AND j needs to run before i),
            // we prioritize the registration order (i -> j) to break the potential cycle here.
            // This means j depends on i.
            if i_must_run_before_j && j_must_run_before_i {
                 // Mutual dependency detected, enforce registration order (i -> j)
                 // Add edge i -> j, meaning j depends on i
                 rev_adj.entry(j).or_default().insert(i);
            } else if i_must_run_before_j {
                 // Add edge i -> j, meaning j depends on i
                rev_adj.entry(j).or_default().insert(i);
            } else if j_must_run_before_i {
                 // Add edge j -> i, meaning i depends on j
                rev_adj.entry(i).or_default().insert(j);
            }
        }
    }

    rev_adj
}

/// Calculates execution stages based on a dependency graph using Kahn's algorithm.
/// The input graph represents reverse dependencies (j -> {i} means j depends on i).
/// Returns a list of stages, where each stage contains system indices that can run concurrently,
/// or an error string if a cycle is detected.
fn calculate_execution_stages(
    rev_dep_graph: &DependencyGraph,
) -> Result<Vec<Vec<usize>>, String> {
    let num_systems = rev_dep_graph.len();
    if num_systems == 0 {
        return Ok(Vec::new());
    }

    // 1. Calculate In-degrees and build Forward Adjacency List
    let mut in_degree: HashMap<usize, usize> = HashMap::with_capacity(num_systems);
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::with_capacity(num_systems); // Forward graph: i -> {j} means j depends on i

    for (&sys_idx, dependencies) in rev_dep_graph {
        in_degree.entry(sys_idx).or_insert(dependencies.len()); // Set initial in-degree based on reverse deps
        adj.entry(sys_idx).or_default(); // Ensure node exists in forward graph

        // Populate forward graph
        for &dep_idx in dependencies {
            adj.entry(dep_idx).or_default().push(sys_idx);
        }
    }

    // Ensure all nodes are in in_degree map, even those with 0 dependencies initially
    for i in 0..num_systems {
        in_degree.entry(i).or_insert(0);
        adj.entry(i).or_default(); // Ensure node exists in forward graph
    }


    // 2. Initialize Queue with nodes having in-degree 0
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (&node, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(node);
        }
    }

    // 3. Process nodes level by level (stages)
    let mut stages: Vec<Vec<usize>> = Vec::new();
    let mut processed_count = 0;

    while !queue.is_empty() {
        let mut current_stage: Vec<usize> = Vec::new();
        let stage_size = queue.len();

        for _ in 0..stage_size {
            let u = queue.pop_front().unwrap();
            current_stage.push(u);
            processed_count += 1;

            // For each neighbor v of u in the forward graph
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if let Some(degree) = in_degree.get_mut(&v) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }
        // Sort stage for deterministic output (optional but good for tests)
        current_stage.sort_unstable();
        stages.push(current_stage);
    }

    // 4. Check for Cycles
    if processed_count != num_systems {
        Err(format!(
            "Cycle detected in system dependencies. Processed {} out of {} systems.",
            processed_count, num_systems
        ))
    } else {
        Ok(stages)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    // Ensure AccessType is imported alongside other necessary items
    use crate::ecs::system::{System, SystemAccess, AccessType}; // Removed unused DataAccess
    // Add other necessary imports for tests
    use crate::{Position, World, Component, Resource};
    use std::any::TypeId;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex}; // Added for execution order test

    // --- Mock Components and Resources for Tests ---
    #[derive(Debug, Clone, Copy, Component)]
    struct Velocity { dx: f32, dy: f32 }

    #[derive(Debug, Clone, Copy)]
    struct ResourceA { value: i32 }
    impl Resource for ResourceA {}

    #[derive(Debug, Clone)] // Removed Copy derive
    struct ResourceB { value: String }
    impl Resource for ResourceB {}
    // --- End Mock Components and Resources ---


    // Mock system that reads Position
    struct PositionReaderSystem;
    
    impl System for PositionReaderSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new()
                .with_component(TypeId::of::<Position>(), AccessType::Read)
        }
        
        fn run(&mut self, world: &mut World) {
            // Just read positions
            for (_, pos) in world.components.query::<Position>() {
                let _ = pos.x;
            }
        }
    }
    
    // Mock system that writes Position
    struct PositionWriterSystem;
    
    impl System for PositionWriterSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new()
                .with_component(TypeId::of::<Position>(), AccessType::Write)
        }
        
        fn run(&mut self, world: &mut World) {
            // Modify positions
            for (_, pos) in world.components.query_mut::<Position>() {
                pos.x += 1.0;
            }
        }
    }

    // Mock system that reads Velocity
    struct VelocityReaderSystem;
    impl System for VelocityReaderSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new().with_component(TypeId::of::<Velocity>(), AccessType::Read)
        }
        fn run(&mut self, _world: &mut World) {}
    }

    // Mock system that writes Velocity
    struct VelocityWriterSystem;
    impl System for VelocityWriterSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new().with_component(TypeId::of::<Velocity>(), AccessType::Write)
        }
        fn run(&mut self, _world: &mut World) {}
    }

     // Mock system reading ResourceA and writing ResourceB
    struct ResourceTransformerSystem;
    impl System for ResourceTransformerSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new()
                .with_resource(TypeId::of::<ResourceA>(), AccessType::Read)
                .with_resource(TypeId::of::<ResourceB>(), AccessType::Write)
        }
        fn run(&mut self, _world: &mut World) {}
    }

    // Mock system writing ResourceA
    struct ResourceAWriterSystem;
    impl System for ResourceAWriterSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new().with_resource(TypeId::of::<ResourceA>(), AccessType::Write)
        }
        fn run(&mut self, _world: &mut World) {}
    }

     // Mock system reading ResourceB
    struct ResourceBReaderSystem;
    impl System for ResourceBReaderSystem {
        fn access(&self) -> SystemAccess {
            SystemAccess::new().with_resource(TypeId::of::<ResourceB>(), AccessType::Read)
        }
        fn run(&mut self, _world: &mut World) {}
    }


    #[test]
    fn test_system_conflict_detection() {
        let reader = PositionReaderSystem;
        let writer = PositionWriterSystem;
        let writer2 = PositionWriterSystem;
        
        // A reader and writer should conflict
        assert!(SystemRegistry::systems_conflict(&reader, &writer));
        
        // Two writers should conflict
        assert!(SystemRegistry::systems_conflict(&writer, &writer2));
        
        // Two readers should not conflict
        assert!(!SystemRegistry::systems_conflict(&reader, &reader));
    }
    
    #[test]
    fn test_registry_conflict_prevention() {
        let mut registry = SystemRegistry::new();
        
        // First system should be added successfully
        assert!(registry.add_system(PositionWriterSystem));
        assert_eq!(registry.system_count(), 1);
        
        // Second conflicting system should fail to be added
        assert!(!registry.add_system(PositionWriterSystem));
        assert_eq!(registry.system_count(), 1);
        
        // Non-conflicting system should be added
        // (in reality, this would be a different type of system)
    }
    
    #[test]
    fn test_scheduler() {
        let mut world = World::new();
        let entity = world.create_entity();
        world.add_component(entity, Position { x: 0.0, y: 0.0 });
        
        let mut scheduler = SystemScheduler::new();
        scheduler.add_system(PositionWriterSystem);
        
        scheduler.run(&mut world);
        
        // Position should be modified
        assert_eq!(world.get_component::<Position>(entity).unwrap().x, 1.0);
    }

    // --- Tests for build_dependency_graph ---

    // Helper to create expected graph for tests, ensuring all nodes up to max_index exist
    // Remember: graph is rev_adj, so j -> {i} means j depends on i
    fn expected_graph_with_nodes(max_index: usize, edges: &[(usize, usize)]) -> DependencyGraph {
        let mut graph: DependencyGraph = HashMap::new();
        for i in 0..=max_index {
            graph.insert(i, HashSet::new()); // Initialize all nodes
        }
        for &(u, v) in edges {
            // Edge u -> v means v depends on u
            graph.entry(v).or_default().insert(u);
        }
        graph
    }


    #[test]
    fn test_build_dependency_graph_simple_read_write() {
        // 0: Read Pos
        // 1: Write Pos
        let access_patterns = vec![
            PositionReaderSystem.access(), // 0
            PositionWriterSystem.access(), // 1
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: 0 depends on 1 (Read after Write) -> Edge 1 -> 0
        let expected = expected_graph_with_nodes(1, &[(1, 0)]);
        assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }

     #[test]
    fn test_build_dependency_graph_simple_write_read() {
        // 0: Write Pos
        // 1: Read Pos
        let access_patterns = vec![
            PositionWriterSystem.access(), // 0
            PositionReaderSystem.access(), // 1
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: 1 depends on 0 (Read after Write) -> Edge 0 -> 1
        let expected = expected_graph_with_nodes(1, &[(0, 1)]);
         assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }


    #[test]
    fn test_build_dependency_graph_write_write() {
        // 0: Write Pos
        // 1: Write Pos
        let access_patterns = vec![
            PositionWriterSystem.access(), // 0
            PositionWriterSystem.access(), // 1
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: 1 depends on 0 (Write after Write - registration order) -> Edge 0 -> 1
        let expected = expected_graph_with_nodes(1, &[(0, 1)]);
         assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }

     #[test]
    fn test_build_dependency_graph_read_read() {
        // 0: Read Pos
        // 1: Read Pos
        let access_patterns = vec![
            PositionReaderSystem.access(), // 0
            PositionReaderSystem.access(), // 1
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: No dependencies
        let expected = expected_graph_with_nodes(1, &[]);
         assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }

    #[test]
    fn test_build_dependency_graph_mixed() {
        // 0: Write Pos
        // 1: Read Pos, Write Vel
        // 2: Read Vel
        let access_patterns = vec![
            PositionWriterSystem.access(), // 0
            SystemAccess::new() // 1
                .with_component(TypeId::of::<Position>(), AccessType::Read)
                .with_component(TypeId::of::<Velocity>(), AccessType::Write),
            VelocityReaderSystem.access(), // 2
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected:
        // 1 depends on 0 (Read Pos after Write Pos) -> Edge 0 -> 1
        // 2 depends on 1 (Read Vel after Write Vel) -> Edge 1 -> 2
        let expected = expected_graph_with_nodes(2, &[(0, 1), (1, 2)]);
        assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph[&2], expected[&2]);
        assert_eq!(graph.len(), expected.len());
    }

     #[test]
    fn test_build_dependency_graph_independent() {
        // 0: Write Pos
        // 1: Write Vel
        let access_patterns = vec![
            PositionWriterSystem.access(), // 0
            VelocityWriterSystem.access(), // 1
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: No dependencies
        let expected = expected_graph_with_nodes(1, &[]);
        assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }

     #[test]
    fn test_build_dependency_graph_resources() {
        // 0: Write ResourceA
        // 1: Read ResourceA, Write ResourceB
        // 2: Read ResourceB
         let access_patterns = vec![
            ResourceAWriterSystem.access(), // 0
            ResourceTransformerSystem.access(), // 1
            ResourceBReaderSystem.access(), // 2
        ];
        let graph = build_dependency_graph(&access_patterns);
        // Expected:
        // 1 depends on 0 (Read A after Write A) -> Edge 0 -> 1
        // 2 depends on 1 (Read B after Write B) -> Edge 1 -> 2
        let expected = expected_graph_with_nodes(2, &[(0, 1), (1, 2)]);
        assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph[&2], expected[&2]);
        assert_eq!(graph.len(), expected.len());
    }

     #[test]
    fn test_build_dependency_graph_mutual_dependency_resolution() {
        // System 0: Writes CompA (Pos), Reads CompB (Vel)
        // System 1: Reads CompA (Pos), Writes CompB (Vel)
        // This creates a potential cycle if not handled.
        // Conflict 1: Sys1 reads Pos, Sys0 writes Pos => 1 depends on 0 (0 -> 1)
        // Conflict 2: Sys0 reads Vel, Sys1 writes Vel => 0 depends on 1 (1 -> 0)
        // We expect registration order (0 -> 1) to break the tie.
        let access_patterns = vec![
            SystemAccess::new() // 0
                .with_component(TypeId::of::<Position>(), AccessType::Write) // CompA = Position
                .with_component(TypeId::of::<Velocity>(), AccessType::Read), // CompB = Velocity
            SystemAccess::new() // 1
                .with_component(TypeId::of::<Position>(), AccessType::Read)  // CompA = Position
                .with_component(TypeId::of::<Velocity>(), AccessType::Write), // CompB = Velocity
        ];
        let graph = build_dependency_graph(&access_patterns);

        // Expected: 1 depends on 0 (due to registration order breaking the tie) -> Edge 0 -> 1
        let expected = expected_graph_with_nodes(1, &[(0, 1)]);
        assert_eq!(graph[&0], expected[&0]);
        assert_eq!(graph[&1], expected[&1]);
        assert_eq!(graph.len(), expected.len());
    }

    // --- Tests for calculate_execution_stages ---

    #[test]
    fn test_calculate_stages_empty() {
        let graph = DependencyGraph::new();
        let stages = calculate_execution_stages(&graph).unwrap();
        assert!(stages.is_empty());
    }

    #[test]
    fn test_calculate_stages_no_dependencies() {
        // 0, 1, 2 run independently
        let graph = expected_graph_with_nodes(2, &[]);
        let stages = calculate_execution_stages(&graph).unwrap();
        // Expect one stage with all systems
        assert_eq!(stages, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn test_calculate_stages_linear_dependency() {
        // 0 -> 1 -> 2
        // Graph: 1 depends on 0, 2 depends on 1
        let graph = expected_graph_with_nodes(2, &[(0, 1), (1, 2)]);
        let stages = calculate_execution_stages(&graph).unwrap();
        // Expect three stages: [0], [1], [2]
        assert_eq!(stages, vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn test_calculate_stages_parallel_branches() {
        //    -> 1 -> 3
        // 0 -
        //    -> 2 -> 4
        // Graph: 1 dep 0, 2 dep 0, 3 dep 1, 4 dep 2
        let graph = expected_graph_with_nodes(4, &[(0, 1), (0, 2), (1, 3), (2, 4)]);
        let stages = calculate_execution_stages(&graph).unwrap();
        // Expect stages: [0], [1, 2], [3, 4] (sorted within stage)
        assert_eq!(stages, vec![vec![0], vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn test_calculate_stages_complex_graph() {
        // 0 -> 1 \
        //        -> 3 -> 4
        //      2 /
        // Graph: 1 dep 0, 3 dep 1, 3 dep 2, 4 dep 3
        let graph = expected_graph_with_nodes(4, &[(0, 1), (1, 3), (2, 3), (3, 4)]);
        let stages = calculate_execution_stages(&graph).unwrap();
        // Expect stages: [0, 2], [1], [3], [4] (sorted within stage)
        assert_eq!(stages, vec![vec![0, 2], vec![1], vec![3], vec![4]]);
    }

     #[test]
    fn test_calculate_stages_multiple_independent_starts() {
        // 0 -> 2
        // 1 -> 3
        // Graph: 2 dep 0, 3 dep 1
        let graph = expected_graph_with_nodes(3, &[(0, 2), (1, 3)]);
        let stages = calculate_execution_stages(&graph).unwrap();
        // Expect stages: [0, 1], [2, 3] (sorted within stage)
        assert_eq!(stages, vec![vec![0, 1], vec![2, 3]]);
    }

    #[test]
    fn test_calculate_stages_cycle_detection() {
        // 0 -> 1 -> 2 -> 0 (cycle)
        // Graph: 1 dep 0, 2 dep 1, 0 dep 2
        let graph = expected_graph_with_nodes(2, &[(0, 1), (1, 2), (2, 0)]);
        let result = calculate_execution_stages(&graph);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle detected"));
    }

     #[test]
    fn test_calculate_stages_cycle_with_other_nodes() {
         // 0 -> 1 -> 2 -> 1 (cycle 1-2)
         // 3 -> 4
         // Graph: 1 dep 0, 2 dep 1, 1 dep 2, 4 dep 3
        let graph = expected_graph_with_nodes(4, &[(0, 1), (1, 2), (2, 1), (3, 4)]); // Removed mut
        let result = calculate_execution_stages(&graph);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle detected"));
    }

    // --- Test for Scheduler Execution Order ---

    #[test]
    fn test_scheduler_execution_order() {
        // Shared state to record execution order
        let execution_log = Arc::new(Mutex::new(Vec::<String>::new()));

        // Define systems with dependencies:
        // SysA (writes ResA) -> SysB (reads ResA, writes ResB) -> SysC (reads ResB)
        struct SysA { log: Arc<Mutex<Vec<String>>> }
        impl System for SysA {
            fn access(&self) -> SystemAccess {
                SystemAccess::new().with_resource(TypeId::of::<ResourceA>(), AccessType::Write)
            }
            fn run(&mut self, world: &mut World) {
                self.log.lock().unwrap().push("A".to_string());
                // Simulate writing to ResourceA
                if let Some(res) = world.get_resource_mut::<ResourceA>() {
                    res.value += 1;
                } else {
                     world.insert_resource(ResourceA { value: 1 });
                }
            }
        }

        struct SysB { log: Arc<Mutex<Vec<String>>> }
        impl System for SysB {
            fn access(&self) -> SystemAccess {
                SystemAccess::new()
                    .with_resource(TypeId::of::<ResourceA>(), AccessType::Read)
                    .with_resource(TypeId::of::<ResourceB>(), AccessType::Write)
            }
            fn run(&mut self, world: &mut World) {
                self.log.lock().unwrap().push("B".to_string());
                // Simulate reading ResourceA and writing ResourceB
                let _a_val = world.get_resource::<ResourceA>().map(|r| r.value).unwrap_or(0);
                 if let Some(res) = world.get_resource_mut::<ResourceB>() {
                    res.value = format!("B wrote {}", _a_val);
                } else {
                     world.insert_resource(ResourceB { value: format!("B wrote {}", _a_val) });
                }
            }
        }

        struct SysC { log: Arc<Mutex<Vec<String>>> }
        impl System for SysC {
            fn access(&self) -> SystemAccess {
                SystemAccess::new().with_resource(TypeId::of::<ResourceB>(), AccessType::Read)
            }
            fn run(&mut self, world: &mut World) {
                self.log.lock().unwrap().push("C".to_string());
                // Simulate reading ResourceB
                 let _b_val = world.get_resource::<ResourceB>().map(|r| r.value.clone()).unwrap_or_default();
            }
        }

        // Setup world and scheduler
        let mut world = World::new();
        let mut scheduler = SystemScheduler::new();

        // Add systems (order shouldn't matter for execution if dependencies are correct)
        scheduler.add_system_unchecked(SysC { log: execution_log.clone() }); // Index 0
        scheduler.add_system_unchecked(SysA { log: execution_log.clone() }); // Index 1
        scheduler.add_system_unchecked(SysB { log: execution_log.clone() }); // Index 2

        // Run the scheduler
        scheduler.run(&mut world);

        // Assert the execution order based on dependencies
        // Expected stages: [SysA], [SysB], [SysC] -> Order A, B, C
        // Note: Indices in graph/stages refer to registration order (C=0, A=1, B=2)
        // Graph: B depends on A (2 dep 1), C depends on B (0 dep 2)
        // Stages: [1], [2], [0] -> Execution order A, B, C
        let log = execution_log.lock().unwrap();
        assert_eq!(*log, vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    }
}
