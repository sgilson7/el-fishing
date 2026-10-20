#version 300 es
precision highp float;
// Ape Race's light: the sky's colour from above and the ground's from
// below, a sun whose light wraps part way round each faceted shape, and
// haze in the place's colour. Under the surface the water's own colour
// closes in with depth, less while the scope clears the view. Every
// number is a uniform the core sends (Builder D3).
in vec3 vNormal;
in vec3 vWorld;
in vec4 vColor;
in vec3 vTint;
uniform float uPass;
uniform vec3 uEye;
uniform vec3 uFogColour;
uniform vec2 uFog;
uniform vec3 uSkyLight;
uniform vec3 uGroundLight;
uniform vec3 uSun;
uniform vec3 uSunDir;
uniform vec4 uMix;
uniform vec2 uGlow;
uniform float uLevel;
uniform vec3 uDeep;
uniform vec2 uMurk;
uniform float uClarity;
// a river run's still surface (T38), as water.vert reads it: under it is
// under the water, wherever down the run
uniform sampler2D uHeightMap;
uniform vec4 uHeightBox;
uniform vec4 uHeightK;
uniform float uHalf;
// Saturn's middle and radius (T50; radius 0: no planet), and its shadow's
// soft edge in metres and the share of the sun left in it
uniform vec4 uPlanet;
uniform vec2 uShadow;
out vec4 outColor;
float shadowed(vec3 p) {
  if (uPlanet.w <= 0.0) return 0.0;
  vec3 L = uPlanet.xyz - p;
  float along = dot(L, uSunDir);
  if (along <= 0.0) return 0.0;
  float d = sqrt(max(dot(L, L) - along * along, 0.0));
  return (1.0 - smoothstep(uPlanet.w - uShadow.x, uPlanet.w + uShadow.x, d)) * (1.0 - uShadow.y);
}
float metres(vec4 t) {
  return t.r * uHeightK.x + t.g * uHeightK.y - uHeightK.z;
}
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
void main() {
  vec3 n = normalize(vNormal);
  if (!gl_FrontFacing) n = -n;
  float f = smoothstep(uFog.x, uFog.y, length(vWorld - uEye));
  vec3 base = vColor.rgb * vTint;
  if (uPass > 0.0) {
    float facing = abs(dot(n, normalize(uEye - vWorld)));
    outColor = vec4(base * vColor.a * facing * facing * uGlow.x * (1.0 - f), 1.0);
    return;
  }
  float ndl = dot(n, uSunDir);
  float wrapped = clamp(ndl * uMix.w + uMix.w, 0.0, 1.0);
  vec3 ambient = mix(uGroundLight, uSkyLight, n.y * uMix.w + uMix.w);
  float sunlit = 1.0 - shadowed(vWorld);
  vec3 lit = base * (ambient * uMix.x + uSun * sunlit * (max(ndl, 0.0) * uMix.y + wrapped * uMix.z));
  lit = mix(lit, base * uGlow.y, min(vColor.a, 1.0));
  float depth = max(uLevel + still(vWorld.xz) - vWorld.y, 0.0);
  float reach = uMurk.x * mix(1.0, uMurk.y, uClarity);
  lit = mix(lit, uDeep, 1.0 - exp(-depth / reach));
  outColor = vec4(mix(lit, uFogColour, f), 1.0);
}
