# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

*Extremely Lifelike Fishing*: a 3D browser fishing game. The angler sits in a pedal-drive kayak on simulated water and drives one arm joint by joint to cast, work a lure, set the hook and fight a fish on a simulated line. Round 2 built the MVP at the Open Swell; round 3 fixed the water and the fight (T2 to T7); round 4 (17 October 2026) added the Ledge River, a speedboat, a fish log, one casting arm, the fight cue and a battery power wind (T8 to T17). The design journal is `../journal/fishing-journal.html`; the paper is in `../paper/`.

## Rules that hold it up

- The sea (`sea.rs`): tide, eight Gerstner waves and a current, in Q16 fixed point. Each wave's phase is a wrapping 32-bit binary angle; the water shader draws the same waves from the phases the core sends. Every place has eight waves (the shaders are compiled once).
- The river (`river.rs`): a ring-shaped channel whose current runs round it, fastest mid-channel, turned round each rock by a potential-flow doublet and slowed or reversed in each rock's wake. The sea asks it for the current, the bottom and how to push a point off a bank (`current_at`, `bed_at`, `push_out`).
- The hull (`hull.rs`): 16 buoyancy probes; pedals or propeller, drift about a pivot, drag against the moving water, the waves' slope, wind and the line's pull. Water drag and damping act only through the probes in the water, so a speedboat flies off a swell; a planing hull gets lift that grows as speed squared. Force sums are 64-bit (R4.11).
- The arm (`arms.rs`): one casting arm, shoulder and elbow on motors; the rod rises 20 degrees above the right forearm.
- The line (`line.rs`): XPBD with long-range ties to the tip and the line's stretch lumped on the end. Free spool pays out only up to the lure's distance plus 20% (T10). The drag holds up to the power wind's 26 N while it pulls. Bottom corrections are not counted as velocity (R4.8).
- The fish (`fish.rs`): cruise, notice, follow, strike, hold, spit; trout hold in eddies behind rocks. A bite needs the hook inside the mouth zone; a hook set is a 4 N tension rise within a quarter second.
- `world.rs` steps them together, runs the power wind's bursts and battery, and keeps fish in the water. `session.rs` holds the place and craft chosen, the fish log, the landed close-up, the fight cue and what is only seen; `flowmap.rs` packs the river's current for the water shader.

## Shape of the code

