# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

A 3D browser fishing game: a pedal-drive kayak on simulated water, two arms driven joint by joint to cast, work a bait, set a hook and land a fish on a simulated line, a scope over one eye, and an arcade of five bodies of water. As of 17 October 2026 only the design round is done: brief, glossary, notebook, research, a draft paper and mockups. No game code exists. The design journal is `../journal/fishing-journal.html`; the paper is in `../paper/`.

## Rules that hold it up

The water design in the journal's water tab: layers L0–L3 (level, waves, current, ripples) are rules in the core in fixed point, evaluated at probe points; L4 is GPU-only detail no rule reads. Wave phase is a wrapping 32-bit binary angle in the core, sent to the shader each frame. None is built or tested yet.

## Shape of the code

The unchanged Builder template (26ea47c), plus `BRIEF.md`, `GLOSSARY.md` (18 terms), `SECOND-ORDER.md`, `DECISIONS.md`.

## Commands

As in the template's `README.md`. `cargo test --workspace` passes 38 template tests.

## What will bite within the hour

The plan is not approved in advance. Do not write `PLAN.md` decisions into code until the person answers the questions in the journal's water and mockups tabs. The publishing grant in `BRIEF.md` applies only after stage 3 and a passing check, and needs a repository name (R1.1).

## Open for the person

- R1.1 the game's name and repository name.
- R1.2 the paper's venue: CHI PLAY as drafted, or a CS-education venue.
- R1.5 the eighteen glossary terms and their `Not:` lines.
- R1.6 smooth water inside a faceted world.
- R1.8 Gerstner sum in the core, FFT only as eye detail.
- R1.9 place names, species, working title and mockup strings.
- R1.14 the draft key layout.
- R1.15 an anonymised Builder copy before the paper is submitted.

## Mistakes so far

Research and writing: a bibliography field written from memory (R1.11) was caught and checked; the rule is that bibliography fields come from an opened source. Mockup tool: labels projected with a stale camera matrix (R1.12).

## What passed

`cargo test --workspace`: 38 passed, 0 failed (17 October 2026, before any game code). `./scripts/check.sh` has not been run in this project; the browser check needs `./scripts/setup.sh` first.

## The next action

Read the person's answers to the water and mockup questions, then write `PLAN.md` from the brief and the water design, and stop for stage 3.
