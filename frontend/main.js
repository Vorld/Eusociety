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
// Renamed from BinaryParticleParser
class BinaryStateParser {
    constructor() {
        // Constants for AntState enum discriminants (assuming default bincode representation)
        this.AntState = {
            Foraging: 0,
            ReturningToNest: 1,
        };
    }


    parse(buffer) {
        const view = new DataView(buffer);
        let offset = 0;
        // console.log(`Parsing buffer of size: ${buffer.byteLength}`); // Log buffer size

        try {
            // Read frame number (u64)
            if (offset + 8 > buffer.byteLength) throw new Error("Buffer too small for frame");
            const frameLow = view.getUint32(offset, true); offset += 4;
            const frameHigh = view.getUint32(offset, true); offset += 4; // Read high bits of frame

            // Read timestamp (f64) - 8 bytes
            if (offset + 8 > buffer.byteLength) throw new Error("Buffer too small for timestamp");
            const timestamp = view.getFloat64(offset, true); offset += 8;
            // console.log(`Parsed header. Frame: ${frameLow}, Timestamp: ${timestamp.toFixed(3)}, Offset: ${offset}`); // Keep console clean for now

            // --- Read Ants Vec ---
            // console.log(`Reading ant count at offset ${offset}`);
            // Read Vec length (u64) - 8 bytes
            if (offset + 8 > buffer.byteLength) throw new Error("Buffer too small for ant count");
            const antCountLow = view.getUint32(offset, true); offset += 4;
            const antCountHigh = view.getUint32(offset, true); offset += 4; // Read high bits of count
            // JS numbers are safe up to 2^53, so antCountLow should be sufficient unless > 4 billion ants
            if (antCountHigh !== 0) console.warn(`Ant count exceeds 2^32: ${antCountHigh}${antCountLow}`);
            const antCount = antCountLow;
            // console.log(`Expecting ${antCount} ants. Offset: ${offset}`);
            const ants = [];
            const antSize = 4 + 4 + 4 + 4; // id(u32) + x(f32) + y(f32) + state(u32 discriminant) = 16 bytes
            if (offset + antCount * antSize > buffer.byteLength) throw new Error(`Buffer too small for ${antCount} ants`);
            for (let i = 0; i < antCount; i++) {
                // const startOffset = offset; // No longer needed with upfront size check

                // Check if there is enough data to read a 32-bit unsigned integer
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Uint32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const id = view.getUint32(offset, true); offset += 4;

                // Check if there is enough data to read a 32-bit float
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Float32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const x = view.getFloat32(offset, true); offset += 4;

                // Check if there is enough data to read a 32-bit float
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Float32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const y = view.getFloat32(offset, true); offset += 4;

                // Check if there is enough data to read a 32-bit unsigned integer
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Uint32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const stateDiscriminant = view.getUint32(offset, true); offset += 4; // Read enum discriminant (u32)

                let state;
                switch (stateDiscriminant) {
                    case this.AntState.Foraging: state = 'Foraging'; break;
                    case this.AntState.ReturningToNest: state = 'ReturningToNest'; break;
                    default: state = 'Unknown'; console.warn(`Unknown ant state discriminant: ${stateDiscriminant}`);
                }

                if (isNaN(x) || isNaN(y)) {
                    console.warn(`Parsed NaN position for ant ID ${id}: (${x}, ${y})`);
                    continue; // Skip invalid ant
                }
                ants.push({ id, x, y, state });
            }

            // --- Read Nest Option ---
            // console.log(`Reading nest tag at offset ${offset}`);
            // Read Option tag (u8) - 1 byte
            if (offset + 1 > buffer.byteLength) throw new Error("Buffer too small for nest tag");
            const nestTag = view.getUint8(offset, true); offset += 1;
            let nest = null;
            // console.log(`Nest tag: ${nestTag}. Offset: ${offset}`);
            if (nestTag === 1) { // 1 = Some
                 // Read NestExportState data - x(f32) + y(f32) = 8 bytes
                 if (offset + 8 > buffer.byteLength) throw new Error("Buffer too small for nest data");
                const x = view.getFloat32(offset, true); offset += 4;
                const y = view.getFloat32(offset, true); offset += 4;
                if (!isNaN(x) && !isNaN(y)) {
                    nest = { x, y };
                } else {
                     console.warn(`Parsed NaN position for nest: (${x}, ${y})`);
                }
            }

            // --- Read FoodSources Vec ---
            // console.log(`Reading food count at offset ${offset}`);
            // Read Vec length (u64) - 8 bytes
             if (offset + 8 > buffer.byteLength) throw new Error("Buffer too small for food count");
            const foodCountLow = view.getUint32(offset, true); offset += 4;
            const foodCountHigh = view.getUint32(offset, true); offset += 4; // Read high bits of count
            if (foodCountHigh !== 0) console.warn(`Food source count exceeds 2^32: ${foodCountHigh}${foodCountLow}`);
            const foodCount = foodCountLow;
            // console.log(`Expecting ${foodCount} food sources. Offset: ${offset}`);
            const foodSources = [];
            const foodSize = 4 + 4 + 4; // id(u32) + x(f32) + y(f32) = 12 bytes
            if (offset + foodCount * foodSize > buffer.byteLength) throw new Error(`Buffer too small for ${foodCount} food sources`);
            for (let i = 0; i < foodCount; i++) {
                 // const startOffset = offset; // No longer needed

                // Check if there is enough data to read a 32-bit unsigned integer
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Uint32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const id = view.getUint32(offset, true); offset += 4; // FoodSourceExportState has id

                // Check if there is enough data to read a 32-bit float
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Float32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const x = view.getFloat32(offset, true); offset += 4;

                // Check if there is enough data to read a 32-bit float
                if (offset + 4 > buffer.byteLength) {
                    throw new Error(`Buffer too small to read Float32 at offset ${offset}. Buffer length: ${buffer.byteLength}`);
                }
                const y = view.getFloat32(offset, true); offset += 4;

                if (isNaN(x) || isNaN(y)) {
                    console.warn(`Parsed NaN position for food source ID ${id}: (${x}, ${y})`);
                    continue; // Skip invalid food source
                }
                foodSources.push({ id, x, y });
            }

            // --- Final Logging & Check ---
            // console.log(`Finished parsing. Offset: ${offset}, Buffer Length: ${buffer.byteLength}`); // Keep console clean
            // Log results only if something seems wrong or for initial debugging
            if (!nest || foodSources.length === 0) {
                 console.log(`Parsed Result - Ants: ${ants.length}, Nest: ${nest ? 'Exists' : 'None'}, Food: ${foodSources.length}`);
            }

            // Check if we consumed the whole buffer (optional sanity check)
            if (offset !== buffer.byteLength) {
                console.warn(`Parser did not consume entire buffer. Offset: ${offset}, Length: ${buffer.byteLength}`);
            }

            return {
                frame: frameLow, // Use low part
                timestamp,
                ants,
                nest,
                foodSources,
            };

        } catch (e) {
            // Log the specific error and the offset where it occurred
            console.error(`Error parsing binary data at offset ${offset}:`, e.message, e);
            // Return a default/empty state on error to avoid crashing the renderer
            return { frame: 0, timestamp: 0, ants: [], nest: null, foodSources: [] };
        }
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
            viewportX: 0.0, // World coord at center X (CHANGED TO 0,0 to match backend coordinate system)
            viewportY: 0.0, // World coord at center Y (CHANGED TO 0,0 to match backend coordinate system)
            targetViewportX: 0.0, // Also initialize to 0,0
            targetViewportY: 0.0, // Also initialize to 0,0
            zoom: 1, 
            targetZoom: 1, 
            isDragging: false,
            lastX: 0,
            lastY: 0,
            lerpFactor: 0.2 // Smoothing factor
        };

