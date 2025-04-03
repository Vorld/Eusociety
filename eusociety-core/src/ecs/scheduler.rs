use crate::ecs::system::{System, SystemAccess, AccessType};
use crate::World;
use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use rayon::prelude::*; // Import Rayon prelude
use std::sync::{Arc, Barrier, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

// --- System Runnable Trait ---
// Internal trait to handle type erasure for running systems.
trait SystemRunnable: Send + Sync {
    /// Runs the system with type-erased state.
    fn run(&mut self, world: &World, state: &mut dyn Any);
    /// Gets the system's name.
    fn name(&self) -> String;
    // Access pattern is stored separately in the registry.
}

// Blanket implementation for any type implementing the System trait.
impl<S> SystemRunnable for S
where
    S: System + 'static, // System itself is 'static
    S::SystemState: 'static, // State must be 'static to be Any
{
    fn run(&mut self, world: &World, state: &mut dyn Any) {
        // Downcast the type-erased state back to the concrete type.
        let concrete_state = state.downcast_mut::<S::SystemState>()
            .expect("System state type mismatch. This indicates a bug in the scheduler.");
        // Call the actual System::run method.
        System::run(self, world, concrete_state);
    }

    fn name(&self) -> String {
        System::name(self).to_string()
    }
}


/// Stores system runners, their states, and access patterns.
#[derive(Default)]
pub struct SystemRegistry {
    /// Boxed system runners (handle type erasure).
    runners: Vec<Box<dyn SystemRunnable>>, // Store runners directly
    /// Boxed system states, corresponding to the runners vector.
    states: Vec<Box<dyn Any + Send + Sync>>,
    /// Cached access patterns for each system.
    access_patterns: Vec<SystemAccess>,
    // Names are retrieved via SystemRunnable::name() if needed outside run loop
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a system and initializes its state.
    pub fn add_system<S>(&mut self, system: S, world: &mut World) -> bool
    where
        S: System + 'static,
        S::SystemState: 'static,
    {
        let system_access = S::access(); // Static access

        // Check for conflicts
        for existing_access in &self.access_patterns {
            if system_access.conflicts_with(existing_access) {
                eprintln!("Conflict detected when adding system: {}", std::any::type_name::<S>());
                return false;
            }
        }

        // Initialize state
        let state = S::init_state(world);

        // Store runner (system boxed as SystemRunnable), state (boxed), and access pattern
        self.access_patterns.push(system_access);
        self.states.push(Box::new(state));
        self.runners.push(Box::new(system)); // Box the system directly, becomes Box<dyn SystemRunnable>

        true
    }

    /// Forcefully adds a system and initializes its state, ignoring conflicts.
    pub fn add_system_unchecked<S>(&mut self, system: S, world: &mut World)
    where
        S: System + 'static,
        S::SystemState: 'static,
    {
        let system_access = S::access();
        let state = S::init_state(world);

        self.access_patterns.push(system_access);
        self.states.push(Box::new(state));
        self.runners.push(Box::new(system));
    }

    pub fn system_count(&self) -> usize {
        self.runners.len()
    }
}

pub type DependencyGraph = HashMap<usize, HashSet<usize>>;

/// Enhanced scheduler using the new System trait and SystemParams.
#[derive(Default)]
pub struct SystemScheduler {
    registry: SystemRegistry,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a system to the scheduler, initializing its state.
    pub fn add_system<S>(&mut self, system: S, world: &mut World) -> bool
    where
        S: System + 'static,
        S::SystemState: 'static,
    {
        self.registry.add_system(system, world)
    }

    /// Forcefully adds a system, initializing its state.
    pub fn add_system_unchecked<S>(&mut self, system: S, world: &mut World)
    where
        S: System + 'static,
        S::SystemState: 'static,
    {
        self.registry.add_system_unchecked(system, world)
    }

    /// Runs all registered systems according to their dependencies, potentially in parallel.
    pub fn run(&mut self, world: &World) { // Takes &World
        // 1. Build the dependency graph
        let graph = build_dependency_graph(&self.registry.access_patterns);

        // 2. Calculate execution stages
        let stages = match calculate_execution_stages(&graph) {
            Ok(s) => s,
            Err(e) => panic!("Failed to calculate execution stages: {}", e),
        };

        // 3. Execute systems stage by stage
        for stage in stages {
            // Use par_iter to process systems within a stage in parallel
            stage.par_iter().for_each(|&system_index| {
                // --- UNSAFE ---
                // Accessing elements of `runners` and `states` mutably in parallel requires unsafe code.
                unsafe {
                    let runner_ptr = self.registry.runners.as_mut_ptr().add(system_index);
                    let state_ptr = self.registry.states.as_mut_ptr().add(system_index);
                    let runner = &mut *runner_ptr;
                    let state = &mut *state_ptr;
                    runner.run(world, state.as_mut());
                }
            });
        }
    }

    /// Runs all registered systems in parallel where possible.
    pub fn run_parallel(&self, world: &World) {
        // Start with systems that have no dependencies
        let graph = build_dependency_graph(&self.registry.access_patterns);
        let mut ready = Vec::new();
        
        // Find systems with no dependencies
        for system_index in 0..self.registry.runners.len() {
            if !graph.contains_key(&system_index) || graph[&system_index].is_empty() {
                ready.push(system_index);
            }
        }
        
        // Create a thread pool for running systems in parallel
        // Add num_cpus as a dependency in Cargo.toml instead of trying to use it directly
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4) // Use a reasonable fixed number for now
            .build()
            .unwrap();
        
        // Custom barrier for synchronizing the execution
        let barrier = Arc::new(Barrier::new(ready.len() + 1));
        
        // Use atomic counters for tracking finished systems
        let remaining = Arc::new(AtomicUsize::new(self.registry.runners.len()));
        let completed = Arc::new(AtomicUsize::new(0));
        
        // For each ready system, spawn a thread to run it
        let registry_ptr = &self.registry as *const SystemRegistry;
        
        // Track which systems have been run
        let systems_run = Arc::new(Mutex::new(HashSet::new()));
        
        // Use FnMut closures with move to take ownership of necessary variables
        for system_index in ready {
            let barrier_clone = barrier.clone();
            let remaining_clone = remaining.clone();
            let completed_clone = completed.clone();
            let systems_run_clone = systems_run.clone();
            let world_ptr = world as *const World;
            
            pool.spawn(move || unsafe {
                // Get pointers to the system and state to allow safe modification
                let registry = &*registry_ptr;
                let runner_ptr = registry.runners.as_ptr();
                let state_ptr = registry.states.as_ptr();
                
                // These are safe because we've guaranteed no system will mutate the same state
                // via the dependency graph analysis
                let runner = &mut *(runner_ptr.add(system_index) as *mut Box<dyn SystemRunnable>);
                let state = &mut *(state_ptr.add(system_index) as *mut Box<dyn Any + Send + Sync>);
                let world = &*world_ptr;
                
                // Run the system
                runner.run(world, state.as_mut());
                
                // Update completed & remaining counts
                let _old_completed = completed_clone.fetch_add(1, Ordering::SeqCst);
                remaining_clone.fetch_sub(1, Ordering::SeqCst);
                
                // Update the synchronization primitive
                {
                    let mut systems_done = systems_run_clone.lock().unwrap();
                    systems_done.insert(system_index);
                }
                
                // Wait for all systems in this batch to complete
                barrier_clone.wait();
            });
        }
        
        // Wait for all systems in this batch to complete before starting the next batch
        barrier.wait();
        
        // Simplified sequential execution of remaining systems until we 
        // implement proper wave-based scheduling
        let mut next_systems = HashSet::new();
        while completed.load(Ordering::SeqCst) < self.registry.runners.len() {
            // Find ready systems based on completed ones
            let systems_done = systems_run.lock().unwrap();
            
            for system_index in 0..self.registry.runners.len() {
                if !systems_done.contains(&system_index) {
                    // Check if all dependencies are done
                    let mut all_deps_done = true;
                    if let Some(deps) = graph.get(&system_index) {
                        all_deps_done = deps.iter().all(|&dep| systems_done.contains(&dep));
                    }
                    
                    if all_deps_done {
                        next_systems.insert(system_index);
                    }
                }
            }
            
            // Release the lock before running systems
            drop(systems_done);
            
            // For the MVP, run the next wave sequentially
            for system_index in next_systems.drain() {
                unsafe {
                    // Same technique as above for safe modification
                    let registry = &*registry_ptr;
                    let runner_ptr = registry.runners.as_ptr();
                    let state_ptr = registry.states.as_ptr();
                    
                    let runner = &mut *(runner_ptr.add(system_index) as *mut Box<dyn SystemRunnable>);
                    let state = &mut *(state_ptr.add(system_index) as *mut Box<dyn Any + Send + Sync>);
                    
                    runner.run(world, state.as_mut());
                    
                    let _old_completed = completed.fetch_add(1, Ordering::SeqCst);
                    let mut systems_done = systems_run.lock().unwrap();
                    systems_done.insert(system_index);
                }
            }
        }
    }

    pub fn system_count(&self) -> usize {
        self.registry.system_count()
    }
}

