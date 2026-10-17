//! One place and everything in it, a tick at a time: the sea, the kayak,
//! the arms and rod, the line and the fish. The world reads only the
//! player's key bits and its own seed; the scope, the camera and the HUD
//! live outside it, so none of them can change a rule.

use serde::Deserialize;

use crate::arms::{ArmInput, ArmSpec, Arms};
use crate::fish::{Fish, FishSpec, Happened, Mood, Scene};
use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::hull::{Hull, HullInput, HullSpec};
use crate::line::{EndLoad, Line, LineInput, LineSpec};
use crate::rng::Rng;
use crate::sea::{Sea, SeaSpec};
use crate::ticks::TICKS_PER_SECOND;

/// The key bits the page sends; `data/controls.json` maps keys to their names.
pub mod bits {
    pub const SHOULDER_UP: u32 = 1;
    pub const SHOULDER_DOWN: u32 = 1 << 1;
    pub const ELBOW_BEND: u32 = 1 << 2;
    pub const ELBOW_STRAIGHTEN: u32 = 1 << 3;
    pub const FINGER: u32 = 1 << 4;
    pub const WIND: u32 = 1 << 5;
    pub const PEDAL: u32 = 1 << 6;
    pub const BACK_PEDAL: u32 = 1 << 7;
    pub const DRIFT_LEFT: u32 = 1 << 8;
    pub const DRIFT_RIGHT: u32 = 1 << 9;
    pub const SCOPE: u32 = 1 << 10;
    pub const POWER_WIND: u32 = 1 << 11;
    /// Q: let line out off the reel, the bail closed, before a cast (T23).
    pub const LET_OUT: u32 = 1 << 12;
    /// The four keys that move the casting arm (T12: one arm casts).
    pub const ARMS: u32 = 0xf;
    pub const NAMES: [(&str, u32); 13] = [
        ("shoulder_up", SHOULDER_UP),
        ("shoulder_down", SHOULDER_DOWN),
        ("elbow_bend", ELBOW_BEND),
        ("elbow_straighten", ELBOW_STRAIGHTEN),
        ("finger", FINGER),
        ("wind", WIND),
        ("pedal", PEDAL),
        ("back_pedal", BACK_PEDAL),
        ("drift_left", DRIFT_LEFT),
        ("drift_right", DRIFT_RIGHT),
        ("scope", SCOPE),
        ("power_wind", POWER_WIND),
        ("let_out", LET_OUT),
    ];
}

/// The reel's electric motor: it pulls in jerks and runs from a battery
/// that recharges while it rests (T15).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerSpec {
    /// Each jerk pulls for `on_ms`, then rests for `off_ms`.
    pub on_ms: i32,
    pub off_ms: i32,
    /// Seconds of running a full battery gives, and seconds to refill an empty one.
    pub battery_s: i32,
    pub recharge_s: i32,
}

/// A boat to take out: its hull, and whether it is fished from. The
/// speedboat is driven, not fished from (the person's boat racing mode).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoatSpec {
    /// The key of its name in the strings file.
    pub name: String,
    pub fishing: bool,
    /// Kept for the racing maps to come, and not offered at a fishing place (T25).
    #[serde(default)]
    pub racing: bool,
    pub hull: HullSpec,
}

/// The angler's gear, from `data/gear.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearSpec {
    /// A note for people; the core does not read it.
    #[serde(default, rename = "_note")]
    pub note: String,
    pub boats: Vec<BoatSpec>,
    pub arms: ArmSpec,
    pub line: LineSpec,
    pub power: PowerSpec,
    /// A hook set: the tension rising this much (tenths of N) within this long (ms).
    pub hook_set_dn: i32,
    pub hook_set_ms: i32,
    /// Landing: the fish within this reach of the angler with less line out.
    pub land_reach_cm: i32,
    pub land_line_cm: i32,
}

/// A place, one of the list in `data/places.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    /// A note for people; the core does not read it.
    #[serde(default, rename = "_note")]
    pub note: String,
    pub name: String,
    pub sea: SeaSpec,
    pub start_heading_deg: i32,
    pub fish: Vec<FishSpec>,
    #[serde(default)]
    pub scenery: ScenerySpec,
    /// The key of the name of the place's own weather, the first in the
    /// weather choice; a variant replaces it with its own (T18).
    pub weather: String,
    /// Rain drops drawn round the eye; 0 for none. Only seen.
    #[serde(default)]
    pub rain: i32,
    /// Other weathers for the same place: another sea, rain and clouds.
    #[serde(default)]
    pub variants: Vec<VariantSpec>,
}

