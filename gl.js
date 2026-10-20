// The WebGL 2 renderer. It uploads what the core returns and draws it, in
// the order the core's commands give: the sky, the shapes, the water, the
// glow. Every size, stride, offset, uniform name and setter comes from the
// core's numbers(), so this file holds no layout of its own. After Mage
// Tower Battle's gl.js.

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// Returns null when the browser has no WebGL 2; the page then says so.
export function makeRenderer(canvas, N, shaders, game) {
  const gl = canvas.getContext("webgl2", { antialias: true, powerPreference: "high-performance" });
  if (!gl) return null;
  const waves = String(N.waves);
  const ships = String(N.ships);
  const boils = String(N.boils);
  const spirals = String(N.spirals);
  const src = (s) =>
    s.replaceAll("WAVES", waves).replaceAll("SHIPS", ships).replaceAll("BOILS", boils).replaceAll("SPIRALS", spirals);
  const P = N.programs;
  const programs = {};
  // the core's view is a mirror of the usual one (geom.rs look_at), so its
  // front faces wind clockwise on screen
  gl.frontFace(gl.CW);
  programs[P.sky] = link(gl, src(shaders.skyVert), src(shaders.skyFrag));
  programs[P.shapes] = link(gl, src(shaders.shapesVert), src(shaders.shapesFrag));
  programs[P.water] = link(gl, src(shaders.waterVert), src(shaders.waterFrag));

  const locations = new Map();
  const where = (prog, name) => {
    const key = prog + ":" + name;
    if (!locations.has(key)) locations.set(key, gl.getUniformLocation(programs[prog], name));
    return locations.get(key);
  };
  const set = (prog, name, setter, values) => {
    const loc = where(prog, name);
    if (loc === null) return;
    if (setter === N.matrix) gl[setter](loc, false, values);
    else gl[setter](loc, values);
  };
  // the constants and the flow map: again whenever the core's generation
  // changes (another place or boat); the core says when, not the page
  const flowTexture = gl.createTexture();
  const heightTexture = gl.createTexture();
  let generation = null;
  function setup() {
    const constants = game.constants();
    let at = 0;
    for (const [prog, name, size, setter] of N.constants) {
      gl.useProgram(programs[prog]);
      set(prog, name, setter, constants.subarray(at, at + size));
      at += size;
    }
    const [w, h] = game.flow_size();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, flowTexture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, game.flow_map());
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    // the still surface's map on the second unit, read texel by texel
    const [hw, hh] = game.height_size();
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, heightTexture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, hw, hh, 0, gl.RGBA, gl.UNSIGNED_BYTE, game.height_map());
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    for (const prog of [P.water, P.shapes]) {
      gl.useProgram(programs[prog]);
      gl.uniform1i(where(prog, "uHeightMap"), 1);
    }
    gl.activeTexture(gl.TEXTURE0);
    generation = game.generation();
  }

  // the shapes' meshes, each with its own vertex array
  const shapes = programs[P.shapes];
  const instanceBuffer = gl.createBuffer();
  const attribute = (prog, [name, size, offset]) => ({ loc: gl.getAttribLocation(prog, name), size, offset });
  const vertexAttributes = N.vertex.attributes.map((a) => attribute(shapes, a));
  const instanceAttributes = N.instance.attributes.map((a) => attribute(shapes, a));
  const meshes = N.meshes.map((count, id) => {
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, game.mesh(id), gl.STATIC_DRAW);
    for (const v of vertexAttributes) {
      gl.enableVertexAttribArray(v.loc);
      gl.vertexAttribPointer(v.loc, v.size, gl.FLOAT, false, N.vertex.stride, v.offset);
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffer);
    for (const v of instanceAttributes) {
      gl.enableVertexAttribArray(v.loc);
      gl.vertexAttribDivisor(v.loc, 1);
    }
    return { vao, count };
  });
  // the water's grid and the sky's triangle: positions only
  const plain = (prog, data) => {
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
    for (const a of N.plain.attributes) {
      const v = attribute(prog, a);
      gl.enableVertexAttribArray(v.loc);
      gl.vertexAttribPointer(v.loc, v.size, gl.FLOAT, false, N.plain.stride, v.offset);
    }
    return vao;
  };
  const waterVao = plain(programs[P.water], game.water_mesh());
  const skyVao = plain(programs[P.sky], game.sky_mesh());
  const uPass = gl.getUniformLocation(shapes, "uPass");

  function draws(commands) {
    const C = N.command_values;
    for (let i = 0; i < commands.length; i += C) {
      const [prog, mesh, offset, count, pass] = commands.subarray(i, i + C);
      gl.useProgram(programs[prog]);
      if (prog === P.sky) {
        gl.disable(gl.DEPTH_TEST);
        gl.disable(gl.BLEND);
        gl.depthMask(false);
        gl.bindVertexArray(skyVao);
        gl.drawArrays(gl.TRIANGLES, 0, N.sky_vertices);
        continue;
      }
      gl.enable(gl.DEPTH_TEST);
      if (prog === P.water) {
        // see-through, after the shapes, leaving their depth for the glow
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
        gl.depthMask(false);
        gl.disable(gl.CULL_FACE);
        gl.bindVertexArray(waterVao);
        gl.drawArrays(gl.TRIANGLES, 0, N.water_vertices);
        continue;
      }
      if (pass === N.passes.glow) {
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.ONE, gl.ONE);
        gl.depthMask(false);
      } else {
        gl.disable(gl.BLEND);
        gl.depthMask(true);
      }
      gl.disable(gl.CULL_FACE);
      gl.uniform1f(uPass, pass);
      const m = meshes[mesh];
      gl.bindVertexArray(m.vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffer);
      for (const v of instanceAttributes) {
        gl.vertexAttribPointer(v.loc, v.size, gl.FLOAT, false, N.instance.stride, v.offset + offset);
      }
      gl.drawArraysInstanced(gl.TRIANGLES, 0, m.count, count);
    }
    gl.depthMask(true);
  }

  return {
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    },
    draw() {
      if (game.generation() !== generation) setup();
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, game.values(), gl.STREAM_DRAW);
      const u = game.uniforms();
      let at = 0;
      for (const [prog, name, size, setter] of N.frame) {
        gl.useProgram(programs[prog]);
        set(prog, name, setter, u.subarray(at, at + size));
        at += size;
      }
      gl.depthMask(true);
      gl.clearColor(0, 0, 0, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      draws(game.commands());
    },
  };
}
