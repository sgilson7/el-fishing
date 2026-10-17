#version 300 es
precision highp float;
// The water's look, after the mockups: sky reflected by Fresnel, light
// through thin crests (Sea of Thieves's effect), sun glints, foam on
// folding crests, a wake behind the kayak and rings where the lure lands.
// The fine ripples are the eye-detail layer (L4): they move nothing. The
// surface lets the depths show through, more while the scope is up.
// Every number is a uniform the core sends (Builder D3).
in vec3 vW;
in vec3 vN;
in float vFold;
in float vH;
uniform vec3 uEye;
uniform float uTime;
uniform vec3 uDeep;
uniform vec3 uShallow;
uniform vec3 uCrest;
uniform vec3 uSkyTop;
uniform vec3 uSkyHor;
uniform vec3 uSunDir;
uniform vec3 uSunCol;
uniform vec3 uFogColour;
uniform vec2 uFog;
uniform vec3 uFoamColour;
// sky height power, Fresnel's base reflectance, Fresnel power, crest-light power
uniform vec4 uLook;
// sharp glint power, sharp glint strength, soft glint power, soft glint strength
uniform vec4 uGlint;
// crest colour base, crest colour gain, sunlit base, sunlit gain
uniform vec4 uBody;
// crest light strength, crest light height gain, crest light height base, see-through floor
uniform vec4 uCrestLight;
// fold where foam starts, fold where it is whole, foam strength, foam noise scale
uniform vec4 uFoam;
// foam edge low, foam edge high, foam cover, noise base
uniform vec4 uFoamEdge;
// ripple scale, second ripple scale, ripple drift speed, ripple strength
uniform vec4 uDetail;
// ripple fade distance, current x, current z, current strength
uniform vec4 uFlow;
// hash constants (Hoskins's hash without sine)
uniform vec4 uHash;
// kayak x, kayak z, heading in turns, speed
uniform vec4 uKayak;
// wake arm slope, wake arm sharpness, wake length decay, wake start behind the hull
uniform vec4 uWake;
// wake wave frequency, wake wave speed, wake normal strength, wake foam
uniform vec4 uWakeK;
// lure ring x, z, age in seconds, strength
uniform vec4 uRing;
// ring speed, ring width, ring frequency, ring decay
uniform vec4 uRingK;
// see-through near, see-through with the scope, glare kept through the scope's polariser, unused
uniform vec4 uClear;
uniform float uClarity;
uniform float uTau;
uniform float uHalf;
out vec4 outColor;

float hash(vec2 p) {
  vec3 q = fract(vec3(p.xyx) * uHash.x);
  q += dot(q, q.yzx + uHash.w);
  return fract((q.x + q.y) * q.z);
}
float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (vec2(1.0) + vec2(1.0) + vec2(1.0) - f - f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0)), u.x), u.y);
}
vec3 sky(vec3 d) {
  float t = clamp(d.y, 0.0, 1.0);
  return mix(uSkyHor, uSkyTop, pow(t, uLook.x));
}
void main() {
  vec2 xz = vW.xz;
  float dist = length(uEye - vW);
  // the eye-detail layer: two drifting noise ripples, fading with distance
  float fade = clamp(1.0 - dist / uFlow.x, 0.0, 1.0);
  vec2 drift = uFlow.yz * uFlow.w * uTime;
  vec2 p1 = xz * uDetail.x - drift - vec2(uTime * uDetail.z);
  vec2 p2 = xz * uDetail.y + drift + vec2(uTime * uDetail.z, -uTime * uDetail.z);
  vec2 g = vec2(noise(p1) - noise(p1 + vec2(uHalf, 0.0)), noise(p1) - noise(p1 + vec2(0.0, uHalf)))
         + vec2(noise(p2) - noise(p2 + vec2(uHalf, 0.0)), noise(p2) - noise(p2 + vec2(0.0, uHalf)));
  g *= uDetail.w * fade;
  float foamAdd = 0.0;
  // the wake: a V behind the moving kayak
  if (uKayak.w > 0.0) {
    float a = uKayak.z * uTau;
    vec2 fw = vec2(sin(a), cos(a));
    vec2 rt = vec2(fw.y, -fw.x);
    vec2 rel = xz - uKayak.xy;
    float along = -dot(rel, fw) - uWake.w;
    float side = dot(rel, rt);
    if (along > 0.0) {
      float arm = abs(abs(side) - along * uWake.x);
      float e = exp(-arm * uWake.y) * exp(-along * uWake.z) * uKayak.w;
      g += vec2(sin(along * uWakeK.x - uTime * uWakeK.y)) * e * uWakeK.z;
      foamAdd += e * uWakeK.w * noise(xz * uFoam.w + vec2(uTime));
    }
  }
  // rings where the lure landed
  if (uRing.w > 0.0) {
    vec2 dv = xz - uRing.xy;
    float d = length(dv) + uHash.y;
    float rad = uRing.z * uRingK.x;
    float env = exp(-(d - rad) * (d - rad) * uRingK.y) * exp(-uRing.z * uRingK.w) * uRing.w;
    g += (dv / d) * sin((d - rad) * uRingK.z) * env;
    foamAdd += env * uHash.z;
  }
  vec3 N = normalize(normalize(vN) + vec3(-g.x, 0.0, -g.y));
  vec3 V = normalize(uEye - vW);
  vec3 col;
  float fres = uLook.y + (1.0 - uLook.y) * pow(1.0 - max(dot(N, V), 0.0), uLook.z);
  // the scope's lens is polarised: it cuts the glare off the surface
  fres *= mix(1.0, uClear.z, uClarity);
  if (gl_FrontFacing) {
    vec3 Rf = reflect(-V, N);
    Rf.y = abs(Rf.y);
    vec3 refl = sky(Rf);
    float crest = clamp(vH * uBody.y + uBody.x, 0.0, 1.0);
    vec3 body = mix(uDeep, uShallow, crest);
    float sss = pow(max(dot(V, -uSunDir) * uHalf + uHalf, 0.0), uLook.w) * clamp(vH * uCrestLight.y + uCrestLight.z, 0.0, 1.0);
    body += uCrest * sss * uCrestLight.x;
    body *= uBody.z + uBody.w * max(dot(N, uSunDir), 0.0);
    col = mix(body, refl, clamp(fres, 0.0, 1.0));
    vec3 H = normalize(uSunDir + V);
    float nh = max(dot(N, H), 0.0);
    col += uSunCol * (pow(nh, uGlint.x) * uGlint.y + pow(nh, uGlint.z) * uGlint.w) * mix(1.0, uClear.z, uClarity);
    float foam = smoothstep(uFoam.x, uFoam.y, vFold) * uFoam.z + foamAdd;
    foam = clamp(foam * (uFoamEdge.w + noise(xz * uFoam.w + vec2(uTime * uHalf, 0.0))), 0.0, 1.0);
    col = mix(col, uFoamColour, smoothstep(uFoamEdge.x, uFoamEdge.y, foam) * uFoamEdge.z);
  } else {
    col = mix(uDeep, uSkyHor, uHalf);
  }
  float f = smoothstep(uFog.x, uFog.y, dist);
  col = mix(col, uFogColour, f);
  // see-through: the depths show where the surface faces the eye
  float clear = mix(uClear.x, uClear.y, uClarity);
  float alpha = clamp(1.0 - clear * (1.0 - fres), uCrestLight.w, 1.0);
  alpha = mix(alpha, 1.0, f);
  outColor = vec4(col, alpha);
}
