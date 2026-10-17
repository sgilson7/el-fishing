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
    pub const LEFT_SHOULDER_UP: u32 = 1;
    pub const LEFT_SHOULDER_DOWN: u32 = 1 << 1;
    pub const LEFT_ELBOW_BEND: u32 = 1 << 2;
    pub const LEFT_ELBOW_STRAIGHTEN: u32 = 1 << 3;
    pub const RIGHT_SHOULDER_UP: u32 = 1 << 4;
    pub const RIGHT_SHOULDER_DOWN: u32 = 1 << 5;
    pub const RIGHT_ELBOW_BEND: u32 = 1 << 6;
    pub const RIGHT_ELBOW_STRAIGHTEN: u32 = 1 << 7;
    pub const FINGER: u32 = 1 << 8;
    pub const WIND: u32 = 1 << 9;
    pub const PEDAL: u32 = 1 << 10;
    pub const BACK_PEDAL: u32 = 1 << 11;
    pub const DRIFT_LEFT: u32 = 1 << 12;
    pub const DRIFT_RIGHT: u32 = 1 << 13;
    pub const SCOPE: u32 = 1 << 14;
    /// The eight keys that move the arms.
    pub const ARMS: u32 = 0xff;
    pub const NAMES: [(&str, u32); 15] = [
        ("left_shoulder_up", LEFT_SHOULDER_UP),
        ("left_shoulder_down", LEFT_SHOULDER_DOWN),
        ("left_elbow_bend", LEFT_ELBOW_BEND),
        ("left_elbow_straighten", LEFT_ELBOW_STRAIGHTEN),
        ("right_shoulder_up", RIGHT_SHOULDER_UP),
        ("right_shoulder_down", RIGHT_SHOULDER_DOWN),
        ("right_elbow_bend", RIGHT_ELBOW_BEND),
        ("right_elbow_straighten", RIGHT_ELBOW_STRAIGHTEN),
        ("finger", FINGER),
        ("wind", WIND),
        ("pedal", PEDAL),
        ("back_pedal", BACK_PEDAL),
        ("drift_left", DRIFT_LEFT),
        ("drift_right", DRIFT_RIGHT),
        ("scope", SCOPE),
    ];
}

/// The angler's gear, from `data/gear.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearSpec {
    /// A note for people; the core does not read it.
    #[serde(default, rename = "_note")]
    pub note: String,
    pub hull: HullSpec,
    pub arms: ArmSpec,
    pub line: LineSpec,
    /// A hook set: the tension rising this much (tenths of N) within this long (ms).
    pub hook_set_dn: i32,
    pub hook_set_ms: i32,
    /// Landing: the fish within this reach of the angler with less line out.
    pub land_reach_cm: i32,
    pub land_line_cm: i32,
}

/// A place, from `data/place.json`.
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
}

impl World {
    pub fn new(gear: GearSpec, place: PlaceSpec, seed: u64) -> World {
        let sea = Sea::new(&place.sea);
        let heading = place.start_heading_deg * fx::TURN / 360;
        let mut hull = Hull::new(gear.hull.clone(), V3::ZERO, heading);
        hull.pos.y = -fx::div(
            hull.mass(),
            fx::mul(sea.density, gear.hull.probe_area_cm2 * ONE / 10_000) * 16,
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
        World {
            gear,
            place,
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
        }
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
        let on = |b: u32| keys & b != 0;
        let both = |up: u32, down: u32| on(up) as i32 - on(down) as i32;
        use bits::*;
        let arm_input = ArmInput {
            drive: [
                both(LEFT_SHOULDER_UP, LEFT_SHOULDER_DOWN),
                both(LEFT_ELBOW_BEND, LEFT_ELBOW_STRAIGHTEN),
                both(RIGHT_SHOULDER_UP, RIGHT_SHOULDER_DOWN),
                both(RIGHT_ELBOW_BEND, RIGHT_ELBOW_STRAIGHTEN),
            ],
        };
        let hull_input = HullInput {
            pedal: both(PEDAL, BACK_PEDAL),
            drift: both(DRIFT_RIGHT, DRIFT_LEFT),
        };
        let line_input = LineInput {
            finger: on(FINGER),
            wind: on(WIND),
        };
        self.sea.tick();
        self.arms.tick(arm_input);
        let tip = self.tip();
        // what hangs on the line: the lure, or the fish that holds it
        let end = match self.holder {
            Some(i) => {
                let f = &self.fish[i];
                let scene = self.scene(false);
                let spec = self.place.fish[f.species].clone();
                let (said, force) = self.fish[i].pull(&spec, &scene, &mut self.rng);
                self.say(i, said);
                let f = &self.fish[i];
                EndLoad {
                    mass: f.mass,
                    volume: fx::div(f.mass, ONE * 1025 / 1000),
                    area: fx::mul(fx::mul(f.length, f.length), ONE / 100),
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
            let said = self.fish[i].tick(&spec, &scene, &mut self.rng);
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
            }
        }
        self.lure_was_wet = scene.lure_wet;
    }

    fn scene(&self, with_splash: bool) -> Scene {
        let e = self.line.end();
        let surface = self.sea.height(e.x, e.z);
        let wet = e.y < surface;
        Scene {
            lure: e,
            lure_vel: self.lure_vel,
            lure_wet: wet,
            splash: with_splash && wet && !self.lure_was_wet,
            water: self.sea.water_velocity(),
            surface: self.sea.level(),
            seabed: -self.sea.depth,
            kayak: self.angler(),
            tension: self.line.tension,
        }
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
        let place: PlaceSpec = serde_json::from_str(include_str!("../../../data/place.json")).unwrap();
        World::new(gear, place, seed)
    }

    /// A minute of scripted play: pedal, drift, swing the rod, let line go, wind.
    fn script(t: u32) -> u32 {
        use bits::*;
        match t % 600 {
            0..=59 => PEDAL,
            60..=89 => DRIFT_LEFT | FINGER,
            90..=119 => RIGHT_SHOULDER_UP | FINGER,
            120..=135 => RIGHT_SHOULDER_DOWN | FINGER,
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
        for key in [FINGER, WIND, PEDAL, BACK_PEDAL, DRIFT_LEFT, DRIFT_RIGHT, SCOPE] {
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
        for (name, key) in bits::NAMES.iter().take(8) {
            let mut w = world(1);
            let before = w.arms.angle;
            for _ in 0..20 {
                w.step(*key);
            }
            assert_ne!(w.arms.angle, before, "{name} did not move its joint");
        }
        assert_eq!(bits::ARMS, bits::NAMES.iter().take(8).fold(0, |a, (_, b)| a | b));
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
            w.step(bits::RIGHT_SHOULDER_UP | bits::LEFT_SHOULDER_UP);
        }
        assert_eq!(w.fish[0].mood, Mood::Hooked, "tension {}", w.line.tension);
    }

    #[test]
    fn a_hook_set_with_no_fish_holding_the_lure_does_nothing() {
        let mut w = held(Mood::Cruise);
        for _ in 0..15 {
            w.step(bits::RIGHT_SHOULDER_UP | bits::LEFT_SHOULDER_UP);
        }
        assert!(w.fish.iter().all(|f| f.mood != Mood::Hooked));
        assert_eq!(w.holder, None);
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