/// A weather for a place (T18): the same place and fish under another sea.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantSpec {
    #[serde(default, rename = "_note")]
    pub note: String,
    pub name: String,
    pub sea: SeaSpec,
    #[serde(default)]
    pub rain: i32,
    #[serde(default)]
    pub clouds: Option<Vec<[i32; 4]>>,
}

impl PlaceSpec {
    /// How many weathers the place offers, its own included.
    pub fn weathers(&self) -> usize {
        1 + self.variants.len()
    }

    /// The place under weather `w`: 0 is its own; a variant swaps in its sea,
    /// its rain and its clouds.
    pub fn in_weather(&self, w: usize) -> PlaceSpec {
        let mut p = self.clone();
        if let Some(v) = w.checked_sub(1).and_then(|i| self.variants.get(i)) {
            p.sea = v.sea.clone();
            p.weather = v.name.clone();
            p.rain = v.rain;
            if let Some(c) = &v.clouds {
                p.scenery.clouds = c.clone();
            }
        }
        p.variants.clear();
        p
    }
}

/// What a place shows far off, placed by fixed numbers: sea stacks (x, z,
/// size in tenths) and clouds (x, y, z, size in tenths), in metres.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ScenerySpec {
    pub stacks: Vec<[i32; 3]>,
    pub clouds: Vec<[i32; 4]>,
}

/// Something the HUD reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Noticed(usize),
    Struck(usize),
    Bit(usize),
    Missed(usize),
    Spat(usize),
    HookSet(usize),
    Threw(usize),
    Jumped(usize),
    Snapped,
    Landed,
}

/// A landed fish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catch {
    pub species: usize,
    pub length: Fx,
    pub mass: Fx,
    pub fight_ticks: u32,
    pub kept: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    pub gear: GearSpec,
    pub place: PlaceSpec,
    /// Which of the gear's boats this is.
    pub boat: usize,
    pub sea: Sea,
    pub hull: Hull,
    pub arms: Arms,
    pub line: Line,
    pub fish: Vec<Fish>,
    pub rng: Rng,
    pub tick: u64,
    /// The fish with the lure in its mouth or hooked.
    pub holder: Option<usize>,
    /// The last quarter second of tension, for the hook set.
    tensions: Vec<Fx>,
    pub events: Vec<Event>,
    /// A fish just landed, waiting for release or keep.
    pub landed: Option<Catch>,
    pub catches: Vec<Catch>,
    pub fight_ticks: u32,
    lure_was_wet: bool,
    prev_end: V3,
    pub lure_vel: V3,
    /// Where and when the lure last struck the water, for the rings the page draws.
    pub last_splash: Option<(V3, u64)>,
    /// The motor's battery, ONE full.
    pub battery: Fx,
    /// Ticks E has been held with charge left; the jerks count from it.
    power_ticks: i32,
    /// True on a tick the motor pulled.
    pub pulling: bool,
    /// Ticks Space has been held.
    wind_ticks: u32,
}

impl World {
    pub fn new(gear: GearSpec, place: PlaceSpec, boat: usize, seed: u64) -> World {
        let sea = Sea::new(&place.sea);
        let heading = place.start_heading_deg * fx::TURN / 360;
        let boat = boat.min(gear.boats.len().saturating_sub(1));
        let spec = gear.boats[boat].hull.clone();
        let mut hull = Hull::new(spec.clone(), V3::ZERO, heading);
        hull.pos.y = -fx::div(
            hull.mass(),
            fx::mul(sea.density, spec.probe_area_cm2 * ONE / 10_000) * 16,
        );
        let arms = Arms::new(gear.arms.clone());
        let tip = hull.to_world(arms.rod().tip);
        let line = Line::new(gear.line.clone(), tip);
        let mut rng = Rng::new(seed);
        let mut fish = Vec::new();
        for (i, s) in place.fish.iter().enumerate() {
            for _ in 0..s.count {
                fish.push(Fish::spawn(i, s, V3::ZERO, 0, &mut rng));
            }
        }
        let n = gear.hook_set_ms * TICKS_PER_SECOND as i32 / 1000;
        let mut w = World {
            gear,
            place,
            boat,
            sea,
            hull,
            arms,
            line,
            fish,
            rng,
            tick: 0,
            holder: None,
            tensions: vec![0; n.max(1) as usize],
            events: Vec::new(),
            landed: None,
            catches: Vec::new(),
            fight_ticks: 0,
            lure_was_wet: false,
            prev_end: tip,
            lure_vel: V3::ZERO,
            last_splash: None,
            battery: ONE,
            power_ticks: 0,
            pulling: false,
            wind_ticks: 0,
        };
        for i in 0..w.fish.len() {
            w.settle(i);
        }
        w
    }

