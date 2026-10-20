# Glossary

The source of truth for what each word in this tool means. Strings, tests, the plan and the README use a term in the sense given here. The page shows a definition when a reader hovers the term.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

### Bail

The wire arm on the reel. Open, it lets line run off freely, as in a cast; closed, line comes off only through the drag. Winding the reel closes it.

Not: latch

### Bait

Something on the hook that a fish wants to eat. In this game the bait is worked through the water by moving the rod.

Not: food, treat

### Battery

The charge that runs the power wind. A full battery runs it for {battery.seconds} s, and an empty one refills in {battery.recharge} s while the power wind rests.

Not: fuel, energy

### Bite

The moment a fish closes its mouth on the bait. A bite counts only while the hook is inside the fish's mouth zone.

Not: nibble, chomp

### Buoy

A tall float marking the course, with a flag pointing the way past it. A red buoy points you to its left, so it stays on your right; a yellow one points you to its right. Each passed on its side lights a power arrow.

Not: marker, cone

### Burner

The flame under the lift bag's mouth. Hold B and it burns, the hydrogen in the bag heats and the kayak climbs; let go and the bag cools and the kayak sinks back toward the cloud deck.

Not: thruster, torch

### Canister

A red nitrous canister floating on a river course. Drive or pedal over it to pick it up; you can hold {race.canisters_held} at a time, and Space fires one for a burst of nitrous.

Not: item

### Cast

Swinging the rod so the line pays out and the bait flies out over the water. The distance comes from how fast the rod tip moves when the line is let go.

Not: throw, toss, launch

### Checkpoint

One of the course's corners. The race card counts the ones you have passed out of all of them.

Not: waypoint

### Cloud deck

The top of Jupiter's clouds, in pale zones and darker belts. It is drawn and felt with the same waves as water, slowed. The line, the lure and the creatures sink into it; the kayak hangs just over it.

Not: cloud sea, fog bank

### Crank

One turn of the reel handle with a fish on, made by pressing Space as the dial's needle crosses the sweet spot. A crank brings in line and tires the fish; a press off the sweet spot is a miss.

Not: click, mash

### Current

Water moving in one direction across a body of water, such as a river's flow or a tidal stream. It carries the kayak, the line and the bait.

Not: stream, flow rate

### Dial

The ring that shows the reel handle going round while a fish is on. Its needle turns once a handle turn, and the sweet spot is marked on it.

Not: clock, gauge

### Drag

The setting on the reel that lets line slip out when the pull on it passes a limit, so a strong run does not snap the line.

Not: friction, brake

### Drift

Turning the kayak by swinging the stern while the bow holds its place, so the angler can face a new spot without leaving it.

Not: spin, pivot

### Eddy

Slack water behind a rock in a current, where the flow slows and turns back on itself. Trout hold in eddies and wait for food to drift past.

Not: whirlpool, vortex

### Gap

A gap between two bands of ice in Saturn's rings, kept clear by a moon's pull. There is nothing to float on over it: jump it at speed, or fall through and wipe out.

Not: crack, chasm

### Gate

A slalom gate on a river race: two green and white poles hanging over the water from a wire across the canyon. Pass between the poles to light a power arrow; miss it and they all go out.

Not: door, hoop

### Ghost

Your best run on a course, drawn as light, racing beside you as you went then. It is kept for the visit.

Not: replay

### Gondola

A cabin that runs on a cable from the bottom of the river run back to the top, with the kayak hung below it in a sling. Ride it from the dock in the last pool.

Not: cable car, chairlift

### Haul

Bringing a tired fish in fast with the power wind, so it comes out of the water and into the angler's hands.

Not: yank, fling

### Hole

A place below a ledge or a covered rock where the water drops and its surface turns back upstream. It holds a kayak for a moment; pedal out, or drift to one end where it spills.

Not: hydraulic, stopper

### Hook set

A sharp pull on the rod that drives the hook into the fish's mouth after a bite. A hook set with the hook outside the mouth does nothing.

Not: yank, jerk

### Jet

A band of wind round Jupiter that blows east or west. Over the cloud deck it is the current: it carries the kayak, the line and the creatures.

Not: breeze, gust

### Kayak

The narrow boat the angler sits in. It floats on the water model and is pushed by swell, tide, current and the pedal drive.

Not: canoe, boat, raft

### Lean

Shifting the driver's weight with W and S. Forward gives more grip; back turns tighter and costs speed. In the air the same keys trim the nose.

Not: tilt

### Lift bag

The bag of hot hydrogen the kayak hangs from over Jupiter's clouds. It holds the kayak where its lift equals the kayak's weight, and the burner raises that level.

Not: blimp, zeppelin

### Line

The thin cord between the reel and the hook, simulated as a chain of points. It sags, sinks, is carried by current and snaps above its breaking tension of {line.break_tension}.

