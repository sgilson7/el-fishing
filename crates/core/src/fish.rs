//! Fish: agents that cruise, notice a lure that moves the way they like,
//! follow it, strike, hold it in their mouth for a moment, and spit it out
//! unless the hook is set. A bite counts only while the hook is inside the
//! fish's mouth zone, about one gape from the mouth (Higham, Day and
//! Wainwright 2006: suction reaches about one gape). A hooked fish pulls
//! with its strength times its stamina, which its own work drains.

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{self, v3, V3};
use crate::rng::Rng;
use crate::ticks::TICKS_PER_SECOND;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishSpec {
    /// The key of the species' name in the strings file.
    pub name: String,
    pub count: i32,
    pub length_cm: [i32; 2],
    /// Weight at the shortest length; weight grows as the cube of length.
    pub grams_at_shortest: i32,
    /// Depth below the surface the fish keeps to, cm.
    pub depth_cm: [i32; 2],
    pub cruise_mm_s: i32,
    pub burst_mm_s: i32,
    pub sense_cm: i32,
    /// A lure's splash draws the fish from this far.
    pub splash_cm: i32,
    pub gape_mm: i32,
    /// The lure speeds through the water that hold its interest, mm/s.
    pub lure_speed_mm_s: [i32; 2],
    /// Interest gained a second while the lure moves well; a strike comes at 100.
    pub interest_per_s: i32,
    /// How long it holds a bitten lure before it spits it out, ms.
    pub hold_ms: i32,
    /// Pull at full stamina, N.
    pub strength_n: i32,
    /// Seconds of full pull before it tires.
    pub stamina_s: i32,
    pub jumps: bool,
    /// Body colours, top to belly, and the fins; the pattern by name.
    pub colours: [String; 4],
    pub pattern: String,
    /// Body depth and width as a share of length, percent.
    pub depth_pct: i32,
    pub width_pct: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Cruise,
    Follow,
    Strike,
    Hold,
    Hooked,
    Flee,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fish {
    pub species: usize,
    pub pos: V3,
    pub vel: V3,
    /// Compass heading, turns.
    pub heading: i32,
    pub mood: Mood,
    /// Ticks in the current mood.
    pub timer: i32,
    /// 0 to 100 × ONE.
    pub interest: Fx,
    pub length: Fx,
    pub mass: Fx,
    /// ONE is fresh, 0 is spent.
    pub stamina: Fx,
    /// 0 shut to ONE open.
    pub mouth: Fx,
    pub misses: i32,
    /// Where a strike aims: the lure's place when the fish committed, led by its speed.
    pub aim: V3,
    /// Ticks the line has been slack while hooked.
    pub slack: i32,
    /// A wander target, re-drawn now and then.
    pub goal: V3,
}

/// What a fish sees of the lure and the kayak this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scene {
    pub lure: V3,
    pub lure_vel: V3,
    pub lure_wet: bool,
    /// A splash this tick (the lure landing), at the lure.
    pub splash: bool,
    pub water: V3,
    pub surface: Fx,
    pub seabed: Fx,
    pub kayak: V3,
    pub tension: Fx,
}

/// What a fish did this tick that the game must hear about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Happened {
    Noticed,
    Struck,
    Bit,
    Missed,
    Spat,
    Threw,
    Jumped,
}

fn mm(v: i32) -> Fx {
    v * ONE / 1000
}

fn cm(v: i32) -> Fx {
    v * ONE / 100
}

pub fn per_tick(v: Fx) -> Fx {
    v / TICKS_PER_SECOND as i32
}