    /// True when this boat is fished from.
    pub fn fishing(&self) -> bool {
        self.gear.boats[self.boat].fishing
    }

    pub fn tip(&self) -> V3 {
        self.hull.to_world(self.arms.rod().tip)
    }

    /// The angler's place on the water, for landing and spooking fish.
    pub fn angler(&self) -> V3 {
        self.hull.to_world(v3(0, ONE / 2, -ONE / 3))
    }

    fn reset_line(&mut self) {
        self.line = Line::new(self.gear.line.clone(), self.tip());
        self.holder = None;
    }

    /// One tick with these key bits. Nothing moves while a landed fish waits.
    pub fn step(&mut self, keys: u32) {
        self.events.clear();
        if self.landed.is_some() {
            return;
        }
        self.tick += 1;
        // in a boat that is not fished from, Space snaps its throttle to neutral
        let neutral = !self.fishing() && keys & bits::WIND != 0;
        // from a boat that is not fished from, the arm and reel keys do nothing
        let keys = if self.fishing() {
            keys
        } else {
            keys & !(bits::ARMS | bits::FINGER | bits::WIND | bits::POWER_WIND | bits::LET_OUT)
        };
        let on = |b: u32| keys & b != 0;
        let both = |up: u32, down: u32| on(up) as i32 - on(down) as i32;
        use bits::*;
        let arm_input = ArmInput {
            // the left hand rests on the reel; the right arm casts (T12)
            drive: [
                0,
                0,
                both(SHOULDER_UP, SHOULDER_DOWN),
                both(ELBOW_BEND, ELBOW_STRAIGHTEN),
            ],
        };
        let hull_input = HullInput {
            pedal: both(PEDAL, BACK_PEDAL),
            drift: both(DRIFT_RIGHT, DRIFT_LEFT),
            neutral,
        };
        // the motor: jerks of `on_ms` with rests of `off_ms` between, while the
        // battery lasts; a fish on the line is fair game (T15)
        let p = &self.gear.power;
        let per_s = TICKS_PER_SECOND as i32;
        // a motor that ran flat starts again only with a tenth of a charge
        let can_run = self.battery > 0 && (self.power_ticks > 0 || self.battery >= ONE / 10);
        let running = on(POWER_WIND) && can_run;
        self.power_ticks = if running { self.power_ticks + 1 } else { 0 };
        let on_ticks = (p.on_ms * per_s / 1000).max(1);
        let period = on_ticks + (p.off_ms * per_s / 1000).max(0);
        self.pulling = running && (self.power_ticks - 1) % period < on_ticks;
        self.battery = if running {
            (self.battery - ONE / (p.battery_s.max(1) * per_s)).max(0)
        } else {
            (self.battery + ONE / (p.recharge_s.max(1) * per_s)).min(ONE)
        };
        // a tap of Space closes the bail and holds the line where it is; held
        // past a fifth of a second it winds (T24)
        self.wind_ticks = if on(WIND) { self.wind_ticks + 1 } else { 0 };
        let line_input = LineInput {
            finger: on(FINGER),
            close: on(WIND),
            wind: self.wind_ticks > TICKS_PER_SECOND as u32 / 5,
            power: self.pulling,
            let_out: on(LET_OUT),
        };
        self.sea.tick();
        self.arms.tick(arm_input);
        let tip = self.tip();
        // what hangs on the line: the lure, or the fish that holds it
        let end = match self.holder {
            Some(i) => {
                let f = &self.fish[i];
                let scene = self.around(i, self.scene(false));
                let spec = self.place.fish[f.species].clone();
                let (said, force) = self.fish[i].pull(&spec, &scene, &mut self.rng);
                self.say(i, said);
                let f = &self.fish[i];
                EndLoad {
                    mass: f.mass,
                    volume: fx::div(f.mass, ONE * 1025 / 1000),
                    // ½ C_d A for a fish towed head first: about a thousandth of its length squared
                    area: fx::mul(fx::mul(f.length, f.length), ONE / 1000),
                    force,
                }
            }
            None => self.line.lure(),
        };
        let pull = self.line.tick(&self.sea, tip, line_input, end);
        self.hull.tick(&self.sea, hull_input, pull);
        let e = self.line.end();
        self.lure_vel = e.sub(self.prev_end).scale(TICKS_PER_SECOND as i32 * ONE);
        self.prev_end = e;
        // the tension over the last quarter second, for the hook set
        let slot = (self.tick % self.tensions.len() as u64) as usize;
        let low = *self.tensions.iter().min().unwrap_or(&0);
        self.tensions[slot] = self.line.tension;
        let set = self.line.tension - low > self.gear.hook_set_dn * ONE / 10;
        if self.line.snapped {
            if let Some(i) = self.holder {
                self.fish[i].mood = Mood::Flee;
                self.fish[i].timer = 0;
            }
            self.events.push(Event::Snapped);
            self.reset_line();
            return;
        }
        // the holder: moved by the line, and hooked by a hook set while it holds
        if let Some(i) = self.holder {
            let f = &mut self.fish[i];
            f.pos = e.sub(f.facing().scale(f.length / 2));
            f.vel = self.lure_vel;
            if f.vel.flat_len() > ONE / 20 && f.mood == Mood::Hooked {
                f.heading = crate::geom::yaw_of(-f.vel.x, -f.vel.z);
            }
            match f.mood {
                Mood::Hold if set => {
                    f.mood = Mood::Hooked;
                    f.timer = 0;
                    self.fight_ticks = 0;
                    self.events.push(Event::HookSet(i));
                }
                Mood::Hold | Mood::Hooked => {}
                _ => self.holder = None,
            }
            if self.fish[i].mood == Mood::Hooked {
                self.fight_ticks += 1;
                self.try_land(i);
            }
        }
        // the free fish
        let scene = self.scene(true);
        for i in 0..self.fish.len() {
            if Some(i) == self.holder {
                continue;
            }
            let spec = self.place.fish[self.fish[i].species].clone();
            let here = self.around(i, scene);
            let said = self.fish[i].tick(&spec, &here, &mut self.rng);
            self.keep_in_water(i);
            self.say(i, said);
            if said == Some(Happened::Bit) && self.holder.is_none() {
                self.holder = Some(i);
            } else if said == Some(Happened::Bit) {
                // the lure is taken already: this one comes away with nothing
                self.fish[i].mood = Mood::Flee;
            }
            // a fish left far behind by the kayak is met again ahead
            if self.fish[i].pos.sub(self.hull.pos).flat_len() > 120 * ONE {
                let s = self.fish[i].species;
                let surface = self.sea.level();
                self.fish[i] = Fish::spawn(s, &self.place.fish[s], self.hull.pos, surface, &mut self.rng);
                self.settle(i);
            }
        }
        if scene.splash {
            self.last_splash = Some((scene.lure, self.tick));
        }
        self.lure_was_wet = scene.lure_wet;
    }

