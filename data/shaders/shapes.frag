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
out vec4 outColor;
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
  vec3 lit = base * (ambient * uMix.x + uSun * (max(ndl, 0.0) * uMix.y + wrapped * uMix.z));
  lit = mix(lit, base * uGlow.y, min(vColor.a, 1.0));
  float depth = max(uLevel - vWorld.y, 0.0);
  float reach = uMurk.x * mix(1.0, uMurk.y, uClarity);
  lit = mix(lit, uDeep, 1.0 - exp(-depth / reach));
  outColor = vec4(mix(lit, uFogColour, f), 1.0);
}
