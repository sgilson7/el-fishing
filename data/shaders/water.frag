#version 300 es
precision highp float;
// The water's look, after the mockups: sky reflected by Fresnel, light
// through thin crests (Sea of Thieves's effect), sun glints, foam on
// folding crests, a wake behind the kayak and rings where the lure lands.
// The fine ripples are the eye-detail layer (L4): they move nothing. Over
// Jupiter (T44) the same surface is a cloud deck: belts and zones bent by
// noise and twisted round a storm, after the journal's mockup. The
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
// ripple fade distance, unused, unused, unused
uniform vec4 uFlow;
// the current, as the core samples it on a grid (the flow map): red and
// green the current along x and z, blue the white water round rocks
uniform sampler2D uFlowMap;
// the flow map's corner (x, z) and its size, metres
uniform vec4 uFlowBox;
// the current the map's full scale stands for (m/s), streak strength, white water strength, ripple carry period (s)
uniform vec4 uFlowK;
// streak scale along the current, across it, noise where streaks start, noise where they are whole
uniform vec4 uStreak;
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
// the foam the ships' wakes whip up (water.vert)
in float vShip;
uniform float uTau;
uniform float uHalf;
// the cloud deck (T44): on (1) or off (0), metres a belt and a zone, noise
// scale in metres, drift in metres a second
uniform vec4 uCloud;
uniform vec3 uBeltDark;
uniform vec3 uBeltLight;
uniform vec3 uZone;
uniform vec3 uStorm;
// the storm: its x and z, its radius, and how far it twists the bands at its middle (turns)
uniform vec4 uVortex;
// how far noise bends the bands, the fine noise's share, how tight the storm's colour, unlit light
uniform vec4 uCloudK;
// the ring plane's ice (T50): on (1) or off (0), sparkle's share of the
// grains, the ringlets' strength, the light left in Saturn's shadow
uniform vec4 uIce;
// sparkle grains a metre, ringlets a metre, the shadow's soft edge in
// metres, the grain's strength
uniform vec4 uIceK;
// the C ring's, the B ring's and the A ring's colours
uniform vec3 uIceC;
uniform vec3 uIceB;
uniform vec3 uIceA;
// Saturn's middle and radius, metres (radius 0: no planet)
uniform vec4 uPlanet;
// the still surface's map, whose blue holds the ring a point is on
uniform highp sampler2D uHeightMap;
uniform vec4 uHeightBox;
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
// the eye-detail ripples' slope at a point: two noises at two scales
vec2 ripple(vec2 xz) {
  vec2 p1 = xz * uDetail.x - vec2(uTime * uDetail.z);
  vec2 p2 = xz * uDetail.y + vec2(uTime * uDetail.z, -uTime * uDetail.z);
  return vec2(noise(p1) - noise(p1 + vec2(uHalf, 0.0)), noise(p1) - noise(p1 + vec2(0.0, uHalf)))
       + vec2(noise(p2) - noise(p2 + vec2(uHalf, 0.0)), noise(p2) - noise(p2 + vec2(0.0, uHalf)));
}
// the cloud deck's colour at a point (T44)
vec3 deck(vec2 xz, vec3 N) {
  vec2 dv = xz - uVortex.xy;
  float r = length(dv) / uVortex.z;
  float storm = exp(-r * r);
  float ang = uVortex.w * uTau * storm;
  vec2 p = uVortex.xy + mat2(cos(ang), -sin(ang), sin(ang), cos(ang)) * dv;
  vec2 drift = vec2(uTime * uCloud.w, 0.0);
  vec2 q = (p - drift) / uCloud.z;
  float lat = p.y / uCloud.y + noise(q) * uCloudK.x + noise(q * uCloudK.y + vec2(uTau)) * uCloudK.x * uHalf;
  float band = sin(lat * uTau * uHalf);
  // fine streaks drawn out along the jets, east and west
  float streak = noise(vec2(q.x, q.y * uCloudK.y) * uCloudK.y);
  vec3 belt = mix(uBeltDark, uBeltLight, noise(q * uCloudK.y) * uHalf + streak * uHalf);
  vec3 c = mix(belt, uZone, clamp(band + uHalf, 0.0, 1.0));
  c = mix(c, uStorm, exp(-r * r * uCloudK.z));
  return c * (uCloudK.w + (1.0 - uCloudK.w) * max(dot(N, uSunDir), 0.0));
}
// how much ice there is at a point (0 over a gap, 1 on a ring), from the
// four texels round it, and the ring's code from the nearest
vec2 ice(vec2 q) {
  ivec2 last = textureSize(uHeightMap, 0) - ivec2(1) - ivec2(1);
  vec2 u = q - uHeightBox.xy - vec2(uHalf);
  vec2 c = clamp(floor(u), vec2(0.0), vec2(last));
  vec2 f = clamp(u - c, vec2(0.0), vec2(1.0));
  ivec2 i = ivec2(c);
  vec4 a = texelFetch(uHeightMap, i, 0);
  vec4 b = texelFetch(uHeightMap, i + ivec2(1, 0), 0);
  vec4 d = texelFetch(uHeightMap, i + ivec2(0, 1), 0);
  vec4 e = texelFetch(uHeightMap, i + ivec2(1), 0);
  float on = mix(mix(float(a.b > 0.0), float(b.b > 0.0), f.x), mix(float(d.b > 0.0), float(e.b > 0.0), f.x), f.y);
  float code = max(max(a.b, b.b), max(d.b, e.b));
  return vec2(on, code);
}
// how far into Saturn's shadow a point is: 1 where the planet hides the sun
float shadowed(vec3 p) {
  if (uPlanet.w <= 0.0) return 0.0;
  vec3 L = uPlanet.xyz - p;
  float along = dot(L, uSunDir);
  if (along <= 0.0) return 0.0;
  float d = sqrt(max(dot(L, L) - along * along, 0.0));
  return 1.0 - smoothstep(uPlanet.w - uIceK.z, uPlanet.w + uIceK.z, d);
}
vec3 sky(vec3 d) {
  float t = clamp(d.y, 0.0, 1.0);
  return mix(uSkyHor, uSkyTop, pow(t, uLook.x));
}
void main() {
  vec2 xz = vW.xz;
  float dist = length(uEye - vW);
  // Saturn's rings (T50): ice in bands, nothing over the gaps, lit by a
  // low sun except in the planet's shadow, sparkling where a grain catches it
  if (uIce.x > 0.0) {
    vec2 at = ice(xz);
    if (at.x < uHalf) discard;
    float w = at.y * (1.0 + 1.0 + 1.0);
    vec3 tint = mix(mix(uIceC, uIceB, clamp(w - 1.0, 0.0, 1.0)), uIceA, clamp(w - 1.0 - 1.0, 0.0, 1.0));
    float r = length(xz);
    float lets = noise(vec2(r * uIceK.y, 0.0)) + noise(vec2(r * uIceK.y * uIceK.y, 1.0)) * uHalf;
    tint *= 1.0 - uIce.z + uIce.z * lets;
    tint *= 1.0 - uIceK.w + uIceK.w * noise(xz * uIceK.x);
    vec3 Ni = normalize(normalize(vN) + vec3(-ripple(xz).x, 0.0, -ripple(xz).y) * uDetail.w);
    vec3 Vi = normalize(uEye - vW);
    float sh = shadowed(vW);
    float sun = max(dot(Ni, uSunDir), 0.0) * (1.0 - sh);
    vec3 icecol = tint * (uIce.w + uBody.z * (1.0 - sh) + uBody.w * sun);
    float nh = max(dot(Ni, normalize(uSunDir + Vi)), 0.0);
    float grain = step(1.0 - uIce.y, hash(floor(xz * uIceK.x)));
    icecol += uSunCol * grain * pow(nh, uGlint.z) * uGlint.y * (1.0 - sh);
    outColor = vec4(mix(icecol, uFogColour, smoothstep(uFog.x, uFog.y, dist)), 1.0);
    return;
  }
  // the eye-detail layer: ripples carried by the current in two layers,
  // each faded out as it resets (Vlachos's flow maps), fading with distance
  float fade = clamp(1.0 - dist / uFlow.x, 0.0, 1.0);
  vec4 fm = texture(uFlowMap, (xz - uFlowBox.xy) / uFlowBox.zw);
  vec2 flow = (fm.rg - vec2(uHalf)) * (uFlowK.x + uFlowK.x);
  float speed = length(flow);
  float ph = uTime / uFlowK.w;
  float t1 = fract(ph);
  float t2 = fract(ph + uHalf);
  float w1 = 1.0 - abs(t1 - uHalf) / uHalf;
  vec2 q1 = xz - flow * t1 * uFlowK.w;
  vec2 q2 = xz - flow * t2 * uFlowK.w;
  vec2 g = (ripple(q1) * w1 + ripple(q2) * (1.0 - w1)) * uDetail.w * fade;
  // streaks that run with the current, and white water round the rocks
  vec2 along = flow / (speed + uHash.y);
  vec2 across = vec2(-along.y, along.x);
  float s1 = noise(vec2(dot(q1, along) * uStreak.x, dot(q1, across) * uStreak.y));
  float s2 = noise(vec2(dot(q2, along) * uStreak.x, dot(q2, across) * uStreak.y));
  float streak = smoothstep(uStreak.z, uStreak.w, s1 * w1 + s2 * (1.0 - w1));
  float foamAdd = streak * clamp(speed / uFlowK.x, 0.0, 1.0) * uFlowK.y * fade;
  float churn = noise(q1 * uFoam.w) * w1 + noise(q2 * uFoam.w) * (1.0 - w1);
  foamAdd += fm.b * churn * uFlowK.z;
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
  // the ships' wakes' foam
  foamAdd += vShip * noise(xz * uFoam.w + vec2(uTime));
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
    if (uCloud.x > 0.0) {
      body = deck(xz, N) + uCrest * sss * uCrestLight.x;
    }
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
