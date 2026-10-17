#version 300 es
precision highp float;
// The sky: horizon to zenith by a power of the ray's height, the haze's
// colour below the horizon, a glow round the sun and its disc.
in vec3 vRay;
uniform vec3 uTop;
uniform vec3 uHor;
uniform vec3 uBelow;
uniform vec3 uSunDir;
uniform vec3 uSunCol;
// height power, glow power, glow strength, disc strength
uniform vec4 uSkyK;
// the disc's power: how tight round the sun it falls off
uniform float uDisc;
out vec4 outColor;
void main() {
  vec3 d = normalize(vRay);
  vec3 c = d.y > 0.0 ? mix(uHor, uTop, pow(d.y, uSkyK.x)) : uBelow;
  float g = max(dot(d, uSunDir), 0.0);
  c += uSunCol * (pow(g, uSkyK.y) * uSkyK.z + pow(g, uDisc) * uSkyK.w);
  outColor = vec4(c, 1.0);
}