    fn scene(&self, with_splash: bool) -> Scene {
        let e = self.line.end();
        let surface = self.sea.height(e.x, e.z);
        // a lure already in a fish's mouth is no lure to the others
        let wet = e.y < surface && !(with_splash && self.holder.is_some());
        Scene {
            lure: e,
            lure_vel: self.lure_vel,
            lure_wet: wet,
            splash: with_splash && wet && !self.lure_was_wet,
            water: self.sea.current_at(e.x, e.z),
            here: V3::ZERO,
            hold: None,
            surface: self.sea.level(),
            seabed: self.sea.bed_at(e.x, e.z),
            kayak: self.angler(),
            tension: self.line.tension,
        }
    }

    /// What fish `i` sees: the shared scene, with the current and the bottom
    /// where it swims, and its eddy if it holds behind a rock.
    fn around(&self, i: usize, scene: Scene) -> Scene {
        let p = self.fish[i].pos;
        Scene {
            here: self.sea.current_at(p.x, p.z),
            seabed: self.sea.bed_at(p.x, p.z),
            hold: self.hold_for(i),
            ..scene
        }
    }

    /// The slack water behind a rock that fish `i` holds in: two of the
    /// rock's radii downstream of its middle. Fish spread over the rocks.
    fn hold_for(&self, i: usize) -> Option<V3> {
        let f = &self.fish[i];
        if !self.place.fish[f.species].holds_by_rocks {
            return None;
        }
        let rocks = &self.sea.river.as_ref()?.rocks;
        if rocks.is_empty() {
            return None;
        }
        let k = &rocks[(i * 5 + 3) % rocks.len()];
        Some(k.at.add(k.flow.scale(k.r * 2)))
    }

