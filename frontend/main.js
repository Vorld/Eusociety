// DOM Elements
const canvas = document.getElementById('glCanvas');
const statusSpan = document.getElementById('status');
const fpsSpan = document.getElementById('fps');
const walkerCountSpan = document.getElementById('walkerCount');

// --- Shaders ---
const vertexShaderSource = `#version 300 es
    in vec2 a_position; // World coordinates

    uniform vec2 u_resolution; // Canvas resolution (pixels)
    uniform vec2 u_viewport_center; // World coordinates at the center of the viewport
    uniform float u_zoom; // Zoom level
    uniform float u_point_size;

    void main() {
        // Calculate visible world dimensions based on canvas size and zoom
        float visibleWorldWidth = u_resolution.x / u_zoom;
        float visibleWorldHeight = u_resolution.y / u_zoom;

        // Calculate position relative to the viewport center
        vec2 relativePos = a_position - u_viewport_center;

        // Normalize the relative position to the range [-1, 1]
        // Divide by half the visible world dimensions
        vec2 normalizedPos = vec2(
            relativePos.x / (visibleWorldWidth / 2.0),
            relativePos.y / (visibleWorldHeight / 2.0)
        );

        // Assign to gl_Position, flipping Y
        gl_Position = vec4(normalizedPos.x, -normalizedPos.y, 0.0, 1.0);

        // Scale point size by zoom level
        gl_PointSize = u_point_size * u_zoom;
    }
`;

const fragmentShaderSource = `#version 300 es
    precision mediump float;
    uniform vec4 u_color;

    out vec4 outColor; // Output variable for fragment color

    void main() {
        // Circular points
        vec2 coord = gl_PointCoord - vec2(0.5); // Center the coordinate system
        float r = length(coord); // Distance from center
        // Use smoothstep for anti-aliasing the edge
        float alpha = 1.0 - smoothstep(0.45, 0.5, r);
        if (alpha <= 0.0) {
             discard; // Discard pixels outside the circle
        }
        outColor = vec4(u_color.rgb, u_color.a * alpha);
    }
`;

// --- Binary Parser ---
class BinaryParticleParser {
    constructor() {
        // Cache DataViews for better performance with large numbers of particles
        this.cachedViews = new Map();
    }

    parse(buffer) {
        const view = new DataView(buffer);
        let offset = 0;

        // Read frame number (u64)
        const frameLow = view.getUint32(offset, true); offset += 4;
        const frameHigh = view.getUint32(offset, true); offset += 4;

        // Read timestamp (f64)
        const timestamp = view.getFloat64(offset, true); offset += 8;

        // Read particle count (u64, but we only need the lower 32 bits for JS)
        // bincode serializes Vec length as u64 by default.
        const particleCountLow = view.getUint32(offset, true); offset += 4;
        const particleCountHigh = view.getUint32(offset, true); offset += 4; // Advance offset by 8 bytes total
        const particleCount = particleCountLow; // Use lower 32 bits

        // Read particles
        const particles = [];
        for (let i = 0; i < particleCount; i++) {
            // Read particle ID (usize/u32)
            const id = view.getUint32(offset, true); offset += 4;

            // Read x position (f32)
            const x = view.getFloat32(offset, true); offset += 4;
            // Read y position (f32)
            const y = view.getFloat32(offset, true); offset += 4;

            // Add NaN check for debugging
            if (isNaN(x) || isNaN(y)) {
                console.warn(`Parsed NaN position for particle ID ${id}: (${x}, ${y})`);
                // Skip adding this particle if its position is invalid
                continue;
            }

            particles.push({ id, x, y });
        }

        return {
            frame: frameLow, // We only use the low part since JS numbers are 53-bit safe
            timestamp,
            entities: particles
        };
    }
}

// --- WebGL Renderer Class ---
class EusocietyWebGLRenderer {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.gl = this.canvas.getContext('webgl2'); // Use WebGL 2.0 as requested
        if (!this.gl) {
            // Fallback or error if WebGL2 is not supported
            console.warn("WebGL 2.0 not supported, falling back to WebGL 1.0");
            this.gl = this.canvas.getContext('webgl');
            if (!this.gl) {
                throw new Error('WebGL (1.0 or 2.0) not supported');
            }
        }

        // View State
        this.view = {
            worldWidth: 10000.0, // From config
            worldHeight: 10000.0, // From config
            viewportX: 500.0, // World coord at center X
            viewportY: 500.0, // World coord at center Y
            targetViewportX: 500.0,
            targetViewportY: 500.0,
            zoom: 1, 
            targetZoom: 1, 
            isDragging: false,
            lastX: 0,
            lastY: 0,
            lerpFactor: 0.2 // Smoothing factor
        };