        // Data & State - Removing interpolation
        this.latestState = null;   // Latest state received from backend
        this.antPositions = new Float32Array(0); // Buffer for ant data
        this.nestPosition = new Float32Array(2); // Buffer for nest position
        this.foodSourcePositions = new Float32Array(0); // Buffer for food source positions

        // Timing
        this.lastRenderTime = 0;
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
        // Buffers
        this.antBuffer = gl.createBuffer();
        this.nestBuffer = gl.createBuffer();
        this.foodBuffer = gl.createBuffer();

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
        // No longer storing previous state for interpolation
        
        // Store the new state, converting arrays to Maps using IDs
        const antMap = new Map();
        newState.ants.forEach(a => {
            if (!isNaN(a.x) && !isNaN(a.y)) {
                antMap.set(a.id, { x: a.x, y: a.y, state: a.state });
            } else {
                console.warn(`Received invalid position for ant ID ${a.id}. Skipping.`);
            }
        });

        const foodMap = new Map();
        newState.foodSources.forEach(f => {
            if (!isNaN(f.x) && !isNaN(f.y)) {
                foodMap.set(f.id, { x: f.x, y: f.y });
            } else {
                console.warn(`Received invalid position for food source ID ${f.id}. Skipping.`);
            }
        });

        this.latestState = {
            timestamp: newState.timestamp,
            ants: antMap,
            nest: newState.nest,
            foodSources: foodMap
        };