    /// Keeps fish `i` in the water: off the banks, out of the rocks.
    fn keep_in_water(&mut self, i: usize) {
        let push = self.sea.push_out(self.fish[i].pos, ONE * 3 / 10);
        self.fish[i].pos = self.fish[i].pos.add(push);
    }

    /// A fish just spawned starts at its eddy, or in the water.
    fn settle(&mut self, i: usize) {
        if let Some(h) = self.hold_for(i) {
            let y = self.fish[i].pos.y.max(self.sea.bed_at(h.x, h.z) + ONE / 3);
            self.fish[i].pos = v3(h.x, y, h.z);
        }
        self.keep_in_water(i);
        let p = self.fish[i].pos;
        self.fish[i].pos.y = p.y.max(self.sea.bed_at(p.x, p.z) + ONE / 3);
    }

    fn say(&mut self, i: usize, said: Option<Happened>) {
        let e = match said {
            Some(Happened::Noticed) => Event::Noticed(i),
            Some(Happened::Struck) => Event::Struck(i),
            Some(Happened::Bit) => Event::Bit(i),
            Some(Happened::Missed) => Event::Missed(i),
            Some(Happened::Spat) => Event::Spat(i),
            Some(Happened::Threw) => Event::Threw(i),
            Some(Happened::Jumped) => Event::Jumped(i),
            None => return,
        };
        self.events.push(e);
    }

    fn try_land(&mut self, i: usize) {
        let f = &self.fish[i];
        let reach = self.gear.land_reach_cm * ONE / 100;
        let near = f.pos.sub(self.angler()).flat_len() < reach;
        let short = self.line.out < self.gear.land_line_cm * ONE / 100;
        let up = f.pos.y > self.sea.level() - 3 * ONE / 2;
        // a fresh fish cannot be lifted in, however close it bites
        let spent = f.stamina < ONE / 4;
        if near && short && up && spent {
            self.landed = Some(Catch {
                species: f.species,
                length: f.length,
                mass: f.mass,
                fight_ticks: self.fight_ticks,
                kept: false,
            });
            self.events.push(Event::Landed);
        }
    }

    /// Lets the landed fish go, or keeps it, and fishes on.
    pub fn finish_catch(&mut self, keep: bool) {
        let Some(mut c) = self.landed.take() else {
            return;
        };
        c.kept = keep;
        if let Some(i) = self.holder {
            let s = self.fish[i].species;
            let surface = self.sea.level();
            self.fish[i] = Fish::spawn(s, &self.place.fish[s], self.hull.pos, surface, &mut self.rng);
            self.settle(i);
        }
        self.catches.push(c);
        self.reset_line();
    }

    /// A number that changes when anything in the world does.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: i32| {
            h ^= v as u32 as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for p in &self.line.pts {
            eat(p.x);
            eat(p.y);
            eat(p.z);
        }
        for f in &self.fish {
            eat(f.pos.x);
            eat(f.pos.y);
            eat(f.pos.z);
            eat(f.mood as i32);
        }
        eat(self.hull.pos.x);
        eat(self.hull.pos.y);
        eat(self.hull.pos.z);
        eat(self.hull.heading);
        eat(self.hull.pitch);
        eat(self.hull.roll);
        for a in self.arms.angle {
            eat(a);
        }
        eat(self.line.out);
        eat(self.tick as i32);
        h
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn world(seed: u64) -> World {
        let gear: GearSpec = serde_json::from_str(include_str!("../../../data/gear.json")).unwrap();
        let places: Vec<PlaceSpec> = serde_json::from_str(include_str!("../../../data/places.json")).unwrap();
        World::new(gear, places[0].clone(), 0, seed)
    }

    /// A world at place `place` (0 the Open Swell, 1 the Ledge River) in boat
    /// `boat` (0 the kayak, 1 the speedboat).
    pub fn world_at(place: usize, boat: usize, seed: u64) -> World {
        let gear: GearSpec = serde_json::from_str(include_str!("../../../data/gear.json")).unwrap();
        let places: Vec<PlaceSpec> = serde_json::from_str(include_str!("../../../data/places.json")).unwrap();
        World::new(gear, places[place].clone(), boat, seed)
    }

