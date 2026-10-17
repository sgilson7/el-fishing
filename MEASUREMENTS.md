# Measurements

Stage 4, recorded by the agent before building and again at handoff. `B2 Require step-by-step justification`.

The state measured rather than recalled, so later changes can be compared with it. A probe prints numbers and asserts loose bounds at most; record its output here with the command that produced it.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0 (b940084d7 2026-09-28); wasm-bindgen 0.2.127 | 2026-10-17 |
| Core tests | `./scripts/count-tests.sh` | 91 passed, 0 failed, 1 ignored | 2026-10-17 |
| Full check | `./scripts/check.sh` | BUILDER CHECK PASSED (Chromium, Firefox, WebKit) | 2026-10-17 |
| WebAssembly size | printed by `./scripts/build.sh` | 476 KB | 2026-10-17 |
| Breaks caught | `python3 scripts/break_tests.py` | 15 of 15 (round 3 added one) | 2026-10-17 |
| Frame rate, Open Swell, 1440 × 900 | Playwright Chromium, headed, GPU on: frames counted over 5 s after the start (`fps.py` in the session's scratchpad, not kept) | 120 a second (the display's rate); worst frame 9.4 ms; renderer ANGLE Metal, Apple M2 Max | 2026-10-17 |
| Time for the scripted angler to land a fish | `cargo test --test angler -- --ignored --nocapture` | round 2: 180, 184, 191, 200, 256, 259, 330 and 412 s; round 3, with the weaker fish and faster reel (T3, T4): 70, 123, 68, 146, 43, 75, 123 and 68 s, on seeds 1 to 8 | 2026-10-17 |

## Not measured

Devices, browsers and conditions no measurement here covers.

- A school laptop with integrated graphics (W5): the frame rate above is from an M2 Max and says nothing about it. The first thing to measure when one is to hand.
- Phones and tablets: the game is keyboard-only and not built for them.
- The time a person takes to land a fish (R2.14).

## Water smoothness (T2)

| What | Command | Result | When |
| --- | --- | --- | --- |
| Mean change between consecutive frames, water only, HUD hidden, 960 × 540 | Chrome screencast of 3 s at the display's rate (`cast.py` in the session's scratchpad, not kept) | before: alternating 0.2 and 1.1 with spikes to 2.7 (the waves stepped on ticks and the grid snapped); after: an even 0.5 to 0.7, with 1.0 on frames where the kayak's own tick lands | 2026-10-17 |
