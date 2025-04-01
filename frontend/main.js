// DOM Elements
const canvas = document.getElementById('glCanvas');
const statusSpan = document.getElementById('status');
const fpsSpan = document.getElementById('fps');
const walkerCountSpan = document.getElementById('walkerCount');

// --- Shaders ---
const vertexShaderSource = `
    attribute vec2 a_position;

    uniform vec2 u_resolution; // Canvas resolution
    uniform vec2 u_viewport_origin; // World coordinates at top-left of viewport
    uniform float u_zoom; // Zoom level
    uniform float u_point_size;

    void main() {
        // Calculate position relative to the viewport origin, scaled by zoom
        vec2 scaled_pos = (a_position - u_viewport_origin) * u_zoom;

        // Convert to clip space
        vec2 zeroToOne = scaled_pos / u_resolution;
        vec2 zeroToTwo = zeroToOne * 2.0;
        vec2 clipSpace = zeroToTwo - 1.0;

        gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1); // Flip Y
        // Scale point size by zoom level. u_point_size now acts as a base size at zoom=1.0
        gl_PointSize = u_point_size * u_zoom;
    }
`;

const fragmentShaderSource = `
    precision mediump float;
    uniform vec4 u_color;

    void main() {
        // Simple square points for now
        gl_FragColor = u_color;

        /* // Optional: Circular points like oldproject.js
        vec2 coord = gl_PointCoord - vec2(0.5);
        float r = length(coord);
        float alpha = 1.0 - smoothstep(0.45, 0.5, r); // Adjust smoothing as needed
        gl_FragColor = vec4(u_color.rgb, u_color.a * alpha);
        */
    }
`;

// --- WebGL Renderer Class ---
class EusocietyWebGLRenderer {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.gl = this.canvas.getContext('webgl'); // Use WebGL1 for simplicity, matches oldproject
        if (!this.gl) {
            throw new Error('WebGL not supported');
        }

        // View State
        this.view = {
            worldWidth: 1000.0, // From config
            worldHeight: 1000.0, // From config
            viewportX: 500.0, // World coord at center X
            viewportY: 500.0, // World coord at center Y
            targetViewportX: 500.0,
            targetViewportY: 500.0,
            zoom: 1.0, // 1.0 = world size matches canvas size initially (approx)
            targetZoom: 1.0,
            isDragging: false,
            lastX: 0,
            lastY: 0,
            lerpFactor: 0.2 // Smoothing factor
        };

        // Data
        this.walkers = []; // Array of { x, y }

        // Timing
        this.lastFrameTime = 0;
        this.frameCount = 0;
        this.fps = 0;
        this.lastFpsUpdate = 0;

