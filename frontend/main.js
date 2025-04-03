(() => {
    const canvas = document.getElementById("simulation-canvas");
    const gl = canvas.getContext("webgl", { antialias: true });
    if (!gl) return alert("WebGL not supported");
  
    let worldScale = 3000.0;  // This is your base scale factor
    let zoomLevel = 1.0;

    // Vertex shader with fixed coordinate handling
    const vsSource = `
      attribute vec2 a_position;
      uniform vec2 u_resolution;
      uniform vec2 u_viewport;
      uniform float u_zoomLevel;
      
      void main() {
        // Position relative to viewport center
        vec2 pos = a_position - u_viewport;
        
        // Apply zoom level 
        vec2 zoomedPos = pos / (3000.0 * u_zoomLevel);
        
        // Correct for aspect ratio
        float aspect = u_resolution.x / u_resolution.y;
        zoomedPos.y *= aspect;
        
        // Convert to clip space
        gl_Position = vec4(zoomedPos, 0, 1);
        
        // Adjust point size based on zoom level
        gl_PointSize = max(2.0, 5.0 / u_zoomLevel);
      }
    `;
  
    // Fragment shader (circular points)
    const fsSource = `
      precision mediump float;
      void main() {
        vec2 coord = gl_PointCoord - vec2(0.5);
        if (length(coord) > 0.5) discard;
        gl_FragColor = vec4(0, 0, 0, 1);
      }
    `;
  
    function compile(type, src) {
      const s = gl.createShader(type);
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
        console.error(gl.getShaderInfoLog(s));
        gl.deleteShader(s);
        return null;
      }
      return s;
    }
  
    const program = gl.createProgram();
    gl.attachShader(program, compile(gl.VERTEX_SHADER, vsSource));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, fsSource));
    gl.linkProgram(program);
    gl.useProgram(program);
  
    let posLoc = gl.getAttribLocation(program, "a_position");
    let resLoc = gl.getUniformLocation(program, "u_resolution");
    let vpLoc  = gl.getUniformLocation(program, "u_viewport");
    let zoomLoc = gl.getUniformLocation(program, "u_zoomLevel");
  
    // Use a single buffer reference, not two different ones
    const glBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, glBuffer);
    
    // Initialize with an empty array (important!)
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(0), gl.DYNAMIC_DRAW);
    
    // Enable vertex attribute array immediately
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
  
    // Enable alpha blending
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  
    let numParticles = 0;
    // Initialize at center of simulation with zoomed-out view
    let viewportX = 3000, viewportY = 3000;  
    let targetX = 3000, targetY = 3000;
    const SMOOTH = 0.2;
  
    let dragging = false, lastX = 0, lastY = 0;
    canvas.style.cursor = "grab";
  
    // Pan control with proper scaling
    canvas.addEventListener("mousedown", e => {
      dragging = true; 
      lastX = e.clientX; 
      lastY = e.clientY;
      canvas.style.cursor = "grabbing";
    });
    
    canvas.addEventListener("mousemove", e => {
      if (!dragging) return;
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      
      // Scale by zoom level - when zoomed in (small zoom value), 
      // panning should move a smaller distance in world space
      const panScale = zoomLevel; 
      
      targetX -= dx * panScale * (worldScale / canvas.width);
      targetY += dy * panScale * (worldScale / canvas.height);
      
      lastX = e.clientX;
      lastY = e.clientY;
    });

    ["mouseup","mouseleave"].forEach(evt => 
      canvas.addEventListener(evt, () => {
        dragging = false;
        canvas.style.cursor = "grab";
      })
    );
  
    window.addEventListener("resize", () => {
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
    });
  
    function render() {
      // Sync canvas size with display size
      if (canvas.width !== canvas.clientWidth || canvas.height !== canvas.clientHeight) {
        canvas.width = canvas.clientWidth;
        canvas.height = canvas.clientHeight;
        gl.viewport(0, 0, canvas.width, canvas.height);
      }
    
      // Smooth viewport movement
      viewportX += (targetX - viewportX) * SMOOTH;
      viewportY += (targetY - viewportY) * SMOOTH;
    
      // Clear the canvas
      gl.clearColor(0.95, 0.95, 0.95, 1.0); // Light gray background
      gl.clear(gl.COLOR_BUFFER_BIT);
    
      // Pass canvas dimensions for aspect ratio calculation
      gl.uniform2f(resLoc, canvas.width, canvas.height);
      
      // Pass viewport center position
      gl.uniform2f(vpLoc, viewportX, viewportY);
      
      // Pass the zoom level to the shader
      gl.uniform1f(zoomLoc, zoomLevel);
    
      // Draw the particles
      gl.drawArrays(gl.POINTS, 0, numParticles);
    
      requestAnimationFrame(render);
    }
    requestAnimationFrame(render);
  
    const socket = new WebSocket("ws://127.0.0.1:3030");
    socket.binaryType = 'arraybuffer'; // Set binary data type

    // Add these at the beginning of your code
    let accumulatedChunks = [];
    let accumulatedSize = 0;

    // Single message handler
    socket.onmessage = async ({ data }) => {
        try {
            if (data instanceof ArrayBuffer) {
                // Add new chunk to accumulated data
                const chunk = new Uint8Array(data);
                accumulatedChunks.push(chunk);
                accumulatedSize += chunk.length;
                
                // Check if this is the last chunk (less than MAX_CHUNK_SIZE)
                if (chunk.length < 65536) {
                    // Combine all chunks
                    const combined = new Uint8Array(accumulatedSize);
                    let offset = 0;
                    for (const chunk of accumulatedChunks) {
                        combined.set(chunk, offset);
                        offset += chunk.length;
                    }
                    
                    // Create a view for the combined data
                    const view = new DataView(combined.buffer);
                    // Calculate the correct entity size - each entity has type(1) + id(4) + x(4) + y(4) = 13 bytes
                    numParticles = Math.floor(combined.length / 13);
                    
                    const flat = new Float32Array(numParticles * 2);
                    for (let i = 0; i < numParticles; i++) {
                        const offset = i * 13;
                        flat[i * 2] = view.getFloat32(offset + 5, true);     // X at offset 5
                        flat[i * 2 + 1] = view.getFloat32(offset + 9, true); // Y at offset 9
                    }

                    // Debug output
                    console.log(`Processed ${numParticles} particles from ${accumulatedSize} bytes`);
                    
                    // Update WebGL buffer
                    gl.bindBuffer(gl.ARRAY_BUFFER, glBuffer);
                    gl.bufferData(gl.ARRAY_BUFFER, flat, gl.DYNAMIC_DRAW);
                    
                    // Reset for next message
                    accumulatedChunks = [];
                    accumulatedSize = 0;
                }
            } else {
                console.error('Received non-binary message');
            }
        } catch (err) {
            console.error('Error processing binary data:', err);
            console.error(err.stack);
            // Reset on error
            accumulatedChunks = [];
            accumulatedSize = 0;
        }
    };

    socket.onerror = err => console.error(err);
    socket.onclose = () => console.log("WebSocket closed");

    // Add zoom control
    canvas.addEventListener('wheel', (e) => {
      e.preventDefault();
      const zoomFactor = 1.1;
      
      if (e.deltaY < 0) {
        // Zoom in
        zoomLevel /= zoomFactor;
      } else {
        // Zoom out
        zoomLevel *= zoomFactor;
      }
      
      // Clamp zoom level to reasonable range
      zoomLevel = Math.max(0.1, Math.min(5.0, zoomLevel));
      
      // No need to recompile shader, we just update the uniform
    });
  })();
