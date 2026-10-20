#version 300 es
// The water's grid, displaced by the same Gerstner sum the core floats the
// kayak on (crates/core/src/sea.rs). Each wave's phase arrives as a turn
// fraction the core steps; the shader keeps no time of its own for these.
// WAVES is replaced by the page with the place's wave count from the core,
// SHIPS with the most ship wakes the sea carries (T28), BOILS with the most
// boils a river carries (T38). On a river run the still surface steps down
// the run from a map the core bakes (T38), mixed here as the core mixes it,
// and the chop is scaled by how rough the flow map says the water is.
in vec3 aPos;
uniform mat4 uVP;
uniform vec2 uOrigin;
uniform float uLevel;
uniform float uTau;
uniform float uHalf;
// direction x, direction z, turns a metre (1 / length), phase in turns
uniform vec4 uWaveA[WAVES];
// height, sway, slope (k × height), unused
uniform vec4 uWaveB[WAVES];
// the grid's radius, and its spacing factor (4 / cells)
uniform vec2 uGrid;
// a wave fades out between these many grid spacings to its length
uniform vec2 uLod;
// a ship's bow x and z and its heading's x and z
uniform vec4 uShipA[SHIPS];
// half its beam, its length, its wake's height (0: no ship), foam per metre of heap
uniform vec4 uShipB[SHIPS];
// Kelvin's tangent, the arm's band, the crests' spacing, the length it fades over
uniform vec4 uWakeArm;
// the bow wave's share of the height, its share of the length, its band, its reach ahead
uniform vec4 uWakeBow;
// the still surface's map: millimetres in two bytes, read without filtering
uniform sampler2D uHeightMap;
// its corner (x, z) and size, metres
uniform vec4 uHeightBox;
// metres for the high byte, metres for the low byte, the offset, unused
uniform vec4 uHeightK;
// the ring plane's spiral waves (T50): arms, turns a metre across the
// ring, phase in turns, height; the ice's switch in uIce.x
uniform vec4 uSpiral[SPIRALS];
uniform vec4 uIce;
// the flow map: alpha is how rough the chop is (1 on the sea)
uniform sampler2D uFlowMap;
uniform vec4 uFlowBox;
// a boil's x, z, radius and height (0: none)
uniform vec4 uBoil[BOILS];
out vec3 vW;
out vec3 vN;
out float vFold;
out float vH;
out float vShip;
// the ships' wakes at a point, as sea.rs wake_height lays them; heap is the
// foam they whip up
float wake(vec2 q, out float heap) {
  float h = 0.0;
  heap = 0.0;
  for (int i = 0; i < SHIPS; i++) {
    vec4 a = uShipA[i];
    vec4 b = uShipB[i];
    if (b.z <= 0.0) continue;
    vec2 d = q - a.xy;
    float behind = -dot(d, a.zw);
    float side = abs(d.x * a.w - d.y * a.z);
    if (behind < -uWakeBow.w || behind > uWakeArm.w) continue;
    if (behind > 0.0) {
      float off = side - (b.x + behind * uWakeArm.x);
      if (abs(off) < uWakeArm.y) {
        float r = off / uWakeArm.y;
        float env = (1.0 - r * r) * (1.0 - r * r);
        float w = b.z * env * (1.0 - behind / uWakeArm.w);
        h += w * cos(uTau * off / uWakeArm.z);
        heap += w * b.w;
      }
    }
    float bowLen = b.y * uWakeBow.y;
    if (behind < bowLen && side < b.x + uWakeBow.z) {
      float along = behind < 0.0 ? 1.0 + behind / uWakeBow.w : 1.0 - behind / bowLen;
      float out_ = side >= b.x ? 1.0 - (side - b.x) / uWakeBow.z : (behind < 0.0 ? 1.0 : 0.0);
      float w = b.z * uWakeBow.x * along * out_;
      h += w;
      heap += w * b.w;
    }
  }
  return h;
}
float metres(vec4 t) {
  return t.r * uHeightK.x + t.g * uHeightK.y - uHeightK.z;
}
// the still surface at a point: the four texels round it, mixed
float still(vec2 q) {
  ivec2 last = textureSize(uHeightMap, 0) - ivec2(1) - ivec2(1);
  vec2 u = q - uHeightBox.xy - vec2(uHalf);
  vec2 c = clamp(floor(u), vec2(0.0), vec2(last));
  vec2 f = clamp(u - c, vec2(0.0), vec2(1.0));
  ivec2 i = ivec2(c);
  float a = metres(texelFetch(uHeightMap, i, 0));
  float b = metres(texelFetch(uHeightMap, i + ivec2(1, 0), 0));
  float d = metres(texelFetch(uHeightMap, i + ivec2(0, 1), 0));
  float e = metres(texelFetch(uHeightMap, i + ivec2(1), 0));
  return mix(mix(a, b, f.x), mix(d, e, f.x), f.y);
}
// the boils' domes, as river.rs boil_height lays them
float boils(vec2 q) {
  float h = 0.0;
  for (int i = 0; i < BOILS; i++) {
    vec4 b = uBoil[i];
    float d = length(q - b.xy) / b.z;
    if (d < 1.0 && b.w > 0.0) {
      float k = 1.0 - d * d;
      h += b.w * k * k;
    }
  }
  return h;
}
// the spiral waves winding round the rings, as ringplane.rs swell_at lays them
float spirals(vec2 q) {
  if (uIce.x <= 0.0) return 0.0;
  float r = length(q);
  float b = atan(q.x, q.y) / uTau;
  float h = 0.0;
  for (int i = 0; i < SPIRALS; i++) {
    vec4 s = uSpiral[i];
    h += s.w * sin(uTau * (r * s.y + s.x * b - s.z));
  }
  return h;
}
float ground(vec2 q) {
  return still(q) + boils(q) + spirals(q);
}
void main() {
  vec2 xz = aPos.xz + uOrigin;
  // the grid's spacing here: a wave shorter than a few spacings would alias
  // into stripes that jump as the grid slides, so it fades out (the eye only:
  // the core floats things on every wave)
  float r = max(abs(aPos.x), abs(aPos.z));
  float spacing = uGrid.y * sqrt(r * uGrid.x) + uGrid.y;
  float rough = texture(uFlowMap, (xz - uFlowBox.xy) / uFlowBox.zw).a;
  float base = ground(xz);
  vec3 p = vec3(xz.x, uLevel + base, xz.y);
  vec3 n = vec3(0.0, 1.0, 0.0);
  n.x -= ground(xz + vec2(uHalf, 0.0)) - ground(xz - vec2(uHalf, 0.0));
  n.z -= ground(xz + vec2(0.0, uHalf)) - ground(xz - vec2(0.0, uHalf));
  float fold = 0.0;
  for (int i = 0; i < WAVES; i++) {
    vec4 a = uWaveA[i];
    vec4 b = uWaveB[i] * rough * clamp((1.0 / (a.z * spacing) - uLod.x) / (uLod.y - uLod.x), 0.0, 1.0);
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
  // the wakes, and their slope from their heights half a metre either side
  float heap;
  float unused;
  p.y += wake(xz, heap);
  n.x -= wake(xz + vec2(uHalf, 0.0), unused) - wake(xz - vec2(uHalf, 0.0), unused);
  n.z -= wake(xz + vec2(0.0, uHalf), unused) - wake(xz - vec2(0.0, uHalf), unused);
  vShip = heap;
  vW = p;
  vN = n;
  vFold = fold;
  vH = p.y - uLevel - base;
  gl_Position = uVP * vec4(p, 1.0);
}
