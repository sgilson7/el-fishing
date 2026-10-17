# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

Round 2, 17 October 2026. The person approved the plan in advance and answered every open question with the agent's recommendation (`BRIEF.md`, Autonomy; `DECISIONS.md`). This plan covers the MVP: one place, the Open Swell, with everything needed to catch a fish there. The other places, weather, costumes and menus in the mockups wait for later rounds.

## Answers to the open questions

From the design journal's water tab (W) and mockups tab (M). "Recommendation" means the person took the agent's; "agent's choice" marks a question that had no recommendation, decided here and open to change in `TRIAGE.md`.

| Question | Answer |
| --- | --- |
| W1 The split sea | Yes: L0 level, L1 waves and L2 current are rules in the core in fixed point; L4 detail is GPU only. L3 (the local ripple grid) is drawn on the GPU from values the core sends and moves nothing in the MVP; it becomes a core layer when something has to feel it. |
| W2 Gerstner or FFT | A Gerstner sum in the core (recommendation). |
| W3 How hard the sea pushes | Full physics, a steady-hull setting that adds damping, and no capsizing (recommendation). |
| W4 Tide length | One full cycle in 20 minutes of play (recommendation). |
| W5 The slowest machine | Agent's choice: a school laptop with integrated graphics. Measured on this Mac first and recorded in `MEASUREMENTS.md`. |
| W6 Keys | Q/A and W/S for the left shoulder and elbow, I/K and O/L for the right, F holds the line, Space winds, arrows pedal and drift, right mouse holds the scope (recommendation). |
| W7 Mouse on the rod | Keys only first (recommendation). |
| W8 Keep or release, and a log | Agent's choice: Release is the default button and Keep is offered; the catch log lasts one visit, because D17 stays (no storage). |
| W9 Name and repository | *Extremely Lifelike Fishing*, repository `el-fishing`, at `sgilson7.github.io/el-fishing` (the person's words). |
| W10 Paper venue | CHI PLAY as drafted (recommendation). |
| M1 The look | Agent's choice: yes, the faceted world with smooth water. |
| M2, M10 Place names and lists | Agent's choice: keep the drafts; only the Open Swell is built. |
| M3 Fish | Keep the list; the Open Swell has mahi-mahi and red snapper. |
| M4 Scope | The full circle while the button is held (recommendation). |
| M5 HUD | Keep the tension ring; the arm panel folds away after the first catch (recommendation). |
| M6 Title | *Extremely Lifelike Fishing* replaces the working title. |
| M7 Volcano and Jupiter physics | Measured numbers where they exist, "invented" labels elsewhere (recommendation); not built in the MVP. |
| M8, M9 Costumes | Two costumes open places; a save code instead of storage (recommendations); not built in the MVP. |
| M11 Weather | Changes during a visit from the place's seed (recommendation); the MVP has one fixed sky. |

## Decisions in build order

Numbers in the tests are worked by hand here, before the code exists.

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | Q16.16 fixed point (`fx.rs`), seeded chance (`rng.rs`) and fixed 60 Hz ticks (`ticks.rs`) from the realtime kit, as Mage Tower Battle uses them. | D2: no floats, clocks or randomness in the core. | The kit's own tests, kept. |
| 2 | Each wave's phase is a wrapping 32-bit binary angle stepped once a tick; the period comes from the deep-water dispersion relation T = √(2πL/g). | The core and the shader agree on phase without either keeping time in floats (R1.10). | A 156.13 m wave has a 10 s period, so its step is 2³²/600 = 7,158,279 (within 0.1%), and after 600 ticks it is back within 0.1% of a turn of its start. |
| 3 | The sea surface is level (tide) plus a sum of Gerstner waves; the core reads height and slope at a point with one fixed-point step back through the horizontal displacement. | Probes need heights at points, and the shader draws the displaced grid. | One wave, A = 1 m, L = 10 m, no steepness: the height 2.5 m along its direction at phase 0 is 1 m (a quarter turn), within 2 mm. |
| 4 | The Open Swell's eight wave components are authored in `data/place.json`; the HUD's swell height is computed from them as Hs = 4√(Σa²/2). | Numbers are computed, not typed. | Amplitudes 0.55, 0.35, 0.25, 0.18, 0.10, 0.06, 0.03, 0.015 m give Σa² = 0.5346, so Hs = 2.07 m. |
| 5 | Tide: level = 0.6 m × sin of a phase that turns once in 20 minutes (72,000 ticks). | W4. | A quarter period (18,000 ticks) gives +0.6 m within 2 mm; the tide off gives level 0. |
| 6 | Current: one water velocity for the Open Swell, 0.4 kn (0.206 m/s). It carries the kayak, the line and the lure through drag on each. | L2 for a place with no rivers is one cell. | With the pedals idle on flat water, the kayak's speed rises toward 0.206 m/s and is within 5% after 60 s; with the current off it stays 0. |
| 7 | The kayak floats on 16 probes on its waterplane, each pushing up by ρ g a d (d = depth below the surface at that probe, capped by the hull's depth), with heave, pitch and roll damped and roll limited so it cannot capsize. | Kerner's model, as in the water tab. | A 130 kg kayak with 16 probes of 0.147 m² on still sea water (1025 kg/m³) settles 130 ÷ (1025 × 2.352) = 0.0539 m deep, within 1 mm. |
| 8 | The pedal drive pushes 120 N along the keel against drag 50 v² N along it and ten times that across it. | The mockup's "up to 3 kn". | Top speed on still water with no current is √(120/50) = 1.549 m/s (3.01 kn), within 1% after 30 s. |
| 9 | Drift turns the kayak at 30° a second about its bow. | The description: "keep front stationary and rotate the back". | After one second of drift on still water the heading has turned 30° (within 1°) and the bow has moved under 5 cm. |
| 10 | Two arms, each a shoulder and an elbow in the angler's upright plane, driven by motors while their keys are held. The rod runs from the upper (right) hand through the lower (left) hand's direction; its tip is the line's anchor. | QWOP and Vagrancy control, as the description asks. | Every key other than the eight arm keys leaves the rod, line and hook where they were after a second of holding it (pedal and drift tested with the kayak's motion taken out). |
| 11 | The line is a chain of 40 points solved by follow-the-leader projection from the rod tip, so no segment is ever longer than its share of the line out; slack line sags. Water drag, sinking, current and wind act on each point. | Inextensible by construction; stable with a heavy fish at the end (Müller et al. 2012, recorded in `LICENSES.md`). | Under a fish's pull the chain's length never exceeds the line out by more than 1 mm. |
| 12 | Tension is read from the end load; 8 lb fluorocarbon breaks at 35.6 N; the drag slips at 7.1 N. | The water tab's worked numbers. | A 20 g lure hanging still from the tip pulls 0.02 × 9.81 = 0.196 N (within 5%); a pull of 36 N snaps the line and 35 N does not. |
| 13 | F opens the bail with a finger on the line; letting go of F lets line run; Space closes the bail and winds 0.6 m a turn at 2 turns a second. | A spinning reel's real sequence. | A cast with F released at a tip speed of 10 m/s carries the lure more than 10 m; with F held throughout it stays within the line out. |
| 14 | Fish are agents: cruise, notice a lure that moves in their speed band, follow, strike by opening the mouth, hold a bitten lure for a spit window of 1.0 s, then spit. A bite happens only when the hook is inside the mouth zone (one gape across) while the mouth is open. | The brief's done-means. | A strike with the hook 2 cm inside the zone bites; with the hook 2 cm outside it misses. |
| 15 | A hook set is a rise in tension above 4 N within a quarter second while the lure is held. A hook set with no bitten lure does nothing. | The brief's done-means. | A hook set while the fish holds the lure hooks it; the same pull before the bite leaves every fish unhooked. |
| 16 | A hooked fish pulls with its strength times its stamina, which drains with the work it does; it is landed when it is tired and within 1.6 m of the angler with under 3 m of line out. | So reeling and drag decide the fight. | A fish with no stamina and the reel winding comes within reach and is landed. |
| 17 | The scope shows the angler's eye view at 2 to 8 times, toward the lure, and the water clears while it is held. It changes no rule. | Done-means: "the zoom never changes a rule". | Two runs with the same inputs, one holding the scope throughout, end on the same world checksum. |
| 18 | Two runs from the same seed and inputs end on the same checksum. | D2. | A scripted minute of play run twice gives one checksum. |
| 19 | The page is Mage Tower Battle's renderer pattern: the core sends meshes, instances, views and every shader constant as Q16 numbers; the shim turns them into floats; the shaders in `data/shaders/` hold only 0 and 1. The water is its own program drawn from the same wave parameters the core floats things on. | D3, R6. | The boundary tests. |
| 20 | The core describes the HUD and screens (`ui`); the page draws panels generically. | R4, after Peter's games. | Browser interactions: the title screen starts the Open Swell, the HUD shows the place, and a scripted cast puts line out. |

## Open questions for the person

None blocking. Later choices go in `TRIAGE.md` once the person has played.

## Status

- [x] Approved in advance by the person on 17 October 2026 (`DECISIONS.md`).