        // Data & State for Interpolation
        this.previousState = null; // { timestamp: number, walkers: Map<number, {x: number, y: number}> }
        this.latestState = null;   // { timestamp: number, walkers: Map<number, {x: number, y: number}> }
        this.interpolatedPositions = new Float32Array(0); // Buffer for interpolated data

        // Timing
        this.lastRenderTime = 0; // Renamed from lastFrameTime for clarity
        this.frameCount = 0;
        this.fps = 0;
        this.lastFpsUpdate = 0;

        // Setup
        this.setupWebGL();
        this.setupEventListeners();
        this.resize(); // Initial resize
    }

    // Linear interpolation function
    lerp(a, b, t) {
        return a + (b - a) * t;
    }

    setupWebGL() {
        const gl = this.gl;

        const vertexShader = this.createShader(gl.VERTEX_SHADER, vertexShaderSource);
        const fragmentShader = this.createShader(gl.FRAGMENT_SHADER, fragmentShaderSource);
        this.program = this.createProgram(vertexShader, fragmentShader);

        // Locations
        this.positionLocation = gl.getAttribLocation(this.program, 'a_position');
        this.resolutionLocation = gl.getUniformLocation(this.program, 'u_resolution');
        this.viewportCenterLocation = gl.getUniformLocation(this.program, 'u_viewport_center'); // Changed from viewportOrigin
        this.zoomLocation = gl.getUniformLocation(this.program, 'u_zoom');
        this.pointSizeLocation = gl.getUniformLocation(this.program, 'u_point_size');
        this.colorLocation = gl.getUniformLocation(this.program, 'u_color');

        // Check if locations are valid
        if (this.positionLocation === -1 || !this.resolutionLocation || !this.viewportCenterLocation || !this.zoomLocation || !this.pointSizeLocation || !this.colorLocation) {
             console.error("Failed to get one or more shader locations!");
             // Optionally throw an error or handle appropriately
        }

        // Buffer
        this.walkerBuffer = gl.createBuffer();

        // GL Settings
        gl.useProgram(this.program);
        // gl.enable(gl.BLEND); // Optional blending for circular points
        // gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }

    createShader(type, source) {
        const gl = this.gl;
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error(`Shader compile error: ${gl.getShaderInfoLog(shader)}`);
            gl.deleteShader(shader);
            return null;
        }
        return shader;
    }

    createProgram(vertexShader, fragmentShader) {
        const gl = this.gl;
        const program = gl.createProgram();
        gl.attachShader(program, vertexShader);
        gl.attachShader(program, fragmentShader);
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error(`Program link error: ${gl.getProgramInfoLog(program)}`);
            return null;
        }
        return program;
    }

    setupEventListeners() {
        this.canvas.addEventListener('mousedown', this.handleMouseDown.bind(this));
        this.canvas.addEventListener('mousemove', this.handleMouseMove.bind(this));
        this.canvas.addEventListener('mouseup', this.handleMouseUp.bind(this));
        this.canvas.addEventListener('mouseleave', this.handleMouseUp.bind(this));
        this.canvas.addEventListener('wheel', this.handleWheel.bind(this));
        window.addEventListener('resize', this.resize.bind(this));
        this.canvas.style.cursor = 'grab';
    }

    // Method to update the simulation state received from the backend
    updateSimulationState(newState) {
        // Shift latest to previous
        this.previousState = this.latestState;
        // Store the new state, converting array to Map using particle ID
        const walkerMap = new Map();
        newState.entities.forEach(p => {
            // Ensure positions are valid numbers before adding
            if (!isNaN(p.x) && !isNaN(p.y)) {
                walkerMap.set(p.id, { x: p.x, y: p.y });
            } else {
                 console.warn(`Received invalid position for particle ID ${p.id}: (${p.x}, ${p.y}). Skipping.`);
            }
        });
        this.latestState = {
            timestamp: newState.timestamp, // Use backend timestamp
            walkers: walkerMap
        };

        // Update walker count display
        walkerCountSpan.textContent = this.latestState.walkers.size;

        // Don't update the GPU buffer here directly anymore.
        // It will be updated in the render loop with interpolated data.
    }

    handleMouseDown(e) {
        this.view.isDragging = true;
        this.view.lastX = e.clientX;
        this.view.lastY = e.clientY;
        this.canvas.style.cursor = 'grabbing';
    }

    handleMouseMove(e) {
        if (!this.view.isDragging) return;
        const dx = e.clientX - this.view.lastX;
        const dy = e.clientY - this.view.lastY;

        // Adjust target viewport center based on mouse delta, scaled by zoom
        // Panning moves the viewport origin inversely to mouse movement
        this.view.targetViewportX -= dx / this.view.zoom;
        this.view.targetViewportY -= dy / this.view.zoom; // Y-axis is flipped in shader

        this.view.lastX = e.clientX;
        this.view.lastY = e.clientY;
    }

    handleMouseUp() {
        this.view.isDragging = false;
        this.canvas.style.cursor = 'grab';
    }

    handleWheel(event) {
        event.preventDefault();
        const scale = event.deltaY * -0.001; // Adjust sensitivity
        const zoomFactor = Math.exp(scale);

        this.view.targetZoom *= zoomFactor;

        // Clamp zoom
        const minZoom = 0.1;
        const maxZoom = 10.0;
        this.view.targetZoom = Math.max(minZoom, Math.min(maxZoom, this.view.targetZoom));

        // TODO: Zoom towards mouse cursor (more complex)
    }


    resize() {
        const displayWidth = this.canvas.clientWidth;
        const displayHeight = this.canvas.clientHeight;
        if (this.canvas.width !== displayWidth || this.canvas.height !== displayHeight) {
            this.canvas.width = displayWidth;
            this.canvas.height = displayHeight;
            this.gl.viewport(0, 0, this.canvas.width, this.canvas.height);
            console.log(`Canvas resized to ${displayWidth}x${displayHeight}`);
        }
    }

    // updateWalkers is removed - state update is handled by updateSimulationState

    render(currentTime) { // currentTime is from requestAnimationFrame (milliseconds)
        const gl = this.gl;
        const now = performance.now(); // Use high-resolution timer

        // --- Timing & FPS ---
        const deltaTime = (now - this.lastRenderTime) / 1000.0; // Delta time in seconds
        this.lastRenderTime = now;
        this.frameCount++;
        // Use performance.now() for FPS calculation consistency
        if (now - (this.lastFpsUpdate || 0) > 1000) {
            this.fps = this.frameCount; // FPS is frames in the last second
            this.frameCount = 0;
            fpsSpan.textContent = this.fps;
            this.lastFpsUpdate = now;
        }

        // --- View Panning/Zoom Interpolation ---
        // Reintroduce lerping for smoother panning
        this.view.viewportX += (this.view.targetViewportX - this.view.viewportX) * this.view.lerpFactor;
        this.view.viewportY += (this.view.targetViewportY - this.view.viewportY) * this.view.lerpFactor;
        // Keep lerping for zoom
        this.view.zoom += (this.view.targetZoom - this.view.zoom) * this.view.lerpFactor;


        // --- Walker Position Interpolation ---
        let walkerCount = 0;
        if (this.latestState) {
            walkerCount = this.latestState.walkers.size;
            // Ensure buffer is large enough
            if (this.interpolatedPositions.length < walkerCount * 2) {
                this.interpolatedPositions = new Float32Array(walkerCount * 2);
            }

            let bufferIndex = 0;
            if (this.previousState && this.previousState.timestamp !== this.latestState.timestamp) {
                const timeSinceLastUpdate = (now / 1000.0) - this.latestState.timestamp; // Time since latest state arrived (seconds)
                const timeBetweenUpdates = this.latestState.timestamp - this.previousState.timestamp; // Time between last two states (seconds)

                // Avoid division by zero and ensure time flows forward
                // Remove Math.min(1.0, ...) to allow extrapolation
                const interpolationFactor = (timeBetweenUpdates > 0.001)
                    ? Math.max(0.0, timeSinceLastUpdate / timeBetweenUpdates) // Allow factor > 1.0
                    : 1.0; // If updates are too close or timestamps equal, just use latest

                // Log the factor if it goes above 1 for debugging extrapolation
                // if (interpolationFactor > 1.0) {
                //     console.log(`Extrapolating factor: ${interpolationFactor.toFixed(3)} (since last: ${timeSinceLastUpdate.toFixed(3)}s, between: ${timeBetweenUpdates.toFixed(3)}s)`);
                // }

                for (const [id, latestPos] of this.latestState.walkers) {
                    const previousPos = this.previousState.walkers.get(id);
                    if (previousPos) {
                        // Interpolate if particle exists in both states
                        this.interpolatedPositions[bufferIndex++] = this.lerp(previousPos.x, latestPos.x, interpolationFactor);
                        this.interpolatedPositions[bufferIndex++] = this.lerp(previousPos.y, latestPos.y, interpolationFactor);
                    } else {
                        // Particle is new, just use its latest position
                        this.interpolatedPositions[bufferIndex++] = latestPos.x;
                        this.interpolatedPositions[bufferIndex++] = latestPos.y;
                    }
                }
            } else {
                // No previous state or timestamps are the same, just use the latest positions
                for (const [id, latestPos] of this.latestState.walkers) {
                    this.interpolatedPositions[bufferIndex++] = latestPos.x;
                    this.interpolatedPositions[bufferIndex++] = latestPos.y;
                }
            }
            // Trim the buffer view if fewer walkers are present than the buffer size
            // Note: We use bufferIndex which holds the actual count of filled positions (walkerCount * 2)
            const bufferDataView = new Float32Array(this.interpolatedPositions.buffer, 0, bufferIndex);

            // Update GPU buffer with interpolated data
            gl.bindBuffer(gl.ARRAY_BUFFER, this.walkerBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, bufferDataView, gl.DYNAMIC_DRAW);

        } else {
             // No state yet, ensure buffer is empty or cleared if needed
             if (this.interpolatedPositions.length > 0) {
                 this.interpolatedPositions = new Float32Array(0);
                 gl.bindBuffer(gl.ARRAY_BUFFER, this.walkerBuffer);
                 gl.bufferData(gl.ARRAY_BUFFER, this.interpolatedPositions, gl.DYNAMIC_DRAW); // Clear GPU buffer
             }
        }


        // --- Drawing ---
        this.resize(); // Check resize
        gl.clearColor(1.0, 1.0, 1.0, 1.0); // White background
        gl.clear(gl.COLOR_BUFFER_BIT);

        // Set uniforms (View related uniforms are already interpolated)
        gl.uniform2f(this.resolutionLocation, gl.canvas.width, gl.canvas.height);
        gl.uniform2f(this.viewportCenterLocation, this.view.viewportX, this.view.viewportY);
        gl.uniform1f(this.zoomLocation, this.view.zoom);
        gl.uniform1f(this.pointSizeLocation, 2.0); // Base walker size
        gl.uniform4f(this.colorLocation, 0.0, 0.0, 0.0, 1.0); // Black particles

        // Draw walkers using the interpolated data in the buffer
        if (walkerCount > 0) {
            gl.enableVertexAttribArray(this.positionLocation);
            gl.bindBuffer(gl.ARRAY_BUFFER, this.walkerBuffer); // Buffer already bound and updated
            gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);
            gl.drawArrays(gl.POINTS, 0, walkerCount);
            // console.log(`Drawing ${walkerCount} interpolated walkers.`);
        }

        // --- Loop ---
        requestAnimationFrame(this.render.bind(this));
    }

    start() {
        requestAnimationFrame(this.render.bind(this));
    }
}

