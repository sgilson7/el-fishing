# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

*Extremely Lifelike Fishing*: a 3D browser fishing game. The angler sits in a pedal-drive kayak on a simulated sea and drives two arms joint by joint to cast, work a lure, set the hook and fight a fish on a simulated line. Round 2 (17 October 2026) built the MVP: one place, the Open Swell, where a fish can be caught. The design journal is `../journal/fishing-journal.html`; the paper is in `../paper/`.

## Rules that hold it up

- The sea (`sea.rs`): tide, eight Gerstner waves and one current, in Q16 fixed point. Each wave's phase is a wrapping 32-bit binary angle; the water shader draws the same waves from the phases the core sends.
- The kayak (`hull.rs`): 16 buoyancy probes; pedals, drift about the bow, drag against the moving water, the waves' slope, wind and the line's pull. Horizontal motion keeps 32 fraction bits (R2.3).
- The arms (`arms.rs`): four motor-driven joints; the rod runs from the left hand through the right.
- The line (`line.rs`): XPBD with long-range ties to the tip and the line's stretch lumped on the end (R2.6). Tension is read from the end's tie. The bail, the drag and the break are rules here.
- The fish (`fish.rs`): cruise, notice, follow, strike, hold, spit; a bite only with the hook inside the mouth zone; a hook set is a tension rise of 4 N within a quarter second while a fish holds the lure.
- `world.rs` steps them together; `session.rs` holds what is only seen (camera, scope, HUD), so the scope cannot change a rule (tested).

## Shape of the code

`crates/core/src/`: the modules above plus `mesh.rs` (faceted meshes built in the core), `scene.rs` (instances), `render.rs` (draw commands), `look.rs` (shader constants from `data/look.json`), `strings.rs`. `crates/shim`: the `Game` class, Q16 to floats. `web/`: `app.js`, `gl.js` (three programs: sky, shapes, water), `keys.js`, `ui.js`, `style.css`. `data/`: `gear.json`, `place.json`, `look.json`, `controls.json`, `strings.json`, `shaders/`.

## Commands

`cargo test --workspace`; `./scripts/build.sh`; `python3 scripts/serve.py` (or any static server on `dist/`); `./scripts/check.sh`; `python3 scripts/break_tests.py`; `cargo test --test angler -- --ignored --nocapture` prints how long the scripted angler takes to land a fish on eight seeds.

## What will bite within the hour

- `scripts/break_tests.py` edits core files while it runs; do not edit them at the same time.
- The page holds no numbers: a new shader constant goes in `data/look.json` and arrives as a uniform.
- `rustfmt` rewraps long lines, so a scripted edit made after formatting must match the formatted text.

## Open for the person

Round 2 closed R1.1, R1.2, R1.6, R1.8, R1.14, R1.17, R1.18 and R1.21: the person took the agent's recommendation on each (`PLAN.md`, Answers).

- R1.5 the glossary's terms and their `Not:` lines, now 20 with Bail and Mouth zone.
- R1.9 place names, species and the mockups' strings.
- R1.15 an anonymised Builder copy before the paper is submitted.
- R1.19 the far waters, exotic fish, creatures and costumes (not built in this round).
- R2.10 the drag is 10 N, not the plan's 7.1 N.
- R2.14 play the Open Swell and say how catching a fish feels; the scripted angler is not a person.
- R2.15 the round-2 strings in `data/strings.json`.

## Mistakes so far

Round 1: a bibliography field from memory (R1.11); labels projected with a stale camera (R1.12). Round 2: a line solver that rang (R2.6), an overflow that sank every fish (R2.7), a drag that slipped on the wrong quantity (R2.8), strikes that could never bite (R2.9), a scope camera inside its own tube (R2.13).

## What passed

To be filled in at the end of the round with `./scripts/check.sh`'s result and the live browser check.

## The next action

Publish to `el-fishing`, check the live address, then add the game to the website with the `el-fishing` chip.