impl Fish {
    /// A fish of a random size at a random place in a ring round `centre`.
    pub fn spawn(species: usize, spec: &FishSpec, centre: V3, surface: Fx, rng: &mut Rng) -> Fish {
        let length = cm(rng.between(spec.length_cm[0], spec.length_cm[1].max(spec.length_cm[0])));
        let shortest = cm(spec.length_cm[0].max(1));
        let r = fx::div(length, shortest);
        let mass = fx::mul(spec.grams_at_shortest * ONE / 1000, fx::mul(r, fx::mul(r, r)));
        let a = rng.below(fx::TURN as u32) as i32;
        let dist = cm(rng.between(1500, 4500));
        let depth = cm(rng.between(spec.depth_cm[0], spec.depth_cm[1].max(spec.depth_cm[0])));
        let pos = centre.add(v3(fx::mul(fx::sin(a), dist), 0, fx::mul(fx::cos(a), dist)));
        let pos = v3(pos.x, surface - depth, pos.z);
        Fish {
            species,
            pos,
            vel: V3::ZERO,
            heading: rng.below(fx::TURN as u32) as i32,
            mood: Mood::Cruise,
            timer: 0,
            interest: 0,
            length,
            mass,
            stamina: ONE,
            mouth: 0,
            misses: 0,
            aim: pos,
            slack: 0,
            goal: pos,
        }
    }

    pub fn forward(&self) -> V3 {
        v3(fx::sin(self.heading), 0, fx::cos(self.heading))
    }

    /// The middle of the mouth.
    pub fn mouth_at(&self) -> V3 {
        self.pos.add(self.facing().scale(self.length / 2))
    }

    /// Where the fish faces, with its climb or dive.
    pub fn facing(&self) -> V3 {
        let flat = self.forward();
        let speed = self.vel.flat_len();
        if speed < ONE / 20 {
            return flat;
        }
        flat.add(v3(0, fx::div(self.vel.y, speed).clamp(-ONE / 2, ONE / 2), 0))
            .with_len(ONE)
    }

    /// The mouth zone's reach: one gape.
    pub fn zone(&self, spec: &FishSpec) -> Fx {
        mm(spec.gape_mm)
    }

    /// True when the hook is inside the mouth zone.
    pub fn in_zone(&self, spec: &FishSpec, hook: V3) -> bool {
        hook.sub(self.mouth_at()).len() <= self.zone(spec)
    }

    fn mood(&mut self, m: Mood) {
        self.mood = m;
        self.timer = 0;
    }

    /// Steers toward a velocity, as fast as a fish of this kind can turn and speed up.
    fn steer(&mut self, spec: &FishSpec, want: V3) {
        let accel = per_tick(mm(spec.burst_mm_s) * 2);
        let dv = want.sub(self.vel);
        let step = if dv.len() > accel { dv.with_len(accel) } else { dv };
        self.vel = self.vel.add(step);
        self.pos = self.pos.add(per_tick_v(self.vel));
        if self.vel.flat_len() > ONE / 50 {
            self.heading = geom::yaw_of(self.vel.x, self.vel.z);
        }
    }

