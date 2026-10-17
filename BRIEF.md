# Brief

Stage 1. The person's own words come first; the rest can be written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

Pasted unchanged on 17 October 2026. It includes the instructions for this first round as well as the game, because a paraphrase is already an interpretation.

> clone the builder repo in here, you will be working entirely from within that framework, found here https://github.com/sgilson7/builder-setting/tree/main familiarize yourself with the process you will be following by reading all of the docs and files within builder, as you will be following that to execute the game build I will provide. set up a design journal like mage tower battle (found one directory up, also made with builder), then write about how you will follow the builder-setting to constrain your behavior to optimize the browser based game I will ask you to create. the game is a fishing game with the most realistic possible fluid mechanics we can create in 3d in builder for future use, my inspiration is sea of thieves level fluid mechanics.  then the fishing is about controlling your arms QWOP / sgilson7 vagrancy style to cast your rod, bait the hook around to attract fish, manuever it gently into their mouth, and then pull and start reeling them in, all being controlled by your arms pulling the rod which has an actual simulated line and hook that is biting into a fish mouth. you have a big scope attached to one eye you can use to zoom in on the fish you are catching, your character is in one of those kayaks with bike pedals to move around and can move directly forward easily, and can drift (keep front stationary and rotate the back of the kayak). start by creating the log, then doing a full research report, writing a paper you could submit to chi games cs ed conference on how youre going to optimize these fluids to run in the browser looking at inspirations like sea of thieves, other fluid mechanic simulation papers, and other games you think are good inspirations, and optimize them for the browser in 3d no less, in order to accomodate a player moving around on top of the water, which should be affected by the waves and tides and currents of the body of water they are in. then add the paper and its write up to the design log. use your findings to create mock ups of the different aspects of the fishing game, using the fidelity of the new mage tower and ape race as the target goal for how pretty the models and the character should look. take all the screenshots and add hem to the design log. then, stop and allow me to review both how you will implement the hyper realistic water simulation for fishing, and another tab for the mockups, incuding menus, an arcade mode where you go to different bodies of water like lakes, rivers, oceans, ponds, etc, all with screenshots

## The tool

*Extremely Lifelike Fishing* is a 3D fishing game that runs in a desktop browser. The player sits in a pedal-drive kayak on a body of water (a pond, a lake, a river, a coast, open ocean), and the water moves under them: swell, wind chop, tide and current all push the kayak and the line. The player drives two arms joint by joint, in the manner of QWOP and Vagrancy, to swing a rod. The rod carries a simulated line and hook. The player casts, works the bait to draw fish, eases the hook into a fish's mouth, sets it, and reels the fish in against its pull. A scope over one eye zooms in on the water and the fish. Pedals drive the kayak straight ahead; a drift holds the bow still while the stern swings round. An arcade mode moves between bodies of water.

The water model is meant to outlive this game: a deterministic, tested water core other Builder games can reuse.

## Done means

Draft, for the person to revise at stage 3. Each condition is checkable.

- The kayak's height, pitch and roll on any body of water come from the core's water model, and a core test with hand-worked numbers shows a floating hull settles at the waterline its mass and volume predict.
- The surface the page draws and the surface the core floats things on are the same waves: a test samples the core's surface at fixed points, and a browser interaction reads the drawn height at the same points through `data-*` attributes.
- Waves, tide and current each move the kayak, the line and the bait, and each has a core test that turns it off and shows the difference.
- Only the arm keys move the rod. A core test drives every other key and shows the rod, line and hook unchanged.
- The line is a chain of simulated points with a length that cannot stretch past its limit; it snaps above its breaking tension, with a test that works the breaking case out by hand.
- A fish bites only when the hook is inside its mouth zone; a hook set while the hook is outside does nothing; both are tests.
- Pedals drive the kayak forward; a drift turns the stern while the bow stays within a tolerance of its point; both are tests.
- The scope zooms in and out, and the zoom never changes a rule.
- The arcade mode lists at least a pond, a lake, a river and an ocean, each with its own water settings in `data/`.
- Two runs from the same seed and the same inputs end on the same checksum in Chromium, Firefox and WebKit.
- The game holds a frame rate on a school laptop that the person names at stage 3, measured and recorded in `MEASUREMENTS.md`.
- `./scripts/check.sh` passes.

