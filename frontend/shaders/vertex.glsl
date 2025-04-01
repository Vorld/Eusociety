attribute vec2 aVertexPosition;

uniform mat4 uProjectionMatrix;
uniform mat4 uViewMatrix;
uniform float uPointSize;

void main() {
  // Apply view and projection matrices to the 2D position
  // We use vec4(..., 0.0, 1.0) because matrices are 4x4
  gl_Position = uProjectionMatrix * uViewMatrix * vec4(aVertexPosition, 0.0, 1.0);

  // Set the size of the point to be rendered
  gl_PointSize = uPointSize;
}
