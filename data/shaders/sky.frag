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
// the moons (T44): toward each, its colour; their radii (radians), the
// edge's softness, and the light on their dark sides
uniform vec3 uMoon1Dir;
uniform vec3 uMoon2Dir;
uniform vec3 uMoon1Col;
uniform vec3 uMoon2Col;
uniform vec4 uMoonK;
// space (T50): the stars' brightness (0: none), cells a radian, the share
// of cells holding a star, the Milky Way's brightness
uniform vec4 uStars;
// the Milky Way's pole, its colour, and how narrow it is, how fast its
// brightness swells along it, and two keys for the stars' brightness and tint
uniform vec3 uGalaxy;
uniform vec3 uGalaxyCol;
uniform vec4 uGalaxyK;
// hash constants (Hoskins's hash without sine)
uniform vec4 uHash;
out vec4 outColor;
float hash3(vec3 p) {
  p = fract(p * uHash.x);
  p += dot(p, p.yzx + uHash.w);
  return fract((p.x + p.y) * p.z);
}
// a moon over the sky colour c: a disc of radius r round m, lit as a ball by the sun
vec3 moon(vec3 c, vec3 d, vec3 m, float r, vec3 tint) {
  float a = acos(clamp(dot(d, m), -1.0, 1.0));
  float cover = clamp((r - a) / uMoonK.z, 0.0, 1.0);
  vec3 o = (d - m * dot(d, m)) / max(r, uMoonK.z);
  vec3 n = o - m * sqrt(max(1.0 - dot(o, o), 0.0));
  float lit = uMoonK.w + (1.0 - uMoonK.w) * max(dot(n, uSunDir), 0.0);
  return mix(c, tint * lit, cover);
}
void main() {
  vec3 d = normalize(vRay);
  vec3 c = d.y > 0.0 ? mix(uHor, uTop, pow(d.y, uSkyK.x)) : uBelow;
  float g = max(dot(d, uSunDir), 0.0);
  c += uSunCol * (pow(g, uSkyK.y) * uSkyK.z + pow(g, uDisc) * uSkyK.w);
  c = moon(c, d, uMoon1Dir, uMoonK.x, uMoon1Col);
  c = moon(c, d, uMoon2Dir, uMoonK.y, uMoon2Col);
  if (uStars.x > 0.0) {
    // a star in a share of the cells, each its own brightness and tint
    vec3 cell = floor(d * uStars.y);
    float h = hash3(cell);
    float star = step(1.0 - uStars.z, h) * hash3(cell + uGalaxyK.zzz);
    vec3 tint = mix(vec3(1.0), uGalaxyCol, hash3(cell + uGalaxyK.www));
    // the Milky Way: a band round the sky, patchy, with more stars in it
    float band = exp(-dot(d, uGalaxy) * dot(d, uGalaxy) * uGalaxyK.x);
    float swell = sin(dot(d, uGalaxy.yzx) * uGalaxyK.y) * sin(dot(d, uGalaxy.zxy) * uGalaxyK.y);
    c += uGalaxyCol * band * (1.0 + swell) * uStars.w;
    c += tint * star * uStars.x * (1.0 + band);
  }
  outColor = vec4(c, 1.0);
}
