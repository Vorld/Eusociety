// Damn. Claude is amazing.

// Vertex shader
const vertexShaderSource = `
    attribute vec2 a_position;
    uniform vec2 u_resolution;
    uniform vec2 u_viewport;
    uniform vec2 u_size;

    void main() {
        // Adjust position by viewport offset and convert to clip space
        vec2 position = a_position - u_viewport;
        vec2 zeroToOne = position / u_resolution;
        vec2 zeroToTwo = zeroToOne * 2.0;
        vec2 clipSpace = zeroToTwo - 1.0;
        gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);
        gl_PointSize = u_size.x;
    }
`;

// Fragment shader
const fragmentShaderSource = `
    precision mediump float;
    uniform vec4 u_color;
    
    void main() {
        // For ants, use a circular shape
        vec2 coord = gl_PointCoord - vec2(0.5);
        float r = length(coord);
        float alpha = 1.0 - smoothstep(0.45, 0.5, r);
        
        // For food (square shape), just render it directly
        gl_FragColor = vec4(u_color.rgb, u_color.a * alpha);
    }
`;

class WebGLAntSimulation {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.gl = this.canvas.getContext('webgl', { antialias: true });
        
        if (!this.gl) {
            throw new Error('WebGL not supported');
        }

        // Initialize properties
        this.viewportX = 0;
        this.viewportY = 0;
        this.targetViewportX = 0;
        this.targetViewportY = 0;
        this.isDragging = false;
        this.lastX = 0;
        this.lastY = 0;
        this.ants = [];
        this.foods = []; 
        this.pheromones = [];
        this.SMOOTHING_FACTOR = 0.2;
        
