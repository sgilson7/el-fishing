// The page: loads the module and the data, then each display frame passes
// the keys to the core, draws what it returns with WebGL and redraws the
// HTML it describes. It decides nothing (Builder R6).
import init, { Game } from "./pkg/tool_shim.js?v=67225dcc5244";
import { makeRenderer } from "./gl.js?v=67225dcc5244";
import { makeKeys } from "./keys.js?v=67225dcc5244";
import { makeUi } from "./ui.js?v=67225dcc5244";

const el = (id) => document.getElementById(id);
const text = async (path) => (await fetch(path)).text();

async function main() {
  await init();
  const files = [
    "strings.json",
    "gear.json",
    "places.json",
    "races.json",
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
  const [strings, gear, places, races, look, glossary, controls, skyVert, skyFrag, shapesVert, shapesFrag, waterVert, waterFrag] =
    // ?v=67225dcc5244 becomes "?v=<build>" when scripts/build.sh assembles dist/, so a
    // new build's data is never served from an old cache beside its module
    await Promise.all(files.map((f) => text(`data/${f}?v=67225dcc5244`)));
  // the page draws a seed; the core keeps it (rng.rs)
  const seed = new Uint32Array(1);
  crypto.getRandomValues(seed);
  const game = new Game(strings, gear, places, races, look, glossary, seed[0]);
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
  // a drag on the game: the pixels moved go to the core as they are, and the
  // core turns the camera (T39)
  let drag = null;
  canvas.addEventListener("pointerdown", (e) => {
    if (!(e.buttons & C.drag.buttons)) return;
    drag = [e.clientX, e.clientY];
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag || !(e.buttons & C.drag.buttons)) {
      drag = null;
      return;
    }
    act(C.drag.act, `${e.clientX - drag[0]} ${e.clientY - drag[1]}`);
    drag = [e.clientX, e.clientY];
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