// --- Main Execution ---
try {
    const renderer = new EusocietyWebGLRenderer('glCanvas');
    const binaryParser = new BinaryParticleParser();

    // --- WebSocket ---
    const socketUrl = 'ws://127.0.0.1:8090';
    let socket = null;

    function connectWebSocket() {

        statusSpan.textContent = 'Connecting...';
        socket = new WebSocket(socketUrl);
        
        // Set binary type to arraybuffer
        socket.binaryType = 'arraybuffer';

        socket.onopen = () => {
            console.log('WebSocket connection established.');
            statusSpan.textContent = 'Connected';
        };

        socket.onmessage = (event) => {
            try {
                let worldState;
                
                // Process binary data or JSON
                if (event.data instanceof ArrayBuffer) {
                    // Binary data
                    worldState = binaryParser.parse(event.data);
                } else {
                    // JSON data (fallback)
                    worldState = JSON.parse(event.data);
                }
                
                if (worldState && Array.isArray(worldState.entities)) {
                    // Pass the full parsed state to the renderer
                    renderer.updateSimulationState(worldState);

                    // Optionally show received data count in console
                    // console.log(`Received state for ${worldState.entities.length} particles, frame: ${worldState.frame}, timestamp: ${worldState.timestamp}`);
                } else {
                    console.warn('Received unexpected data format:', worldState);
                }
            } catch (e) {
                console.error('Failed to process WebSocket message:', e);
                console.error('Error details:', e.message);
            }
        };

        socket.onerror = (error) => {
            console.error('WebSocket Error:', error);
            statusSpan.textContent = 'Error';
        };

        socket.onclose = () => {
            console.log('WebSocket connection closed. Attempting to reconnect...');
            statusSpan.textContent = 'Disconnected';
            // Clear renderer state on disconnect
            renderer.previousState = null;
            renderer.latestState = null;
            renderer.updateSimulationState({ timestamp: 0, entities: [] }); // Clear display
            setTimeout(connectWebSocket, 5000); // Reconnect logic
        };
    }

    // Start connection and rendering
    connectWebSocket();
    renderer.start();

} catch (error) {
    console.error("Failed to initialize renderer:", error);
    statusSpan.textContent = 'Init Error';
    alert(`Initialization failed: ${error.message}`);
}
