// The page: loads the module and the data, then each display frame passes
// the keys to the core, draws what it returns with WebGL and redraws the
// HTML it describes. It decides nothing (Builder R6).
import init, { Game } from "./pkg/tool_shim.js";
import { makeRenderer } from "./gl.js";
import { makeKeys } from "./keys.js";
import { makeUi } from "./ui.js";

const el = (id) => document.getElementById(id);
const text = async (path) => (await fetch(path)).text();

async function main() {
  await init();
  const files = [
    "strings.json",
    "gear.json",
    "places.json",
    "look.json",
    "glossary.md",
    "controls.json",
    "shaders/sky.vert",
    "shaders/sky.frag",
    "shaders/shapes.vert",
    "shaders/shapes.frag",
    "shaders/water.vert",
    "shaders/water.frag",
  ];
  const [strings, gear, places, look, glossary, controls, skyVert, skyFrag, shapesVert, shapesFrag, waterVert, waterFrag] =
    // BUILD_STAMP becomes "?v=<build>" when scripts/build.sh assembles dist/, so a
    // new build's data is never served from an old cache beside its module
    await Promise.all(files.map((f) => text(`data/${f}BUILD_STAMP`)));
  // the page draws a seed; the core keeps it (rng.rs)
  const seed = new Uint32Array(1);
  crypto.getRandomValues(seed);
  const game = new Game(strings, gear, places, look, glossary, seed[0]);
  const N = JSON.parse(game.numbers());
  const C = JSON.parse(controls);
  const canvas = el("gl");
  const renderer = makeRenderer(canvas, N, { skyVert, skyFrag, shapesVert, shapesFrag, waterVert, waterFrag }, game);
  if (!renderer) el("error").textContent = JSON.parse(strings).no_webgl;
  const keys = makeKeys(C, N.keys, canvas);
  const act = (name, value) => {
    el("error").textContent = game.act(name, value);
  };
  const ui = makeUi(el, act);

  window.addEventListener("keydown", (e) => {
    const a = C.acts[e.code];
    if (a && !e.repeat) act(a[0], a[1]);
  });
  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      act("scope.zoom", e.deltaY < 0 ? "+" : "-");
    },
    { passive: false },
  );

  function resize() {
    const stage = el("stage");
    const [w, h] = game.resize(stage.clientWidth, stage.clientHeight, window.devicePixelRatio || 1);
    if (renderer) renderer.resize(w, h);
  }
  resize();
  window.addEventListener("resize", resize);

  let last = "";
  function frame(now) {
    game.frame(now, keys.read());
    if (renderer) renderer.draw();
    const u = game.ui();
    if (u !== last) {
      last = u;
      ui.draw(JSON.parse(u));
    }
    document.body.dataset.checksum = game.checksum();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  document.body.dataset.ready = "true";
}

main();
