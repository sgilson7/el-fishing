# Extremely Lifelike Fishing

A 3D fishing game for the desktop browser. You sit in a pedal-drive kayak on the open sea and fish with your own two arms: each shoulder and each elbow has its own pair of keys, so a cast takes practice, as in QWOP. The line is a chain of simulated points with weight, drag and stretch; it snaps above its breaking tension. A fish bites only when the hook is inside its mouth zone, and a scope over one eye lets you see it happen. The sea moves the kayak with real swell, tide and current.

Play it at `sgilson7.github.io/el-fishing`. This first version has one place, the Open Swell.

## How to fish

- Cast: hold F, swing the rod back with I, swing it forward with K, and let go of F as the rod passes upright.
- Watch Lure depth: let the lure sink to the depth your fish swims at (the title screen lists each species' depth and the lure speed it likes), then press Space to close the bail and wind. Wind in bursts and pause; a following fish often bites as the lure stops.
- Hold E for the electric power wind: it brings the lure back fast while no fish has it, and a mahi-mahi may chase it.
- When a fish bites, sweep the rod up fast (I, with Q for the other arm): that is the hook set.
- Keep the line tight. Wind while the fish rests; the drag gives line when it runs.
- Arrow keys pedal and drift. Hold the right mouse button for the scope: it looks at the mouth of the fish nearest the lure, and the wheel zooms. Esc opens the settings, where slower arms and steady hands make casting easier.

## How it is built

It is a Builder project: every rule lives in a Rust core in fixed-point arithmetic, compiled to WebAssembly, with a thin shim and a static page that draws what the core describes. The design, the research behind it and the water model are in the design journal kept beside the project.

## Build, test and serve

```bash
./scripts/setup.sh              # once: Python venv with Playwright and three browsers
cargo test --workspace          # the core, boundary, copy and storage tests, in seconds
./scripts/build.sh              # release WebAssembly build into dist/
python3 scripts/serve.py        # http://127.0.0.1:8000/
./scripts/check.sh              # everything, before publishing
python3 scripts/break_tests.py  # each row of tests/breaks.json must make its test fail
./scripts/count-tests.sh        # the test count to quote in a document
./scripts/publish.sh            # stage 8: push dist/ to gh-pages, from a clean tree
```

Needs Rust (pinned in `rust-toolchain.toml`), `wasm-bindgen-cli` 0.2.127, and Python 3. Rendering art needs `pdflatex` and `pdftocairo`; the check does not.

## Layout

| Path | Holds |
| --- | --- |
| `crates/core` | every rule: `sea`, `hull`, `arms`, `line`, `fish`, `world`; and what is only seen: `session`, `scene`, `mesh`, `render`, `look` |
| `crates/shim` | the `wasm-bindgen` functions the page calls; each is one call into the core |
| `web/` | the static page: a WebGL renderer for the core's draw list and a generic HTML view model |
| `data/` | reader-visible text, presentation numbers, shaders |
| `GLOSSARY.md`, `TONE.md` | what each term means; how every string is written |
| `tools/art/`, `styles/default/` | the art pipeline and its fallback style pack |
| `tests/browser/` | the browser check and the game's interactions |
| `tests/breaks.json` | one deliberate break per rule, for `scripts/break_tests.py` |
| `scripts/` | build, check, serve, publish, import audit, mutation audit, test count |