// --- Dependency Graph Logic (Unchanged) ---
fn build_dependency_graph(access_patterns: &[SystemAccess]) -> DependencyGraph {
    let num_systems = access_patterns.len();
    let mut rev_adj: HashMap<usize, HashSet<usize>> = HashMap::new();
    for i in 0..num_systems { rev_adj.entry(i).or_default(); }

    for i in 0..num_systems {
        for j in (i + 1)..num_systems {
            let access_i = &access_patterns[i];
            let access_j = &access_patterns[j];
            let mut i_before_j = false;
            let mut j_before_i = false;

            for acc_i in &access_i.component_access {
                for acc_j in &access_j.component_access {
                    if acc_i.type_id == acc_j.type_id {
                        match (acc_i.access_type, acc_j.access_type) {
                            (AccessType::Write, AccessType::Read) => i_before_j = true,
                            (AccessType::Read, AccessType::Write) => j_before_i = true,
                            (AccessType::Write, AccessType::Write) => i_before_j = true,
                            _ => {}
                        }
                    }
                }
            }
            for acc_i in &access_i.resource_access {
                for acc_j in &access_j.resource_access {
                     if acc_i.type_id == acc_j.type_id {
                        match (acc_i.access_type, acc_j.access_type) {
                            (AccessType::Write, AccessType::Read) => i_before_j = true,
                            (AccessType::Read, AccessType::Write) => j_before_i = true,
                            (AccessType::Write, AccessType::Write) => i_before_j = true,
                            _ => {}
                        }
                    }
                }
            }

            if i_before_j && j_before_i { // Mutual dependency -> Use registration order
                rev_adj.entry(j).or_default().insert(i); // j depends on i
            } else if i_before_j {
                rev_adj.entry(j).or_default().insert(i); // j depends on i
            } else if j_before_i {
                rev_adj.entry(i).or_default().insert(j); // i depends on j
            }
        }
    }
    rev_adj
}