    /// One tick for a free fish (not holding the lure, not hooked).
    pub fn tick(&mut self, spec: &FishSpec, s: &Scene, rng: &mut Rng) -> Option<Happened> {
        self.timer += 1;
        let cruise = mm(spec.cruise_mm_s);
        let burst = mm(spec.burst_mm_s);
        let to_lure = s.lure.sub(self.mouth_at());
        let lure_dist = to_lure.len();
        let lure_speed = s.lure_vel.sub(s.water).len();
        let liked = lure_speed >= mm(spec.lure_speed_mm_s[0]) && lure_speed <= mm(spec.lure_speed_mm_s[1]);
        let kayak_close = s.kayak.sub(self.pos).flat_len() < 3 * ONE && self.pos.y > s.surface - 2 * ONE;
        let depth_lo = s.surface - cm(spec.depth_cm[1]);
        let depth_hi = s.surface - cm(spec.depth_cm[0]);
        let mut said = None;
        match self.mood {
            Mood::Cruise => {
                if self.timer % 180 == 1 || self.goal.sub(self.pos).len() < ONE {
                    // a new place to wander to, within 40 m of the kayak
                    let a = rng.below(fx::TURN as u32) as i32;
                    let r = cm(rng.between(800, 4000));
                    let y = depth_lo + fx::mul(depth_hi - depth_lo, rng.below(ONE as u32) as i32);
                    self.goal = v3(
                        s.kayak.x + fx::mul(fx::sin(a), r),
                        y,
                        s.kayak.z + fx::mul(fx::cos(a), r),
                    );
                }
                let want = self.goal.sub(self.pos).with_len(cruise);
                self.steer(spec, want);
                let heard = s.splash && lure_dist < cm(spec.splash_cm);
                if s.lure_wet && ((lure_dist < cm(spec.sense_cm) && liked) || heard) {
                    self.mood(Mood::Follow);
                    self.interest = if heard { 30 * ONE } else { 0 };
                    said = Some(Happened::Noticed);
                }
            }
            Mood::Follow => {
                // hold station a body length behind the lure, matching its speed
                let behind = s.lure.sub(to_lure.with_len(self.length));
                let gap = behind.sub(self.mouth_at());
                let want = s.lure_vel.add(gap.scale(ONE * 3 / 2)).flat().add(v3(0, gap.y, 0));
                let want = if want.len() > burst {
                    want.with_len(burst)
                } else {
                    want
                };
                self.steer(spec, want);
                let rate = per_tick(spec.interest_per_s * ONE);
                if liked && s.lure_wet {
                    self.interest += rate;
                } else if lure_speed < mm(spec.lure_speed_mm_s[0]) && self.interest > 60 * ONE {
                    // a pause after a good run is when a fish commits
                    self.interest += rate * 2;
                } else {
                    self.interest -= rate;
                }
                if kayak_close || !s.lure_wet || lure_dist > cm(spec.sense_cm) * 2 {
                    self.mood(Mood::Flee);
                } else if self.interest <= 0 && self.timer > TICKS_PER_SECOND as i32 {
                    self.mood(Mood::Cruise);
                } else if self.interest >= 100 * ONE {
                    // commit: aim where the lure will be in a third of a second
                    self.aim = s.lure.add(s.lure_vel.ratio(1, 3));
                    self.mood(Mood::Strike);
                    said = Some(Happened::Struck);
                }
            }
            Mood::Strike => {
                // a rush at the aim point, mouth opening; the bite is judged at the aim
                let mouth = self.mouth_at();
                let gap = self.aim.sub(mouth);
                let want = if gap.len() > ONE / 20 {
                    gap.with_len(burst)
                } else {
                    V3::ZERO
                };
                self.steer(spec, want);
                self.mouth = (self.mouth + ONE / 6).min(ONE);
                let arrived =
                    self.aim.sub(self.mouth_at()).len() < ONE / 10 || self.timer > TICKS_PER_SECOND as i32;
                if arrived {
                    if self.in_zone(spec, s.lure) {
                        self.mood(Mood::Hold);
                        said = Some(Happened::Bit);
                    } else {
                        self.misses += 1;
                        self.mouth = 0;
                        self.interest = 40 * ONE;
                        self.mood(if self.misses >= 3 {
                            Mood::Flee
                        } else {
                            Mood::Follow
                        });
                        said = Some(Happened::Missed);
                    }
                }
            }
            Mood::Hold | Mood::Hooked => {}
            Mood::Flee => {
                let away = self.pos.sub(s.kayak).flat();
                let away = if away.len() == 0 { v3(ONE, 0, 0) } else { away };
                let want = away.with_len(burst).add(v3(0, -cruise / 2, 0));
                self.steer(spec, want);
                self.mouth = 0;
                if self.timer > 4 * TICKS_PER_SECOND as i32 {
                    self.misses = 0;
                    self.interest = 0;
                    self.mood(Mood::Cruise);
                }
            }
        }
        // stay in the water and above the seabed
        self.pos.y = self.pos.y.clamp(s.seabed + ONE / 4, s.surface - ONE / 4);
        said
    }

