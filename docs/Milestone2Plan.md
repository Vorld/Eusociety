# WebSocket Sender Implementation Tasks for 10,000 Particles

Below is a breakdown of tasks and subtasks for implementing an efficient WebSocket sender for simulating 10,000 particles, following your existing architecture.

## 1. Binary Serializer Implementation
**Task:** Create an efficient binary serializer implementation for the `Serializer` trait

**Subtasks:**

- [ ] Add `bincode`/`serde` dependencies to `Cargo.toml`
- [ ] Implement `BinarySerializer` struct implementing the `Serializer` trait
- [ ] Define compact binary representation for particle data (position, id, etc.)
- [ ] Benchmark serialization performance with 10,000 particles
- [ ] Implement error handling for serialization failures

## 2. WebSocket Sender Implementation
**Task:** Create a WebSocket sender implementation for the `Sender` trait

**Subtasks:**

- [ ] Add `tokio`, `tokio-tungstenite` dependencies to `Cargo.toml`
- [ ] Implement `WebSocketSender` struct implementing the `Sender` trait
- [ ] Set up async WebSocket server with client connection handling
- [ ] Implement client tracking (add/remove as they connect/disconnect)
- [ ] Implement efficient broadcasting mechanism (e.g., using `tokio::sync::broadcast`) to all connected clients, handling join/leave events gracefully.
- [ ] Add error handling and connection status monitoring

## 3. Performance Optimizations
**Task:** Implement optimizations for efficiently sending 10,000 particles

**Subtasks:**

- [ ] Implement delta compression (only send entities that have changed position)
- [ ] Add frame skipping mechanism (send only every N frames)
- [ ] Implement binary-level optimizations (fixed-point numbers vs floats)
- [ ] Add optional compression layer (LZ4/zstd) for large payloads
- [ ] Implement client-specific data throttling based on connection quality

## 4. Integration with Bevy ECS
**Task:** Integrate WebSocket sender with existing Bevy ECS simulation

**Subtasks:**

- [ ] Create `TransportController` combining serializer and sender
- [ ] Implement a Bevy system that extracts entity data at configured intervals
- [ ] Add simulation state resource to track frame numbers
- [ ] Configure non-blocking transport (e.g., spawning tasks on Tokio runtime) to prevent simulation slowdown
- [ ] Add diagnostics for monitoring transport performance

## 5. Frontend Client Implementation
**Task:** Implement WebSocket client to receive and render particle data

**Subtasks:**

- [ ] Set up WebSocket connection handling in frontend
- [ ] Implement binary data parser for received messages
- [ ] Create efficient rendering system using a custom WebGL 2.0 implementation (consider techniques like instancing).
- [ ] Add client-side viewport culling (only render visible particles)
- [ ] Implement interpolation for smooth movement between updates

## 6. Testing and Benchmarking
**Task:** Test and benchmark the WebSocket transport with 10,000 particles

**Subtasks:**

- [ ] Develop load tests simulating multiple connected clients
- [ ] Measure bandwidth usage per client
- [ ] Benchmark server-side performance (CPU, memory)
- [ ] Test various optimization configurations to find optimal settings
- [ ] Create visualization of performance metrics

## 7. Configuration Integration
**Task:** Add WebSocket transport configuration to existing config system

**Subtasks:**

- [ ] Update configuration schema to include WebSocket settings
- [ ] Add serializer configuration options (compression, precision, etc.)
- [ ] Implement configuration validation for WebSocket parameters
- [ ] Add documentation for all configuration options

## Implementation Notes
- **Binary Efficiency:** Use fixed-size structures where possible. For example, each particle could be encoded as 12 bytes (4 for ID, 4 for X, 4 for Y).
- **Parallelism:** Use Bevy's parallel query execution for extracting entity data.
- **Async Handling:** The WebSocket sender should use non-blocking I/O to avoid affecting simulation performance.
- **Client-side Filtering:** Since different clients view different parts of the simulation, implement client-side viewport culling instead of server-side.
- **Scalability:** Design the system to allow for horizontal scaling if needed (multiple WebSocket servers behind a load balancer).

This implementation should allow you to efficiently transmit data for 10,000 particles while maintaining good performance both on the server and client sides.

---

## Critique and Suggestions

This plan is comprehensive but consider the following points during implementation:

1.  **Optimization Prioritization (Sec 3):**
    *   Focus initially on core functionality and high-impact optimizations like delta compression and frame skipping.
    *   Evaluate the need for fixed-point numbers vs. floating-point carefully; the complexity might outweigh benefits initially.
    *   Benchmark before adding payload compression (LZ4/zstd) as it adds CPU overhead.
    *   Client-specific throttling is advanced; consider deferring it.
2.  **Binary Format Definition (Sec 1):** Define the *exact* byte layout, field order, data types, and endianness early and document it clearly, as both server and client rely heavily on this.
3.  **Error Handling Details (Sec 1 & 2):** Elaborate on the specific error handling strategies. How will serialization failures be managed? How will WebSocket disconnections during broadcasts be handled to ensure robustness?
4.  **Phased Implementation:** Consider an iterative approach: build a basic end-to-end version first, then layer on and benchmark optimizations one by one.