        // Setup WebGL
        this.setupWebGL();
        this.setupEventListeners();
        this.resize();
    }

    setupWebGL() {
        const gl = this.gl;

        // Create and compile shaders
        const vertexShader = this.createShader(gl.VERTEX_SHADER, vertexShaderSource);
        const fragmentShader = this.createShader(gl.FRAGMENT_SHADER, fragmentShaderSource);

        // Create program
        this.program = this.createProgram(vertexShader, fragmentShader);
        
        // Get locations
        this.positionLocation = gl.getAttribLocation(this.program, 'a_position');
        this.resolutionLocation = gl.getUniformLocation(this.program, 'u_resolution');
        this.viewportLocation = gl.getUniformLocation(this.program, 'u_viewport');
        this.sizeLocation = gl.getUniformLocation(this.program, 'u_size');
        this.colorLocation = gl.getUniformLocation(this.program, 'u_color');

        // Create buffer
        this.antBuffer = gl.createBuffer();
        this.foodBuffer = gl.createBuffer();
        this.pheromoneBuffer = gl.createBuffer();

        // Set up initial state
        gl.useProgram(this.program);
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }

    createShader(type, source) {
        const gl = this.gl;
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);

        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error(gl.getShaderInfoLog(shader));
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
            console.error(gl.getProgramInfoLog(program));
            return null;
        }
        return program;
    }

    setupEventListeners() {
        this.canvas.addEventListener('mousedown', this.handleMouseDown.bind(this));
        this.canvas.addEventListener('mousemove', this.handleMouseMove.bind(this));
        this.canvas.addEventListener('mouseup', this.handleMouseUp.bind(this));
        this.canvas.addEventListener('mouseleave', this.handleMouseUp.bind(this));
        window.addEventListener('resize', this.resize.bind(this));
    }

    handleMouseDown(e) {
        this.isDragging = true;
        this.lastX = e.clientX;
        this.lastY = e.clientY;
        this.canvas.style.cursor = 'grabbing';
    }

    handleMouseMove(e) {
        if (!this.isDragging) return;
        
        const dx = e.clientX - this.lastX;
        const dy = e.clientY - this.lastY;
        
        this.targetViewportX -= dx;
        this.targetViewportY -= dy;
        
        this.lastX = e.clientX;
        this.lastY = e.clientY;
    }

    handleMouseUp() {
        this.isDragging = false;
        this.canvas.style.cursor = 'grab';
    }

    resize() {
        const displayWidth = this.canvas.clientWidth;
        const displayHeight = this.canvas.clientHeight;

        if (this.canvas.width !== displayWidth || this.canvas.height !== displayHeight) {
            this.canvas.width = displayWidth;
            this.canvas.height = displayHeight;
            this.gl.viewport(0, 0, this.canvas.width, this.canvas.height);
        }
    }

    updateAnts(newAnts) {
        this.ants = newAnts;
        
        // Update vertex data
        const positions = new Float32Array(newAnts.length * 2);
        newAnts.forEach((ant, i) => {
            positions[i * 2] = ant.posX;
            positions[i * 2 + 1] = ant.posY;
        });

        const gl = this.gl;
        gl.bindBuffer(gl.ARRAY_BUFFER, this.antBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, positions, gl.DYNAMIC_DRAW);
    }

    updateFoods(newFoods) {
      this.foods = newFoods;
      
      // Update vertex data
      const foodPositions = new Float32Array(newFoods.length * 2);
      newFoods.forEach((food, i) => {
        foodPositions[i * 2] = food.posX;
        foodPositions[i * 2 + 1] = food.posY;
      });

      const gl = this.gl;
      gl.bindBuffer(gl.ARRAY_BUFFER, this.foodBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, foodPositions, gl.DYNAMIC_DRAW);
  }

  updatePheromones(newPheromones) {
    this.pheromones = newPheromones;
    
    // Update vertex data
    const pheromonePositions = new Float32Array(newPheromones.length * 2);
    newPheromones.forEach((pheromone, i) => {
        pheromonePositions[i * 2] = pheromone.posX;
        pheromonePositions[i * 2 + 1] = pheromone.posY;
    });

    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.pheromoneBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, pheromonePositions, gl.DYNAMIC_DRAW);
    console.log(this.pheromones);
}

    render() {
        const gl = this.gl;

        // Smooth viewport movement
        this.viewportX += (this.targetViewportX - this.viewportX) * this.SMOOTHING_FACTOR;
        this.viewportY += (this.targetViewportY - this.viewportY) * this.SMOOTHING_FACTOR;

        // Clear canvas
        gl.clearColor(1, 1, 1, 1);
        gl.clear(gl.COLOR_BUFFER_BIT);

        // Set uniforms
        gl.uniform2f(this.resolutionLocation, gl.canvas.width, gl.canvas.height);
        gl.uniform2f(this.viewportLocation, this.viewportX, this.viewportY);

        // Draw ants
        gl.uniform2f(this.sizeLocation, 10, 10); // Ant size
        gl.uniform4f(this.colorLocation, 0, 0, 0, 1); // Black color
        this.drawEntities(this.antBuffer, this.ants.length);

        // Draw food
        gl.uniform4f(this.colorLocation, 1, 0, 0, 1); // Food color (red)
        gl.uniform2f(this.sizeLocation, 20, 20); // Food size
        this.drawEntities(this.foodBuffer, this.foods.length);

        // Draw food
        gl.uniform4f(this.colorLocation, 1, 0, 1, 1); // Pheromone color (red)
        gl.uniform2f(this.sizeLocation, 20, 20); // Pheromone size
        this.drawEntities(this.pheromoneBuffer, this.pheromones.length);

        // Request next frame
        requestAnimationFrame(this.render.bind(this));
    }

    drawEntities(buffer, count) {
        const gl = this.gl;
        gl.enableVertexAttribArray(this.positionLocation);
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.vertexAttribPointer(this.positionLocation, 2, gl.FLOAT, false, 0, 0);
        gl.drawArrays(gl.POINTS, 0, count);
    }

    start() {
        this.render();
    }
}

// Usage
const simulation = new WebGLAntSimulation('simulation-canvas');

// Connect to WebSocket
const socket = new WebSocket('ws://localhost:3000');

socket.onopen = () => {
    console.log('Connected to server');
};

socket.onmessage = (event) => {
    const simulationState = JSON.parse(event.data);
    simulation.updateAnts(simulationState.ants);
    simulation.updateFoods(simulationState.foods);
    simulation.updatePheromones(simulationState.pheromones);
};

// Start the simulation
simulation.start();