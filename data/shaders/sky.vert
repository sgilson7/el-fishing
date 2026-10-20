#version 300 es
// One triangle over the screen; each corner carries its view ray.
in vec3 aPos;
uniform vec3 uFwd;
uniform vec3 uRight;
uniform vec3 uUp;
out vec3 vRay;
void main() {
  vRay = uFwd + uRight * aPos.x + uUp * aPos.y;
  gl_Position = vec4(aPos.xy, 1.0, 1.0);
}
