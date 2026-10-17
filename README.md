# Extremely Lifelike Fishing

A 3D fishing game for the desktop browser. You sit in a pedal-drive kayak and fish with one arm whose shoulder and elbow each have their own pair of keys, so a cast takes practice, as in QWOP. The line is a chain of simulated points with weight, drag and stretch; it snaps above its breaking tension. A fish bites only when the hook is inside its mouth zone, and a scope over one eye lets you see it happen. The water moves the kayak with real swell, tide and current.

Play it at `sgilson7.github.io/el-fishing`. There are two places, the Open Swell and the Ledge River, a river that loops round so its current never ends, and two craft: the kayak to fish from and a speedboat to drive.

## How to fish

- Cast: hold F, swing the rod back with W, swing it forward with S, and let go of F as the rod passes upright. A and D bend and straighten the elbow.
- Watch Lure depth: let the lure sink to the depth your fish swims at (the title screen lists each species' depth and the lure speed it likes), then press Space to close the bail and wind. Wind in bursts and pause; a following fish often bites as the lure stops.
- Hold E for the power wind: it pulls line in short bursts and runs on a battery that refills while it rests.
- When a fish bites, wind with Space and sweep the rod up with W together: that is the hook set.
- Keep the line tight. The cue under the message says Wind or Hold; when it says Haul, the fish is tired and close, and the power wind lifts it into your hands.
- On the river, trout hold in the eddies behind the rocks, facing upstream.
- Arrow keys pedal and drift. Hold the right mouse button for the scope: it looks at the mouth of the fish nearest the lure, through the kayak if need be, and the wheel zooms. Esc opens the settings and the fish log.

## The speedboat

Choose it on the title. Up and Down are the throttle, Left and Right steer. It planes as it gathers speed, slides in hard turns and leaves the water off the swell.

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