    /// The fish's own swimming force while it has the lure in its mouth or
    /// is hooked; the line moves it. Returns what happened and the force.
    pub fn pull(&mut self, spec: &FishSpec, s: &Scene, rng: &mut Rng) -> (Option<Happened>, V3) {
        self.timer += 1;
        let strength = spec.strength_n * ONE;
        let away = self.pos.sub(s.kayak).flat();
        let away = if away.len() == 0 {
            v3(0, 0, ONE)
        } else {
            away.with_len(ONE)
        };
        let mut said = None;
        let force = match self.mood {
            Mood::Hold => {
                // it turns away with the lure, gently, until it spits it out
                if self.timer * 1000 > spec.hold_ms * TICKS_PER_SECOND as i32 {
                    self.mood(Mood::Flee);
                    self.mouth = 0;
                    said = Some(Happened::Spat);
                }
                away.scale(strength / 8)
            }
            Mood::Hooked => {
                // runs away and down, swerving; spent fish pull less
                if self.timer % 90 == 1 {
                    let swerve = rng.between(-fx::TURN / 8, fx::TURN / 8);
                    self.goal = away.yaw(swerve);
                }
                let dive = if self.pos.y > s.surface - cm(spec.depth_cm[1]) {
                    -ONE / 3
                } else {
                    0
                };
                let mut dir = self.goal.add(v3(0, dive, 0)).with_len(ONE);
                if spec.jumps && self.stamina > ONE / 2 && self.timer % 300 == 150 {
                    dir = v3(dir.x / 3, ONE, dir.z / 3).with_len(ONE);
                    said = Some(Happened::Jumped);
                }
                // the fish tires with the work it does against the line
                let drain = per_tick(fx::div(ONE, spec.stamina_s.max(1) * ONE));
                let worked = fx::mul(drain, fx::div(s.tension, strength.max(1)));
                self.stamina = (self.stamina - drain / 2 - worked).max(0);
                if s.tension < ONE / 5 {
                    self.slack += 1;
                } else {
                    self.slack = 0;
                }
                if self.slack > 5 * TICKS_PER_SECOND as i32 / 2 {
                    // a slack line lets the hook fall out
                    self.mood(Mood::Flee);
                    said = Some(Happened::Threw);
                }
                dir.scale(fx::mul(strength, self.stamina.max(ONE / 20)))
            }
            _ => V3::ZERO,
        };
        (said, force)
    }
}

