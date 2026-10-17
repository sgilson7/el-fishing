#version 300 es
// The water's grid, displaced by the same Gerstner sum the core floats the
// kayak on (crates/core/src/sea.rs). Each wave's phase arrives as a turn
// fraction the core steps; the shader keeps no time of its own for these.
// WAVES is replaced by the page with the place's wave count from the core.
in vec3 aPos;
uniform mat4 uVP;
uniform vec2 uOrigin;
uniform float uLevel;
uniform float uTau;
// direction x, direction z, turns a metre (1 / length), phase in turns
uniform vec4 uWaveA[WAVES];
// height, sway, slope (k × height), unused
uniform vec4 uWaveB[WAVES];
// the grid's radius, and its spacing factor (4 / cells)
uniform vec2 uGrid;
// a wave fades out between these many grid spacings to its length
uniform vec2 uLod;
out vec3 vW;
out vec3 vN;
out float vFold;
out float vH;
void main() {
  vec2 xz = aPos.xz + uOrigin;
  // the grid's spacing here: a wave shorter than a few spacings would alias
  // into stripes that jump as the grid slides, so it fades out (the eye only:
  // the core floats things on every wave)
  float r = max(abs(aPos.x), abs(aPos.z));
  float spacing = uGrid.y * sqrt(r * uGrid.x) + uGrid.y;
  vec3 p = vec3(xz.x, uLevel, xz.y);
  vec3 n = vec3(0.0, 1.0, 0.0);
  float fold = 0.0;
  for (int i = 0; i < WAVES; i++) {
    vec4 a = uWaveA[i];
    vec4 b = uWaveB[i] * clamp((1.0 / (a.z * spacing) - uLod.x) / (uLod.y - uLod.x), 0.0, 1.0);
    float th = uTau * (dot(a.xy, xz) * a.z - a.w);
    float s = sin(th);
    float c = cos(th);
    p.x += b.y * a.x * c;
    p.z += b.y * a.y * c;
    p.y += b.x * s;
    n.x -= a.x * b.z * c;
    n.z -= a.y * b.z * c;
    float q = b.y * uTau * a.z * s;
    n.y -= q;
    fold += q;
  }
  vW = p;
  vN = n;
  vFold = fold;
  vH = p.y - uLevel;
  gl_Position = uVP * vec4(p, 1.0);
}