    #[test]
    fn a_kayak_left_alone_drifts_down_the_river_and_stays_off_the_banks() {
        let mut w = world_at(1, 0, 1);
        let r = w.sea.river.clone().unwrap();
        let start = w.hull.pos;
        for t in 0..3600 {
            w.step(0);
            let s = r.across(w.hull.pos).abs();
            assert!(s < ONE, "on the bank at tick {t}: {s}");
        }
        // 1.23 m/s mid-channel for a minute is 74 m; a kayak gathering speed
        // and slowing by the rocks goes less, but well past 20 m
        let gone = w.hull.pos.sub(start).flat_len();
        assert!(gone > 20 * ONE, "drifted {gone}");
        assert!(w.hull.pos.z < start.z, "downstream is toward -z at the start");
    }

    #[test]
    fn a_kayak_pedalled_at_the_bank_stops_at_it() {
        let mut w = world_at(1, 0, 1);
        let r = w.sea.river.clone().unwrap();
        // turn to face the outer bank (+x), then pedal at it for half a minute
        w.hull.heading = fx::TURN / 4;
        for _ in 0..1800 {
            w.step(bits::PEDAL);
            assert!(r.across(w.hull.pos) < ONE);
        }
        // the middle stops half a beam (0.43 m) and 0.2 m short of the 20 m
        // bank, and the bow 0.2 m short: across ends near (20 − 1.95 − 0.2) / 20
        let s = r.across(w.hull.pos);
        assert!(s > ONE * 85 / 100, "stopped at {s}");
    }

    #[test]
    fn trout_hold_in_the_slack_water_behind_their_rocks() {
        let mut w = world_at(1, 0, 2);
        for _ in 0..1800 {
            w.step(0);
        }
        let mut held = 0;
        for i in 0..w.fish.len() {
            if let (Some(h), Mood::Cruise) = (w.hold_for(i), w.fish[i].mood) {
                if w.fish[i].pos.sub(h).flat_len() < 3 * ONE {
                    held += 1;
                }
            }
        }
        // eighteen rainbow trout in data/places.json; one may be off
        // following a lure or fleeing the kayak
        assert!(held >= 16, "{held} of 18 trout at their rocks");
    }

    #[test]
    fn the_speedboat_planes_past_fifteen_metres_a_second() {
        // 5200 N against 16 N per (m/s)² gives √(5200/16) = 18.0 m/s with the
        // whole hull in the water. Planing, part of the hull is out and drags
        // less, so it runs faster: with a quarter of the probes wet, no faster
        // than √(5200/16 × 4) = 36 m/s. Up holds the throttle lever forward:
        // full after two seconds at 50% a second
        let mut w = world_at(0, 1, 1);
        for _ in 0..900 {
            w.step(bits::PEDAL);
        }
        assert_eq!(w.hull.throttle, ONE);
        let v = w.hull.speed();
        assert!(v > 15 * ONE && v < 36 * ONE, "speed {v}");
    }

    #[test]
    fn the_speedboat_leaves_the_water_off_the_swell() {
        let mut w = world_at(0, 1, 1);
        let mut air = 0;
        for _ in 0..3600 {
            w.step(bits::PEDAL);
            let dry = w.hull.probes().iter().all(|p| {
                let q = w.hull.to_world(*p);
                q.y > w.sea.height(q.x, q.z)
            });
            if dry {
                air += 1;
            }
        }
        assert!(air > 0, "never left the water in a minute at full throttle");
    }

    #[test]
    fn the_speedboat_is_not_fished_from() {
        let mut w = world_at(0, 1, 1);
        let arms = w.arms.angle;
        let out = w.line.out;
        for _ in 0..60 {
            w.step(bits::SHOULDER_UP | bits::FINGER | bits::POWER_WIND);
        }
        assert_eq!(w.arms.angle, arms);
        assert_eq!(w.line.out, out);
        assert_eq!(w.battery, ONE);
    }

    /// A minute of scripted play: pedal, drift, swing the rod, let line go, wind.
    fn script(t: u32) -> u32 {
        use bits::*;
        match t % 600 {
            0..=59 => PEDAL,
            60..=89 => DRIFT_LEFT | FINGER,
            90..=119 => SHOULDER_UP | FINGER,
            120..=135 => SHOULDER_DOWN | FINGER,
            136..=299 => 0,
            _ => WIND,
        }
    }