Not: string, rope, wire

### Lip

The edge where a pool spills into the rapid below it. The water there is smooth and fast, a tongue pointing the way in.

Not: edge, brink

### Lure

Bait made to look alive, such as a spinner or a soft plastic. It is worked with twitches and pauses of the rod.

Not: jig, decoy

### Medal

Gold, silver or bronze, for a finish inside the course's times.

Not: award

### Moonlet

A small moon inside Saturn's rings, from a few metres to a few kilometres across, such as Pan and Daphnis. Its pull ripples the ice beside it.

Not: asteroid

### Moonlet wake

The ripples a moonlet leaves in the ice behind it, which you can ride. Small moonlets leave them in a propeller shape.

Not: trail

### Mouth zone

The space in front of a fish's open mouth that its suction reaches, about one gape across. A bite counts only while the hook is inside it.

Not: target, hitbox

### Nitrous

A burst of {race.nitrous_top_pct}% more top speed for {race.nitrous_seconds} s, from driving through a ring or from a perfect start.

Not: speed-up

### Outrigger

A long pole swung out from each side of the sportfisher. Its line runs out to a clip at the pole's end, so its lure trolls wide of the others.

Not: boom, spar

### Pedal drive

The pedals under the angler's feet that push the kayak forward. A rudder at the stern steers it.

Not: paddle, motor, engine

### Pool

A deep, slow stretch of river between two rapids, where fish hold and casting is easy.

Not: lake, basin

### Power arrow

One of up to {race.arrows} arrows lit by buoys passed on their side or gates passed between their poles, each worth {race.arrow_top_pct}% more top speed. A buoy passed on the wrong side, or a gate missed, puts them all out.

Not: pip

### Power wind

An electric drive on the reel that pulls line in short bursts at {reel.power_wind}, much faster than winding by hand. It runs on the battery, with or without a fish on.

Not: turbo, boost

### Propeller

Over Jupiter's clouds the pedal drive turns a propeller in place of fins, so the same keys push the kayak through the air.

Not: rotor, fan

### Race

A run against three rivals from the start lights to the line: round a course on the open sea or Saturn's rings in a speedboat, or down the Ledge River run in a speedboat or a kayak.

Not: heat

### Ramp

A floating ramp on pontoons. Take it straight at speed for a long jump; its sides and its back are solid.

Not: kicker

### Rapid

A steep, fast stretch of river between two pools, too fast to pedal up. Your rod stows itself there.

Not: cascade

### Reel

The spool on the rod that winds line in and lets it out. Winding the reel shortens the line by {reel.wind_per_turn} a turn.

Not: winch, spool

### Ring

A gold ring standing in the water or over a ramp's landing. Drive or pedal through it and its nitrous fires at once.

Not: hoop

### Ring plane

The flat sheet of Saturn's rings, ice in bands round the planet, raced on as water. Down is toward it: the ring's own pull and Saturn's tug back to its equator.

Not: disc

### Rival

A speedboat or kayak the game drives, on its own line along the course, at its own pace.

Not: bot, enemy

### Rod

The long, bending pole the angler holds in the casting hand. Its tip bends under the line's tension, and that bend is a cue the player reads.

Not: pole, stick

### Scope

The zoom lens strapped over one of the angler's eyes. It magnifies the view up to {scope.max_zoom} times and never changes a rule.

Not: binoculars, telescope, spyglass

### Ship

A big vessel on a fixed timetable: a container ship, a tanker, a superyacht at anchor or a ketch under sail. Its hull is solid and its wake is real water you can ride.

Not: liner

### Speedboat

A fast planing craft, raced with Wave Race's keys: hold Up for throttle, steer with the outboard, lean with W and S. It rides up on its hull at speed and leaves the water off a swell, a wake or a ramp.

Not: jetski, hovercraft

### Sportfisher

The craft for trolling, with a centre console, outriggers and rods in holders. It is driven with a throttle lever and an outboard, like the speedboat, at trolling speeds.

Not: trawler, yacht

### Spread

The sportfisher's lines together: each rod's line streaming behind at its own distance and depth.

Not: array, fleet

### Streak

Cranks in a row on the sweet spot. Each crank of a streak brings in more line than the last; a miss ends it.

Not: combo, chain

### Strike

A fish's fast rush at the bait before it bites. A strike can miss.

Not: attack, hit

### Sweet spot

The marked arc on the dial where a press of Space makes a crank. It narrows as the pull nears the drag and widens as the fish tires; on some fish it drifts round the dial or moves after every crank.

Not: target, window

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

### Wake

The waves a moving hull leaves: the V behind the kayak, and behind a ship its bow wave and two long arms spreading at Kelvin's angle.

Not: trail

### Wipeout

A landing too steep, nose first: the hull stops, and after {race.respawn_seconds} s it is set back on the course.

Not: crash

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->
