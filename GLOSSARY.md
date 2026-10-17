# Glossary

The source of truth for what each word in this tool means. Strings, tests, the plan and the README use a term in the sense given here. The page shows a definition when a reader hovers the term.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

### Bail

The wire arm on the reel. Open, it lets line run off freely, as in a cast; closed, line comes off only through the drag. Winding the reel closes it.

Not: latch, gate

### Bait

Something on the hook that a fish wants to eat. In this game the bait is worked through the water by moving the rod.

Not: food, treat

### Battery

The charge that runs the power wind. A full battery runs it for {battery.seconds} s, and an empty one refills in {battery.recharge} s while the power wind rests.

Not: fuel, energy

### Bite

The moment a fish closes its mouth on the bait. A bite counts only while the hook is inside the fish's mouth zone.

Not: nibble, chomp

### Cast

Swinging the rod so the line pays out and the bait flies out over the water. The distance comes from how fast the rod tip moves when the line is let go.

Not: throw, toss, launch

### Current

Water moving in one direction across a body of water, such as a river's flow or a tidal stream. It carries the kayak, the line and the bait.

Not: stream, flow rate

### Drag

The setting on the reel that lets line slip out when the pull on it passes a limit, so a strong run does not snap the line.

Not: friction, brake

### Drift

Turning the kayak by swinging the stern while the bow holds its place, so the angler can face a new spot without leaving it.

Not: spin, pivot

### Eddy

Slack water behind a rock in a current, where the flow slows and turns back on itself. Trout hold in eddies and wait for food to drift past.

Not: whirlpool, vortex

### Haul

Bringing a tired fish in fast with the power wind, so it comes out of the water and into the angler's hands.

Not: yank, fling

### Hook set

A sharp pull on the rod that drives the hook into the fish's mouth after a bite. A hook set with the hook outside the mouth does nothing.

Not: yank, jerk

### Kayak

The narrow boat the angler sits in. It floats on the water model and is pushed by swell, tide, current and the pedal drive.

Not: canoe, boat, raft

### Line

The thin cord between the reel and the hook, simulated as a chain of points. It sags, sinks, is carried by current and snaps above its breaking tension of {line.break_tension}.

Not: string, rope, wire

### Lure

Bait made to look alive, such as a spinner or a soft plastic. It is worked with twitches and pauses of the rod.

Not: jig, decoy

### Mouth zone

The space in front of a fish's open mouth that its suction reaches, about one gape across. A bite counts only while the hook is inside it.

Not: target, hitbox

### Outrigger

A long pole swung out from each side of the sportfisher. Its line runs out to a clip at the pole's end, so its lure trolls wide of the others.

Not: boom, spar

### Pedal drive

The pedals under the angler's feet that push the kayak forward. A rudder at the stern steers it.

Not: paddle, motor, engine

### Power wind

An electric drive on the reel that pulls line in short bursts at {reel.power_wind}, much faster than winding by hand. It runs on the battery, with or without a fish on.

Not: turbo, boost

### Reel

The spool on the rod that winds line in and lets it out. Winding the reel shortens the line by {reel.wind_per_turn} a turn.

Not: winch, spool

### Rod

The long, bending pole the angler holds in the casting hand. Its tip bends under the line's tension, and that bend is a cue the player reads.

Not: pole, stick

### Scope

The zoom lens strapped over one of the angler's eyes. It magnifies the view up to {scope.max_zoom} times and never changes a rule.

Not: binoculars, telescope, spyglass

### Speedboat

A fast planing craft driven with the same keys as the kayak. It rides up on its hull at speed and leaves the water off a swell.

Not: jetski, hovercraft

### Sportfisher

The craft for trolling, with a centre console, outriggers and rods in holders. It is driven with a throttle lever and an outboard, like the speedboat, at trolling speeds.

Not: trawler, yacht

### Spread

The sportfisher's lines together: each rod's line streaming behind at its own distance and depth.

Not: array, fleet

### Strike

A fish's fast rush at the bait before it bites. A strike can miss.

Not: attack, hit

### Swell

Long waves that roll in from far away, independent of the local wind. They lift and tilt the kayak slowly.

Not: surf, chop

### Tension

The pull along the line, shown by the rod's bend, the line's bow and the tension ring. Above {line.break_tension} the line snaps.

Not: stress, strain, pressure

### Tide

The slow rise and fall of the water level on a coast, over a cycle of {tide.period_minutes} minutes in game time. It moves the shoreline and the tidal current.

Not: sea level

### Trolling

Fishing by towing lures behind a moving hull, from rods in holders, instead of casting them.

Not: trawling, dragging

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->