    #[test]
    fn the_same_seed_and_keys_end_on_the_same_checksum() {
        let run = |seed: u64| {
            let mut w = world(seed);
            for t in 0..3600 {
                w.step(script(t));
            }
            w.checksum()
        };
        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8), "another seed places the fish elsewhere");
    }

    #[test]
    fn only_the_arm_keys_move_the_rod() {
        use bits::*;
        for key in [
            FINGER,
            WIND,
            PEDAL,
            BACK_PEDAL,
            DRIFT_LEFT,
            DRIFT_RIGHT,
            SCOPE,
            POWER_WIND,
        ] {
            let mut w = world(1);
            let before = (w.arms.angle, w.arms.rod());
            for _ in 0..60 {
                w.step(key);
            }
            assert_eq!(
                (w.arms.angle, w.arms.rod()),
                before,
                "key bit {key} moved the rod"
            );
        }
        for (name, key) in bits::NAMES.iter().take(4) {
            let mut w = world(1);
            let before = w.arms.angle;
            for _ in 0..20 {
                w.step(*key);
            }
            assert_ne!(w.arms.angle, before, "{name} did not move its joint");
        }
        assert_eq!(bits::ARMS, bits::NAMES.iter().take(4).fold(0, |a, (_, b)| a | b));
    }

    #[test]
    fn q_lets_line_out_at_the_kayak_before_a_cast() {
        // the reel's own speed backwards: 0.8 m a turn at 2.5 turns a second
        // is 2 m a second, so a second of Q takes 1.2 m of line out to 3.2 m
        let mut w = world(1);
        for _ in 0..60 {
            w.step(bits::LET_OUT);
        }
        assert!(!w.line.bail_open);
        assert!(
            (w.line.out - ONE * 32 / 10).abs() < ONE / 50,
            "out {}",
            w.line.out
        );
    }

    #[test]
    fn a_tap_of_space_closes_the_bail_and_holds_the_line() {
        // after a cast the bail is open; a tenth of a second of Space closes
        // it without winding, and the line out then stays put (T24)
        use bits::*;
        let mut w = world(1);
        for t in 0..300u32 {
            let k = match t {
                0..=28 => FINGER | SHOULDER_UP,
                29..=37 => FINGER | SHOULDER_DOWN,
                38..=51 => SHOULDER_DOWN,
                _ => 0,
            };
            w.step(k);
        }
        assert!(w.line.bail_open);
        let out = w.line.out;
        for _ in 0..6 {
            w.step(WIND);
        }
        assert!(!w.line.bail_open);
        for _ in 0..120 {
            w.step(0);
        }
        assert!(
            (w.line.out - out).abs() < ONE / 100,
            "out {} from {out}",
            w.line.out
        );
        // held, Space winds
        for _ in 0..60 {
            w.step(WIND);
        }
        assert!(w.line.out < out - ONE, "out {} from {out}", w.line.out);
    }

    #[test]
    fn a_cast_pays_out_no_more_line_than_the_lure_has_gone() {
        // cast, then leave the bail open for ten seconds on the swell: the
        // line out may exceed the tip-to-lure distance by the 20% allowance
        // in data/gear.json (free_slack_pct), and by no more (T10)
        use bits::*;
        let mut w = world(1);
        for t in 0..660u32 {
            let k = match t {
                0..=28 => FINGER | SHOULDER_UP,
                29..=37 => FINGER | SHOULDER_DOWN,
                38..=51 => SHOULDER_DOWN,
                _ => 0,
            };
            w.step(k);
        }
        assert!(w.line.bail_open);
        let straight = w.line.end().sub(w.tip()).len();
        assert!(straight > 2 * ONE, "the lure went out: {straight}");
        assert!(
            w.line.out <= straight * 12 / 10 + ONE / 10,
            "line out {} for a lure {} away",
            w.line.out,
            straight
        );
    }

    /// A world where fish 0 holds the lure at the end of a short line.
    fn held(mood: Mood) -> World {
        let mut w = world(3);
        for _ in 0..60 {
            w.step(0);
        }
        let e = w.line.end();
        let f = &mut w.fish[0];
        f.pos = e.sub(f.facing().scale(f.length / 2));
        f.vel = V3::ZERO;
        f.mood = mood;
        f.timer = 0;
        if mood == Mood::Hold {
            w.holder = Some(0);
        }
        w
    }

    #[test]
    fn a_hook_set_while_a_fish_holds_the_lure_hooks_it() {
        let mut w = held(Mood::Hold);
        // sweep the rod up hard: the tip leaps, the line jerks taut
        for _ in 0..15 {
            w.step(bits::SHOULDER_UP);
        }
        assert_eq!(w.fish[0].mood, Mood::Hooked, "tension {}", w.line.tension);
    }

    #[test]
    fn a_hook_set_with_no_fish_holding_the_lure_does_nothing() {
        let mut w = held(Mood::Cruise);
        for _ in 0..15 {
            w.step(bits::SHOULDER_UP);
        }
        assert!(w.fish.iter().all(|f| f.mood != Mood::Hooked));
        assert_eq!(w.holder, None);
    }

    #[test]
    fn the_power_wind_jerks_line_in() {
        // jerks of 150 ms (9 ticks) with 100 ms (6 ticks) rests: half a second
        // is two whole jerks, 18 ticks pulling at 7 m/s, so 0.3 s × 7 = 2.1 m
        let mut w = held(Mood::Cruise);
        w.line.out = 10 * ONE;
        let mut pulls = Vec::new();
        for _ in 0..30 {
            w.step(bits::POWER_WIND);
            pulls.push(w.pulling);
        }
        let wound = 10 * ONE - w.line.out;
        assert!((wound - 21 * ONE / 10).abs() < ONE / 10, "wound {wound}");
        assert_eq!(pulls.iter().filter(|p| **p).count(), 18);
        assert!(pulls[0] && !pulls[9] && pulls[15], "{pulls:?}");
    }

    #[test]
    fn the_battery_runs_eight_seconds_and_refills_in_forty_five() {
        let mut w = held(Mood::Cruise);
        w.line.out = 100 * ONE;
        // 8 s of running is 480 ticks
        for _ in 0..470 {
            w.step(bits::POWER_WIND);
        }
        assert!(w.battery > 0, "charge left at 470 ticks");
        for _ in 0..20 {
            w.step(bits::POWER_WIND);
        }
        // flat at 480 ticks; the ten ticks since put a little back, which is
        // under the tenth the motor needs to start again
        assert!(w.battery < ONE / 100, "battery {}", w.battery);
        let out = w.line.out;
        for _ in 0..30 {
            w.step(bits::POWER_WIND);
        }
        assert_eq!(w.line.out, out, "a flat battery winds nothing");
        // half the recharge, 22.5 s, brings back about half
        for _ in 0..1350 {
            w.step(0);
        }
        assert!((w.battery - ONE / 2).abs() < ONE / 50, "battery {}", w.battery);
    }

    #[test]
    fn a_power_wind_hauls_a_spent_fish_in_to_be_landed() {
        // a tired fish 6 m out: the motor's jerks lift it to the angler (T15)
        let mut w = held(Mood::Hold);
        w.fish[0].mood = Mood::Hooked;
        w.fish[0].stamina = 0;
        w.line.out = 6 * ONE;
        let mut landed = None;
        for t in 0..300 {
            w.step(bits::POWER_WIND);
            assert!(!w.events.contains(&Event::Snapped), "snapped at {t}");
            if w.landed.is_some() {
                landed = Some(t);
                break;
            }
        }
        assert!(landed.is_some(), "line out {}", w.line.out);
    }

    #[test]
    fn a_fresh_fish_in_reach_is_not_landed() {
        // the fish is on 1.2 m of line under the rod: in reach, but full of fight
        let mut w = held(Mood::Hold);
        w.fish[0].mood = Mood::Hooked;
        for _ in 0..30 {
            w.step(0);
        }
        assert!(w.landed.is_none());
    }

    #[test]
    fn a_spent_fish_brought_in_close_is_landed() {
        let mut w = held(Mood::Hold);
        w.fish[0].mood = Mood::Hooked;
        w.fish[0].stamina = 0;
        for _ in 0..120 {
            w.step(bits::WIND);
            if w.landed.is_some() {
                break;
            }
        }
        let c = w.landed.clone().expect("landed");
        assert_eq!(c.species, w.fish[0].species);
        w.finish_catch(false);
        assert_eq!(w.catches.len(), 1);
        assert_eq!(w.holder, None);
    }
}