`crates/core/src/`: the modules above plus `mesh.rs`, `scene.rs` (instances, the river's banks and bed, the speedboat), `render.rs`, `look.rs` (shader constants from `data/look.json`, with each place's changes and each craft's camera), `strings.rs`. `crates/shim`: the `Game` class, Q16 to floats, the flow map's bytes. `web/`: `app.js`, `gl.js` (three programs and the flow-map texture, re-sent when the core's generation changes), `keys.js`, `ui.js`, `style.css`. `data/`: `gear.json` (both craft, the line, the power wind), `places.json` (both places), `look.json`, `controls.json`, `strings.json`, `shaders/`.

## Commands

`cargo test --workspace`; `./scripts/build.sh`; `python3 scripts/serve.py` (or any static server on `dist/`); `./scripts/check.sh`; `python3 scripts/break_tests.py`; `cargo test --release --test angler -- --include-ignored --nocapture` prints how long the scripted angler takes to land a fish at each place on eight seeds.

## What will bite within the hour

- `scripts/break_tests.py` edits core files while it runs; do not edit them at the same time.
- The page holds no numbers: a new shader constant goes in `data/look.json` and arrives as a uniform.
- `rustfmt` rewraps long lines, so a scripted edit made after formatting must match the formatted text.
- Run the tests in debug at least once: release builds wrap on overflow silently, and only the debug run caught the speedboat's force sums (R4.11).
- The glossary's `Not:` words are checked in every string value in `data/`, keys' values included: Kayak rules out "boat", Pedal drive rules out "motor" and "engine", Hook set rules out "jerk".

## Open for the person

Round 6 (T22 to T26):

- R6.1 the kayak at about 5 m/s.
- R6.2 a tap of Space closes the bail; held, it winds.
- R6.4 Q lets line out.

Round 5 (T18 to T21):

- R5.1 the Weather dropdown, built from the description of Ape Race, not Ape Race itself.
- R5.2 the storm: 6.8 m at 13 s, wind held at 26 kn.
- R5.4 the white water outruns the kayak.
- R5.5 the speedboat's new keys and handling numbers.
- R5.6 the river's scripted angler now has fifteen minutes.
- R5.8 the round-5 strings.

Round 4 (T8 to T17):

- R4.3 one casting arm on W, S, A and D, and the steady-hands setting gone.
- R4.4 the power wind's numbers: bursts of 150 ms on and 100 ms off at 7 m/s, a 26 N pull, a battery of 8 s that refills in 45 s.
- R4.6 the river read as a ring that never ends, rather than a jump back to its start.
- R4.10 river fishing is harder: the scripted angler takes about 3 minutes a fish there against under a minute at sea. Say if it should be easier.
- R4.12 the speedboat: about 80 to 90 km/h, no courses yet.
- R4.13 "Craft" as the chooser's word, because the glossary rules out "boat".
- R4.14 the fish log lasts the visit only.
- R4.18 the river's names, rocks and fish counts.
- R4.19 the round-4 strings and glossary entries.

From earlier rounds:

- R1.5 the glossary's terms and their `Not:` lines.
- R1.9 place names, species and the mockups' strings.
- R1.15 an anonymised Builder copy before the paper is submitted.
- R1.19 the far waters, exotic fish, creatures and costumes.
- R2.10 the drag is 10 N, not the plan's 7.1 N.
- R2.14 play and say how catching a fish feels; the scripted angler is not a person.
- R2.15 the round-2 strings in `data/strings.json`.
- R2.16 the website's two diverged branches, and Mage Tower Battle's listing.
- R2.17 the machine address in the public commit history.
- R3.3 mahi-mahi chase the power wind; red snapper do not.
- R3.4 the round-3 strings and the Power wind glossary entry.

## Mistakes so far

Round 1: a bibliography field from memory (R1.11); labels projected with a stale camera (R1.12). Round 2: a line solver that rang (R2.6), an overflow that sank every fish (R2.7), a drag that slipped on the wrong quantity (R2.8), strikes that could never bite (R2.9), a scope camera inside its own tube (R2.13). Round 4: a spool that ratcheted line off on every wave (R4.1), line flung 45 m up by the bank's correction counted as speed (R4.8), force sums that wrapped in release builds (R4.11), three tests that could not fail until the break run showed it (the rod's elbow, the motor's drag, the trout's hold) and a fourth replaced by a free-fall test.

## What passed

- Round 5 (T18 to T21): `./scripts/check.sh` BUILDER CHECK PASSED (118 tests from `./scripts/count-tests.sh`, 1 ignored; the browser check in all three engines now also picks the storm from the dropdown and reads its sea); `python3 scripts/break_tests.py` 38 of 38 caught. Published on the person's word (BRIEF.md, Round 6); the live browser check failed on one fragile winding step, since rewritten (R6.3).
- Round 4, at 14b18f5 and again at 803e9a4 (the close-up without the rod): `./scripts/check.sh` BUILDER CHECK PASSED on 17 October 2026 (format, lint, 113 tests from `./scripts/count-tests.sh` with 1 ignored, release build, import audit, art, notebook, and the browser check in Chromium, Firefox and WebKit, which now also drives the battery, the fish log, the Ledge River and the speedboat).
- `python3 scripts/break_tests.py`: 34 of 34 breaks caught.
- Published on the person's word "publish" (R4.20) with `./scripts/publish.sh` from 0a6f657 (build 8bb8364420f0); the browser check against `https://sgilson7.github.io/el-fishing` passed in Chromium, Firefox and WebKit.

## The next action

Try the Ledge River in the kayak first: cast upstream above a rock and let the current carry the lure past it, and when the Bite message shows, wind and sweep up with W together. Then take the speedboat out on the Open Swell at full throttle to feel it leave the water. Write what you find as rows in `TRIAGE.md` (T18 onward), and say whether to publish.