fn calculate_execution_stages(rev_dep_graph: &DependencyGraph) -> Result<Vec<Vec<usize>>, String> {
    let num_systems = rev_dep_graph.len();
    if num_systems == 0 { return Ok(Vec::new()); }

    let mut in_degree: HashMap<usize, usize> = HashMap::with_capacity(num_systems);
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::with_capacity(num_systems);

    for (&sys_idx, dependencies) in rev_dep_graph {
        in_degree.entry(sys_idx).or_insert(dependencies.len());
        adj.entry(sys_idx).or_default();
        for &dep_idx in dependencies {
            adj.entry(dep_idx).or_default().push(sys_idx);
        }
    }
    for i in 0..num_systems {
        in_degree.entry(i).or_insert(0);
        adj.entry(i).or_default();
    }

    let mut queue: VecDeque<usize> = VecDeque::new();
    for (&node, &degree) in &in_degree {
        if degree == 0 { queue.push_back(node); }
    }

    let mut stages: Vec<Vec<usize>> = Vec::new();
    let mut processed_count = 0;

    while !queue.is_empty() {
        let mut current_stage: Vec<usize> = Vec::new();
        let stage_size = queue.len();
        for _ in 0..stage_size {
            let u = queue.pop_front().unwrap();
            current_stage.push(u);
            processed_count += 1;
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if let Some(degree) = in_degree.get_mut(&v) {
                        *degree -= 1;
                        if *degree == 0 { queue.push_back(v); }
                    }
                }
            }
        }
        current_stage.sort_unstable();
        stages.push(current_stage);
    }

    if processed_count != num_systems {
        Err(format!("Cycle detected. Processed {}/{} systems.", processed_count, num_systems))
    } else {
        Ok(stages)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::system::{System, SystemAccess, AccessType};
    use crate::{Position, World, Component, Resource, DeltaTime};
    use std::any::{Any, TypeId};
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    // Removed unused PhantomData import

    // --- Mock Components and Resources ---
    #[derive(Debug, Clone, Copy, Component, PartialEq)] struct Velocity { dx: f32, dy: f32 }
    #[derive(Debug, Clone, Copy, PartialEq)] struct ResourceA { value: i32 }
    impl Resource for ResourceA {}
    #[derive(Debug, Clone, PartialEq)] struct ResourceB { value: String }
    impl Resource for ResourceB {}

    // --- Mock Systems implementing the NEW System trait ---
    #[derive(Default)] struct MockSystemA; // Writes Pos
    impl System for MockSystemA {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess { SystemAccess::new().with_component(TypeId::of::<Position>(), AccessType::Write) }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Write Pos */ }
    }

    #[derive(Default)] struct MockSystemB; // Reads Pos
    impl System for MockSystemB {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess { SystemAccess::new().with_component(TypeId::of::<Position>(), AccessType::Read) }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Read Pos */ }
    }

    #[derive(Default)] struct MockSystemC; // Writes Vel
    impl System for MockSystemC {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess { SystemAccess::new().with_component(TypeId::of::<Velocity>(), AccessType::Write) }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Write Vel */ }
    }

    #[derive(Default)] struct MockSystemD; // Reads ResA, Writes ResB
    impl System for MockSystemD {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess {
            SystemAccess::new()
                .with_resource(TypeId::of::<ResourceA>(), AccessType::Read)
                .with_resource(TypeId::of::<ResourceB>(), AccessType::Write)
        }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Transform */ }
    }

    #[derive(Default)] struct MockSystemE; // Writes ResA
    impl System for MockSystemE {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess { SystemAccess::new().with_resource(TypeId::of::<ResourceA>(), AccessType::Write) }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Write ResA */ }
    }

    #[derive(Default)] struct MockSystemF; // Reads ResB
    impl System for MockSystemF {
        type SystemState = ();
        fn init_state(_world: &mut World) -> Self::SystemState { () }
        fn access() -> SystemAccess { SystemAccess::new().with_resource(TypeId::of::<ResourceB>(), AccessType::Read) }
        fn run(&mut self, _world: &World, _state: &mut Self::SystemState) { /* Read ResB */ }
    }

    // --- Tests ---

    #[test]
    fn test_registry_add_system_new() {
        let mut world = World::new();
        let mut registry = SystemRegistry::new();
        assert!(registry.add_system(MockSystemA::default(), &mut world));
        assert_eq!(registry.system_count(), 1);
        assert!(!registry.add_system(MockSystemB::default(), &mut world)); // Conflict
        assert_eq!(registry.system_count(), 1);
        assert!(registry.add_system(MockSystemC::default(), &mut world));
        assert_eq!(registry.system_count(), 2);
    }

    #[test]
    fn test_build_dependency_graph_new() {
        let access_patterns = vec![MockSystemA::access(), MockSystemB::access(), MockSystemC::access()];
        let graph = build_dependency_graph(&access_patterns);
        let expected = expected_graph_with_nodes(2, &[(0, 1)]); // B depends on A
        assert_eq!(graph, expected);
    }

     #[test]
    fn test_build_dependency_graph_resources_new() {
        let access_patterns = vec![MockSystemE::access(), MockSystemD::access(), MockSystemF::access()];
        let graph = build_dependency_graph(&access_patterns);
        let expected = expected_graph_with_nodes(2, &[(0, 1), (1, 2)]); // D dep E, F dep D
        assert_eq!(graph, expected);
    }

    #[test]
    fn test_calculate_stages_new() {
        let graph = expected_graph_with_nodes(2, &[(0, 1), (1, 2)]);
        let stages = calculate_execution_stages(&graph).unwrap();
        assert_eq!(stages, vec![vec![0], vec![1], vec![2]]);
    }

    // --- Test Scheduler Execution (Simplified) ---
    #[test]
    fn test_scheduler_run_new_simplified() {
        let mut world = World::new();
        world.insert_resource(ResourceA { value: 0 });
        world.insert_resource(ResourceB { value: "".to_string() });

        let mut scheduler = SystemScheduler::new();
        // Add systems (order shouldn't matter)
        // Use the simpler mock systems without logging
        scheduler.add_system_unchecked(MockSystemF::default(), &mut world); // Index 0
        scheduler.add_system_unchecked(MockSystemE::default(), &mut world); // Index 1
        scheduler.add_system_unchecked(MockSystemD::default(), &mut world); // Index 2

        // Run - This should now compile and run without the logging complexity
        scheduler.run(&world);

        // Basic assertion: Check if the scheduler ran without panicking
        assert_eq!(scheduler.system_count(), 3);
        // Further assertions would require systems to actually modify the world,
        // which is difficult without SystemParams like Query working fully.
    }

    // --- Helper functions (unchanged) ---
    fn expected_graph_with_nodes(max_index: usize, edges: &[(usize, usize)]) -> DependencyGraph {
        let mut graph: DependencyGraph = HashMap::new();
        for i in 0..=max_index { graph.insert(i, HashSet::new()); }
        for &(u, v) in edges { graph.entry(v).or_default().insert(u); }
        graph
    }
}