        // Setup
        this.setupWebGL();
        this.setupEventListeners();
        this.resize(); // Initial resize
    }

    setupWebGL() {
        const gl = this.gl;

        const vertexShader = this.createShader(gl.VERTEX_SHADER, vertexShaderSource);
        const fragmentShader = this.createShader(gl.FRAGMENT_SHADER, fragmentShaderSource);
        this.program = this.createProgram(vertexShader, fragmentShader);

        // Locations
        this.positionLocation = gl.getAttribLocation(this.program, 'a_position');
        this.resolutionLocation = gl.getUniformLocation(this.program, 'u_resolution');
        this.viewportOriginLocation = gl.getUniformLocation(this.program, 'u_viewport_origin');
        this.zoomLocation = gl.getUniformLocation(this.program, 'u_zoom');
        this.pointSizeLocation = gl.getUniformLocation(this.program, 'u_point_size');
        this.colorLocation = gl.getUniformLocation(this.program, 'u_color');

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

    updateWalkers(newWalkersData) {
        // Expecting [{x, y}, ...]
        this.walkers = newWalkersData;
        walkerCountSpan.textContent = this.walkers.length; // Update info panel

        // Update GPU buffer
        const positions = new Float32Array(this.walkers.length * 2);
        for (let i = 0; i < this.walkers.length; i++) {
            positions[i * 2] = this.walkers[i].x;
            positions[i * 2 + 1] = this.walkers[i].y;
        }
        const gl = this.gl;
        gl.bindBuffer(gl.ARRAY_BUFFER, this.walkerBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, positions, gl.DYNAMIC_DRAW);
        // console.log(`GPU buffer updated with ${this.walkers.length} walkers.`);
    }

    render(currentTime) {
        const gl = this.gl;
        currentTime *= 0.001; // seconds

        // --- Timing & FPS ---
        const deltaTime = currentTime - this.lastFrameTime;
        this.lastFrameTime = currentTime;
        this.frameCount++;
        if (currentTime - (this.lastFpsUpdate || 0) > 1) {
            this.fps = this.frameCount;
            this.frameCount = 0;
            fpsSpan.textContent = this.fps;
            this.lastFpsUpdate = currentTime;
        }

        // --- View Interpolation ---
        this.view.viewportX += (this.view.targetViewportX - this.view.viewportX) * this.view.lerpFactor;
        this.view.viewportY += (this.view.targetViewportY - this.view.viewportY) * this.view.lerpFactor;
        this.view.zoom += (this.view.targetZoom - this.view.zoom) * this.view.lerpFactor;

        // --- Drawing ---
        this.resize(); // Check resize
        gl.clearColor(0,0,0,0); // Dark background
        gl.clear(gl.COLOR_BUFFER_BIT);

        // Calculate viewport origin (top-left corner in world coords)
        const viewportWorldWidth = gl.canvas.width / this.view.zoom;
        const viewportWorldHeight = gl.canvas.height / this.view.zoom;
        const viewportOriginX = this.view.viewportX - viewportWorldWidth / 2;
        const viewportOriginY = this.view.viewportY - viewportWorldHeight / 2;

        // Set uniforms
        gl.uniform2f(this.resolutionLocation, gl.canvas.width, gl.canvas.height);
        gl.uniform2f(this.viewportOriginLocation, viewportOriginX, viewportOriginY);
        gl.uniform1f(this.zoomLocation, this.view.zoom);
        gl.uniform1f(this.pointSizeLocation, 2.0); // Base walker size (pixels at zoom=1.0)
        gl.uniform4f(this.colorLocation, 0.0, 0.0, 0.0, 1.0); // Black

        // Draw walkers
        const walkerCount = this.walkers.length;
        if (walkerCount > 0) {
            gl.enableVertexAttribArray(this.positionLocation);
            gl.bindBuffer(gl.ARRAY_BUFFER, this.walkerBuffer);
            gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);
            gl.drawArrays(gl.POINTS, 0, walkerCount);
            // console.log(`Drawing ${walkerCount} walkers.`);
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

    // --- WebSocket ---
    const socketUrl = 'ws://127.0.0.1:8080';
    let socket = null;

    function connectWebSocket() {
        statusSpan.textContent = 'Connecting...';
        socket = new WebSocket(socketUrl);

        socket.onopen = () => {
            console.log('WebSocket connection established.');
            statusSpan.textContent = 'Connected';
        };

        socket.onmessage = (event) => {
            try {
                // console.log('Raw WebSocket data:', event.data);
                const worldState = JSON.parse(event.data);
                if (typeof worldState === 'object' && worldState !== null && Array.isArray(worldState.entities)) {
                    // Pass only position data to renderer
                    const walkerPositions = worldState.entities.map(e => ({ x: e.x, y: e.y }));
                    renderer.updateWalkers(walkerPositions);
                } else {
                    console.warn('Received unexpected data format:', worldState);
                }
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e);
            }
        };

        socket.onerror = (error) => {
            console.error('WebSocket Error:', error);
            statusSpan.textContent = 'Error';
        };

        socket.onclose = () => {
            console.log('WebSocket connection closed. Attempting to reconnect...');
            statusSpan.textContent = 'Disconnected';
            renderer.updateWalkers([]); // Clear walkers on disconnect
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