        // Update walker count display
        walkerCountSpan.textContent = this.latestState.ants.size;

        // Update all buffers immediately when new state is received
        this.updateAllBuffers();
    }

    // New method to update all buffers without interpolation
    updateAllBuffers() {
        const gl = this.gl;

        // Update Nest Buffer
        if (this.latestState && this.latestState.nest) {
            this.nestPosition[0] = this.latestState.nest.x;
            this.nestPosition[1] = this.latestState.nest.y;
            gl.bindBuffer(gl.ARRAY_BUFFER, this.nestBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, this.nestPosition, gl.DYNAMIC_DRAW);
        } else {
            // Clear buffer if nest doesn't exist
            gl.bindBuffer(gl.ARRAY_BUFFER, this.nestBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(0), gl.DYNAMIC_DRAW);
        }

        // Update Food Source Buffer
        if (this.latestState && this.latestState.foodSources.size > 0) {
            const foodCount = this.latestState.foodSources.size;
            if (this.foodSourcePositions.length < foodCount * 2) {
                this.foodSourcePositions = new Float32Array(foodCount * 2);
            }
            let bufferIndex = 0;
            for (const [id, foodPos] of this.latestState.foodSources) {
                this.foodSourcePositions[bufferIndex++] = foodPos.x;
                this.foodSourcePositions[bufferIndex++] = foodPos.y;
            }
            const bufferDataView = new Float32Array(this.foodSourcePositions.buffer, 0, bufferIndex);
            gl.bindBuffer(gl.ARRAY_BUFFER, this.foodBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, bufferDataView, gl.DYNAMIC_DRAW);
        } else {
            // Clear buffer if no food sources
            gl.bindBuffer(gl.ARRAY_BUFFER, this.foodBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(0), gl.DYNAMIC_DRAW);
        }

        // Update Ant Buffer - directly use the latest state without interpolation
        if (this.latestState && this.latestState.ants.size > 0) {
            const antCount = this.latestState.ants.size;
            if (this.antPositions === undefined || this.antPositions.length < antCount * 2) {
                this.antPositions = new Float32Array(antCount * 2);
            }

            let bufferIndex = 0;
            for (const [id, ant] of this.latestState.ants) {
                this.antPositions[bufferIndex++] = ant.x;
                this.antPositions[bufferIndex++] = ant.y;
            }

            const antBufferDataView = new Float32Array(this.antPositions.buffer, 0, bufferIndex);
            gl.bindBuffer(gl.ARRAY_BUFFER, this.antBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, antBufferDataView, gl.DYNAMIC_DRAW);
        } else {
            // Clear buffer if no ants
            gl.bindBuffer(gl.ARRAY_BUFFER, this.antBuffer);
            gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(0), gl.DYNAMIC_DRAW);
        }
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
        // Keep lerping for view movement (not related to state interpolation)
        this.view.viewportX += (this.view.targetViewportX - this.view.viewportX) * this.view.lerpFactor;
        this.view.viewportY += (this.view.targetViewportY - this.view.viewportY) * this.view.lerpFactor;
        this.view.zoom += (this.view.targetZoom - this.view.zoom) * this.view.lerpFactor;

        // --- Removed Ant Position Interpolation ---
        // We no longer interpolate ant positions - we simply draw the latest positions
        // Note: antsToDraw will contain all ant information directly from the latest state
        const antsToDraw = [];
        if (this.latestState) {
            for (const [id, ant] of this.latestState.ants) {
                antsToDraw.push({ x: ant.x, y: ant.y, state: ant.state });
            }
        }

        // --- Drawing ---
        this.resize(); // Check resize
        gl.clearColor(0.95, 0.95, 0.95, 1.0); // Light grey background
        gl.clear(gl.COLOR_BUFFER_BIT);

        // Set common uniforms
        gl.uniform2f(this.resolutionLocation, gl.canvas.width, gl.canvas.height);
        gl.uniform2f(this.viewportCenterLocation, this.view.viewportX, this.view.viewportY);
        gl.uniform1f(this.zoomLocation, this.view.zoom);
        gl.enableVertexAttribArray(this.positionLocation);

        // --- Draw Nest ---
        if (this.latestState && this.latestState.nest) {
            gl.uniform1f(this.pointSizeLocation, 10.0); // Larger size for nest
            gl.uniform4f(this.colorLocation, 0.2, 0.2, 0.8, 1.0); // Blue nest
            gl.bindBuffer(gl.ARRAY_BUFFER, this.nestBuffer);
            gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);
            gl.drawArrays(gl.POINTS, 0, 1); // Draw 1 point
        }

        // --- Draw Food Sources ---
        if (this.latestState && this.latestState.foodSources.size > 0) {
            const foodCount = this.latestState.foodSources.size;
            gl.uniform1f(this.pointSizeLocation, 5.0); // Medium size for food
            gl.uniform4f(this.colorLocation, 0.2, 0.8, 0.2, 1.0); // Green food
            gl.bindBuffer(gl.ARRAY_BUFFER, this.foodBuffer);
            gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);
            gl.drawArrays(gl.POINTS, 0, foodCount);
        }

        // --- Draw Ants ---
        // We need to draw ants potentially in two batches based on state for different colors
        gl.uniform1f(this.pointSizeLocation, 2.0); // Base ant size
        gl.bindBuffer(gl.ARRAY_BUFFER, this.antBuffer);
        gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);

        // Process the ant states for drawing the correct color
        // Since we have the state information in antsToDraw array, we can still use that
        const antCount = antsToDraw.length;

        // Draw Foraging ants (Black)
        gl.uniform4f(this.colorLocation, 0.0, 0.0, 0.0, 1.0); // Black
        let foragingOffset = 0;
        let foragingCount = 0;
        for(let i = 0; i < antCount; ++i) {
            if (antsToDraw[i].state === 'Foraging') {
                if (foragingCount === 0) foragingOffset = i; // Start of a range
                foragingCount++;
            } else {
                if (foragingCount > 0) {
                    gl.drawArrays(gl.POINTS, foragingOffset, foragingCount); // Draw completed range
                }
                foragingCount = 0; // Reset count
            }
        }
        if (foragingCount > 0) gl.drawArrays(gl.POINTS, foragingOffset, foragingCount); // Draw trailing range

        // Draw ReturningToNest ants (Red)
        gl.uniform4f(this.colorLocation, 0.8, 0.1, 0.1, 1.0); // Red
        let returningOffset = 0;
        let returningCount = 0;
        for(let i = 0; i < antCount; ++i) {
            if (antsToDraw[i].state === 'ReturningToNest') {
                if (returningCount === 0) returningOffset = i;
                returningCount++;
            } else {
                if (returningCount > 0) {
                    gl.drawArrays(gl.POINTS, returningOffset, returningCount);
                }
                returningCount = 0;
            }
        }
        if (returningCount > 0) gl.drawArrays(gl.POINTS, returningOffset, returningCount);

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
    const binaryParser = new BinaryStateParser(); // Use renamed parser

    // --- WebSocket ---
    const socketUrl = 'ws://127.0.0.1:8090'; // Make sure this matches backend config
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

                // Process incoming data
                if (typeof event.data === 'string') {
                    try {
                        console.log("Successfully parsed JSON data structure:", worldState);
                    } catch (e) {
                        console.error("Failed to parse JSON:", e);
                    }
                } else if (event.data instanceof ArrayBuffer) {
                    // Binary data
                    // console.log("Received ArrayBuffer.");
                    worldState = binaryParser.parse(event.data);
                } else {
                     console.warn("Received data of unknown type:", typeof event.data);
                     return; // Don't proceed if type is unknown
                }

                // Validate the structure of the received data
                if (!worldState || !Array.isArray(worldState.ants) || !Array.isArray(worldState.foodSources)) {
                    console.warn('Invalid or incomplete data format received:', worldState);
                    return; // Exit if the data is not in the expected format
                }

                // Check for the new structure
                if (worldState && Array.isArray(worldState.ants) && Array.isArray(worldState.foodSources)) {
                    // Pass the full parsed state to the renderer
                    renderer.updateSimulationState(worldState);
                } else {
                    console.warn('Received unexpected or incomplete data format:', worldState);
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
            renderer.latestState = null;
            // Clear display with empty state matching new structure
            renderer.updateSimulationState({ timestamp: 0, ants: [], nest: null, foodSources: [] });
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
