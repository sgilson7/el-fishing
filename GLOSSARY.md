# Glossary

The source of truth for what each word in this tool means. Strings, tests, the plan and the README use a term in the sense given here. The page shows a definition when a reader hovers the term.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

### Bait

Something on the hook that a fish wants to eat. In this game the bait is worked through the water by moving the rod.

Not: food, treat

### Bail

The wire arm on the reel. Open, it lets line run off freely, as in a cast; closed, line comes off only through the drag. Winding the reel closes it.

Not: latch, gate

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

### Pedal drive

The pedals under the angler's feet that push the kayak forward. A rudder at the stern steers it.

Not: paddle, motor, engine

### Power wind

An electric motor on the reel that brings the line in at {reel.power_wind}, much faster than winding by hand. It runs only while no fish has the lure.

Not: turbo, boost

### Reel

The spool on the rod that winds line in and lets it out. Winding the reel shortens the line by {reel.wind_per_turn} a turn.

Not: winch, spool

### Rod

The long, bending pole the angler holds in both hands. Its tip bends under the line's tension, and that bend is a cue the player reads.

Not: pole, stick

### Scope

The zoom lens strapped over one of the angler's eyes. It magnifies the view up to {scope.max_zoom} times and never changes a rule.

Not: binoculars, telescope, spyglass

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

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->