fn per_tick_v(v: V3) -> V3 {
    v3(per_tick(v.x), per_tick(v.y), per_tick(v.z))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn snapper() -> FishSpec {
        FishSpec {
            name: "fish.snapper".into(),
            count: 4,
            length_cm: [40, 70],
            grams_at_shortest: 1000,
            depth_cm: [800, 1700],
            cruise_mm_s: 400,
            burst_mm_s: 2000,
            sense_cm: 1000,
            splash_cm: 1500,
            gape_mm: 60,
            lure_speed_mm_s: [100, 1000],
            interest_per_s: 60,
            hold_ms: 1000,
            strength_n: 18,
            stamina_s: 45,
            jumps: false,
            colours: [
                "#c4473a".into(),
                "#e8846a".into(),
                "#f7ddd0".into(),
                "#d4553f".into(),
            ],
            pattern: "none".into(),
            depth_pct: 30,
            width_pct: 11,
        }
    }

    fn fish_at(pos: V3) -> Fish {
        let mut f = Fish::spawn(0, &snapper(), V3::ZERO, 0, &mut Rng::new(1));
        f.pos = pos;
        f.vel = V3::ZERO;
        f.heading = 0;
        f.length = ONE / 2;
        f
    }

    fn scene(lure: V3) -> Scene {
        Scene {
            lure,
            lure_vel: V3::ZERO,
            lure_wet: true,
            splash: false,
            water: V3::ZERO,
            surface: 0,
            seabed: -18 * ONE,
            kayak: v3(0, 0, -30 * ONE),
            tension: 0,
        }
    }

    #[test]
    fn weight_grows_as_the_cube_of_length() {
        // 1 kg at 40 cm; a 60 cm fish is 1.5³ = 3.375 kg
        let s = snapper();
        let mut r = Rng::new(3);
        for _ in 0..50 {
            let f = Fish::spawn(0, &s, V3::ZERO, 0, &mut r);
            let ratio = fx::div(f.length, 40 * ONE / 100);
            let want = fx::mul(ONE, fx::mul(ratio, fx::mul(ratio, ratio)));
            assert!((f.mass - want).abs() < ONE / 100, "{} {}", f.mass, want);
            assert!(f.length >= 40 * ONE / 100 && f.length <= 70 * ONE / 100);
        }
    }

    #[test]
    fn a_strike_with_the_hook_inside_the_mouth_zone_bites() {
        // gape 6 cm: the zone reaches 6 cm from the mouth; the hook sits 4 cm off the aim
        let mut f = fish_at(v3(0, -10 * ONE, 0));
        f.aim = f.mouth_at();
        f.mood = Mood::Strike;
        let hook = f.aim.add(v3(4 * ONE / 100, 0, 0));
        let mut said = None;
        for _ in 0..70 {
            said = f.tick(&snapper(), &scene(hook), &mut Rng::new(2)).or(said);
            if f.mood != Mood::Strike {
                break;
            }
        }
        assert_eq!(said, Some(Happened::Bit));
        assert_eq!(f.mood, Mood::Hold);
    }

    #[test]
    fn a_strike_with_the_hook_outside_the_mouth_zone_misses() {
        // 8 cm off the aim: 2 cm outside the zone
        let mut f = fish_at(v3(0, -10 * ONE, 0));
        f.aim = f.mouth_at();
        f.mood = Mood::Strike;
        let hook = f.aim.add(v3(8 * ONE / 100, 0, 0));
        let mut said = None;
        for _ in 0..70 {
            said = f.tick(&snapper(), &scene(hook), &mut Rng::new(2)).or(said);
            if f.mood != Mood::Strike {
                break;
            }
        }
        assert_eq!(said, Some(Happened::Missed));
        assert_ne!(f.mood, Mood::Hold);
    }

    #[test]
    fn a_fish_spits_a_held_lure_after_its_hold_time() {
        // 1000 ms is 60 ticks
        let mut f = fish_at(v3(0, -10 * ONE, 0));
        f.mood = Mood::Hold;
        let mut spat_at = None;
        for t in 1..=90 {
            if f.pull(&snapper(), &scene(f.mouth_at()), &mut Rng::new(2)).0 == Some(Happened::Spat) {
                spat_at = Some(t);
                break;
            }
        }
        assert_eq!(spat_at, Some(61));
    }

    #[test]
    fn a_hooked_fish_tires_and_pulls_less() {
        let mut f = fish_at(v3(0, -10 * ONE, 0));
        f.mood = Mood::Hooked;
        let mut s = scene(f.mouth_at());
        s.tension = 10 * ONE;
        let first = f.pull(&snapper(), &s, &mut Rng::new(2)).1.len();
        for _ in 0..60 * 60 {
            f.pull(&snapper(), &s, &mut Rng::new(2));
        }
        let later = f.pull(&snapper(), &s, &mut Rng::new(2)).1.len();
        // 18 N fresh; against 10 N for a minute of its 45 s stamina it drains
        // 60/45 × (1/2 + 10/18) = 1.4 of its stamina, so it is spent
        assert!((first - 18 * ONE).abs() < ONE / 4, "{first}");
        assert!(later < first / 2, "{later} from {first}");
    }
}