## Decisions that stay with the person

- The game's name: *Extremely Lifelike Fishing* (chosen 17 October 2026), repository `el-fishing`.
- The venue for the paper. The description says "chi games cs ed conference"; the draft is written for CHI PLAY with a CS-education strand, and the venue is the person's choice.
- The fish: which species, how many, how each one behaves, and whether a caught fish is kept or released.
- The key layout for the arms, the pedals and the scope.
- Whether progress between visits is kept (D17 bans storage by default).
- Whether there is online play (the online kit), and whether WebGPU is required or optional.
- Any music or sound from outside the repository.

## Accessibility

Bites, line tension and danger of a snap are shown by shape, motion and sound, not by colour alone: the line bows, the rod tip bends, a tension bar fills and a ring tightens around the reel. Keyboard and mouse. A gamepad is a question for stage 3. A setting slows the arm motors for players who find QWOP control too hard, and the setting never changes a fish's rules.

## Style

The faceted look of Ape Race and the Mage Tower round-8 mockups, as the person asked: few-sided shapes with flat faces and hand-cut corners, sky and ground light with a wrapping sun, haze in each place's own colour, one bright colour per scene for what matters to play, glow added as light, and dark see-through menu panels with one gold button. The water is the exception: it is drawn smooth, with reflection, refraction and foam, because realistic water is the brief's first goal. The TikZ default style pack is not used.

## Words

Terms from fishing and paddling, defined in `GLOSSARY.md` before any string uses them: bait, bite, cast, current, drag, drift, hook set, kayak, line, lure, pedal drive, reel, rod, scope, strike, swell, tension, tide.

## Autonomy

**Round 4 (17 October 2026): no publishing grant.** The person asked for the river map, a scope that sees through the kayak, a shorter cast, a fish log, one casting arm, a fight cue, a wake, a battery power wind and a speedboat (`TRIAGE.md` T8 to T17), and named no publishing. The agent builds and checks locally and leaves publishing to the person. After reading the round's report, the person wrote, word for word: 

> publish

So the agent publishes round 4 to `sgilson7.github.io/el-fishing` and runs the browser check against that address. The website listing is unchanged.

**Round 2 (17 October 2026): the plan is approved in advance.** The person wrote, word for word:

> name the game "Extremely Lifelike Fishing", take your recomemndation to answer every question, begin the build, dont stop till the mvp is deployed, i give explicit permision to make deploy the website repo with a new chip when the mvp is deployed. start by getting just the first open ocean swells map ready for me to test, don't do the rest of whats in the mockup; just everything necessary to get the first level working and able to catch fish in it
>
> the tag for website should be el-fishing
>
> as in the website extension sgilson7.github.io/el-fishing

So the agent writes `PLAN.md` from the person's chosen recommendations and builds without stopping at stage 3. The first build is one place, the Open Swell, with everything needed to catch a fish there; the rest of the mockups wait. The gate before publishing becomes `./scripts/check.sh` passing and a hand-worked case checked in the browser. The agent may create the public repository `el-fishing`, publish it to GitHub Pages at `sgilson7.github.io/el-fishing`, run the browser check against that address, and then add the game to the person's website repository (`scholar-site`) with a chip tagged `el-fishing` and deploy the website. Scope: this MVP.

**Round 1 (kept for the record):** The plan is **not** approved in advance: the person asked to review the water design and the mockups before anything is built. In this round the agent may write the brief, the glossary, the notebook, the research report, the paper and the mockups, and must then stop. Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues within this round. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request to the game. The design journal may be shared with the person as a private page.

Publishing, granted on 17 October 2026 in the person's words: *"fyi i give you approval to create a public github repo, and a public github pages when the time comes to store and deploy"*. The agent may create a public GitHub repository for the game and publish it to GitHub Pages with `./scripts/publish.sh`, once the plan has been read (stage 3), the game is built, and `./scripts/check.sh` passes, then run the browser check against the live address. "When the time comes" is read as: not in this design round. The repository takes the game's name once the person chooses it (`SECOND-ORDER.md` R1.1).

## Allowed core dependencies

`serde`, `serde_json` (reference default). No others are proposed.
