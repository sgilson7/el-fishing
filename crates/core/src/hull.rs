//! The kayak: a rigid hull floating on 16 probes (Kerner's model, as the
//! water tab describes). Each probe on the waterplane pushes up by ρ g a d,
//! where d is how far it sits below the surface, capped by the hull's
//! depth. Heave, pitch and roll come from those pushes; the pedal drive,
//! drag against the water (current and wave orbits included), the waves'
//! slope, the wind and the line push it across the water. A drift turns
//! it about its bow.
//!
//! Hull space: x across, y up, z toward the bow; the origin is the keel
//! under the middle of the waterplane.

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{self, v3, V3};
use crate::sea::{Sea, G, TAU};
use crate::ticks::TICKS_PER_SECOND;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullSpec {
    pub length_cm: i32,
    pub beam_cm: i32,
    pub depth_cm: i32,
    /// Kayak, angler and gear together.
    pub mass_kg: i32,
    /// The waterplane area each of the 16 probes stands for.
    pub probe_area_cm2: i32,
    pub thrust_n: i32,
    pub reverse_pct: i32,
    /// Quadratic drag along and across the keel, N per (m/s)².
    pub drag_along: i32,
    pub drag_across: i32,
    /// Linear drag across the keel, N per m/s, so a slow sideways drift settles.
    pub drag_across_linear: i32,
    /// Damping rates, hundredths per second.
    pub heave_damp: i32,
    pub pitch_damp: i32,
    pub roll_damp: i32,
    pub pitch_inertia_kg_m2: i32,
    pub roll_inertia_kg_m2: i32,
    /// How high the weight sits above the keel; it tips the hull in a roll.
    pub weight_above_keel_cm: i32,
    pub max_roll_deg: i32,
    pub drift_deg_s: i32,
    /// ½ ρ_air C_d A for the wind on the angler, thousandths of N per (m/s)².
    pub windage: i32,
    /// How much of the waves' slope push reaches the hull, percent.
    pub wave_push_pct: i32,
    /// Extra damping per step of the steady-hull setting, percent.
    pub steady_pct: i32,
    /// Where a drift turns the hull about, as a share of the half-length from
    /// the middle toward the bow: 100 is the bow (a kayak), -30 a little
    /// behind the middle (a speedboat steered from its stern).
    #[serde(default = "bow")]
    pub pivot_pct: i32,
    /// A planing hull's lift from its speed through the water, N at 10 m/s,
    /// growing as the square of the speed; 0 for a kayak.
    #[serde(default)]
    pub planing_n: i32,
}

fn bow() -> i32 {
    100
}

/// What the player asks of the hull this tick: -1, 0 or 1 each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HullInput {
    pub pedal: i32,
    pub drift: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hull {
    pub spec: HullSpec,
    /// The keel's middle: x, y (its height), z.
    pub pos: V3,
    pub vel: V3,
    /// Compass heading in turns (`fx::TURN`): 0 faces +z.
    pub heading: i32,
    /// Radians, bow up positive.
    pub pitch: Fx,
    /// Radians, the +x side up positive.
    pub roll: Fx,
    pub pitch_rate: Fx,
    pub roll_rate: Fx,
    /// The steady-hull setting: 0 off, 1 some, 2 more.
    pub steady: i32,
    /// Horizontal velocity and position with 32 fraction bits. In Q16 a
    /// force under half a newton changes a substep's velocity by less than
    /// one unit and is lost, so a slow drift would stall short (R2.3).
    fine_vel: [i64; 2],
    fine_pos: [i64; 2],
}

const FINE: u32 = 16;

pub const SUBSTEPS: i32 = 4;
pub const PROBES_ALONG: i32 = 8;
pub const PROBES: usize = 16;

/// A 64-bit Q16 sum over a Q16 value, saturating.
fn div64(a: i64, b: Fx) -> Fx {
    ((a << 16) / b.max(1) as i64).clamp(i32::MIN as i64, i32::MAX as i64) as Fx
}

fn rad_to_turns(r: Fx) -> i32 {
    fx::div(r, TAU)
}

impl Hull {
    pub fn new(spec: HullSpec, at: V3, heading: i32) -> Hull {
        Hull {
            spec,
            pos: at,
            vel: V3::ZERO,
            heading,
            pitch: 0,
            roll: 0,
            pitch_rate: 0,
            roll_rate: 0,
            steady: 1,
            fine_vel: [0, 0],
            fine_pos: [(at.x as i64) << FINE, (at.z as i64) << FINE],
        }
    }

    pub fn length(&self) -> Fx {
        self.spec.length_cm * ONE / 100
    }

    pub fn mass(&self) -> Fx {
        self.spec.mass_kg * ONE
    }

    /// A point in hull space, in the world.
    pub fn to_world(&self, local: V3) -> V3 {
        self.pos.add(self.rotate(local))
    }

    /// A direction in hull space, in the world.
    pub fn rotate(&self, local: V3) -> V3 {
        geom::rotate(
            local,
            self.heading,
            -rad_to_turns(self.pitch),
            rad_to_turns(self.roll),
        )
    }

    /// The probes' places in hull space: eight along, two across, on the keel's plane.
    pub fn probes(&self) -> [V3; PROBES] {
        let l = self.length() * 9 / 10;
        let half_beam = self.spec.beam_cm * ONE / 400;
        let mut out = [V3::ZERO; PROBES];
        for i in 0..PROBES_ALONG {
            let z = (2 * i + 1) * l / (2 * PROBES_ALONG) - l / 2;
            out[i as usize * 2] = v3(-half_beam, 0, z);
            out[i as usize * 2 + 1] = v3(half_beam, 0, z);
        }
        out
    }

    /// The bow's tip on the keel, in the world.
    pub fn bow(&self) -> V3 {
        self.to_world(v3(0, 0, self.length() / 2))
    }

    pub fn forward(&self) -> V3 {
        v3(fx::sin(self.heading), 0, fx::cos(self.heading))
    }

    pub fn right(&self) -> V3 {
        v3(fx::cos(self.heading), 0, -fx::sin(self.heading))
    }

    /// Speed through the world (not the water), along the ground.
    pub fn speed(&self) -> Fx {
        self.vel.flat_len()
    }

    /// One tick: forces from the sea and the player, and the drift.
    /// `pull` is the line's pull on the angler, in newtons.
    pub fn tick(&mut self, sea: &Sea, input: HullInput, pull: V3) {
        let s = self.spec.clone();
        let h = ONE / (TICKS_PER_SECOND as i32 * SUBSTEPS);
        let m = self.mass();
        let rho_g_a = fx::mul(fx::mul(sea.density, G), s.probe_area_cm2 * ONE / 10_000);
        let depth = s.depth_cm * ONE / 100;
        let steady = 100 + s.steady_pct * self.steady.clamp(0, 2);
        let damp = |per_100: i32| per_100 * ONE / 100 * steady / 100;
        let weight = fx::mul(m, G);
        let lever = s.weight_above_keel_cm * ONE / 100;
        let max_roll = s.max_roll_deg * TAU / 360;
        let probes = self.probes();
        for _ in 0..SUBSTEPS {
            // summed in 64 bits: a speedboat's sixteen probes together pass
            // the 32 767 N a Q16 value holds
            let (mut lift, mut pitch_t, mut roll_t) = (0i64, 0i64, 0i64);
            let (mut sx, mut sz, mut wet) = (0, 0, 0);
            for p in &probes {
                let w = self.to_world(*p);
                let surface = sea.surface(w.x, w.z);
                let d = (surface.height - w.y).clamp(0, depth);
                if d == 0 {
                    continue;
                }
                let f = fx::mul(rho_g_a, d);
                lift += f as i64;
                pitch_t += fx::mul(f, p.z) as i64;
                roll_t += fx::mul(f, p.x) as i64;
                sx += surface.slope_x;
                sz += surface.slope_z;
                wet += 1;
            }
            // the water damps the hull only where it touches it: out of the
            // water a hull flies free, off a wave's crest (the speedboat)
            let touch = ONE * wet / PROBES as i32;
            // a planing hull rides up on its speed through the water
            let through = self.vel.sub(sea.surface_velocity(self.pos.x, self.pos.z)).flat();
            let u = through.dot(self.forward()) as Fx;
            let plane = if s.planing_n > 0 && wet > 0 {
                let k = fx::div(u.max(0), 10 * ONE);
                fx::mul(fx::mul(s.planing_n * ONE, fx::mul(k, k)), touch).min(weight / 10 * 9)
            } else {
                0
            };
            let damp = |per_100: i32| fx::mul(damp(per_100), touch);
            // heave
            let ay = div64(lift + plane as i64 - weight as i64, m) - fx::mul(damp(s.heave_damp), self.vel.y);
            self.vel.y += fx::mul(ay, h);
            // pitch and roll; the weight above the keel tips the hull further in a roll
            let tip = fx::mul(fx::mul(weight, lever), fx::sin(rad_to_turns(self.roll)));
            // planing lifts the bow: the lift acts a tenth of the length ahead of the middle
            let pitch_t = pitch_t + fx::mul(plane, self.length() / 10) as i64;
            let ap =
                div64(pitch_t, s.pitch_inertia_kg_m2 * ONE) - fx::mul(damp(s.pitch_damp), self.pitch_rate);
            let ar = div64(roll_t + tip as i64, s.roll_inertia_kg_m2 * ONE)
                - fx::mul(damp(s.roll_damp), self.roll_rate);
            self.pitch_rate += fx::mul(ap, h);
            self.roll_rate += fx::mul(ar, h);
            self.pitch += fx::mul(self.pitch_rate, h);
            self.roll += fx::mul(self.roll_rate, h);
            if self.roll.abs() > max_roll {
                // no capsizing in the first version (W3)
                self.roll = self.roll.clamp(-max_roll, max_roll);
                self.roll_rate = 0;
            }
            // across the water: pedals, drag against the moving water, the waves' slope, wind, line
            let fwd = self.forward();
            let right = self.right();
            let water = sea.surface_velocity(self.pos.x, self.pos.z);
            let rel = self.vel.sub(water).flat();
            let u = rel.dot(fwd) as Fx;
            let w = rel.dot(right) as Fx;
            // the pedals' fins or the propeller push only while in the water
            let thrust = if wet == 0 {
                0
            } else {
                match input.pedal.signum() {
                    1 => s.thrust_n * ONE,
                    -1 => -s.thrust_n * ONE * s.reverse_pct / 100,
                    _ => 0,
                }
            };
            // the water drags only on a hull in it
            let along = thrust - fx::mul(fx::mul(s.drag_along * ONE, fx::mul(u, u.abs())), touch);
            let across = fx::mul(
                -fx::mul(s.drag_across * ONE, fx::mul(w, w.abs())) - s.drag_across_linear * w,
                touch,
            );
            let mut f = fwd.scale(along).add(right.scale(across));
            if wet > 0 {
                let push = fx::mul(weight, s.wave_push_pct * ONE / 100) / wet;
                f = f.add(v3(-fx::mul(push, sx), 0, -fx::mul(push, sz)));
            }
            let air = sea.wind.sub(self.vel).flat();
            let air_speed = air.flat_len();
            f = f.add(air.scale(fx::mul(s.windage * ONE / 1000, air_speed)));
            f = f.add(pull.flat());
            // dv = F h / m with 32 fraction bits: F (Q16) << 32 / m (Q16) is Q32 acceleration
            for (i, fi) in [f.x, f.z].into_iter().enumerate() {
                let accel = ((fi as i64) << 32) / m as i64;
                self.fine_vel[i] += (accel * h as i64) >> 16;
                self.fine_pos[i] += (self.fine_vel[i] * h as i64) >> 16;
            }
            self.vel.x = (self.fine_vel[0] >> FINE) as Fx;
            self.vel.z = (self.fine_vel[1] >> FINE) as Fx;
            self.pos.x = (self.fine_pos[0] >> FINE) as Fx;
            self.pos.z = (self.fine_pos[1] >> FINE) as Fx;
            self.pos.y += fx::mul(self.vel.y, h);
            self.keep_off_banks(sea);
        }
        if input.drift != 0 {
            self.drift(input.drift.signum());
        }
    }

    /// A river's banks and rocks stop the hull: the middle keeps half a beam
    /// and a little clear, the bow and stern a little, and the speed into
    /// whatever it met is lost (T16).
    fn keep_off_banks(&mut self, sea: &Sea) {
        let half = self.length() / 2;
        let clear = ONE / 5;
        let beam = self.spec.beam_cm * ONE / 200 + clear;
        for (local, margin) in [
            (V3::ZERO, beam),
            (v3(0, 0, half), clear),
            (v3(0, 0, -half), clear),
        ] {
            let push = sea.push_out(self.to_world(local), margin);
            if push == V3::ZERO {
                continue;
            }
            self.pos = self.pos.add(push.flat());
            let n = push.flat().with_len(ONE);
            let into = self.vel.flat().dot(n) as Fx;
            if into < 0 {
                self.vel = self.vel.sub(n.scale(into));
            }
            self.fine_pos = [(self.pos.x as i64) << FINE, (self.pos.z as i64) << FINE];
            self.fine_vel = [(self.vel.x as i64) << FINE, (self.vel.z as i64) << FINE];
        }
    }

    /// Turns the hull about its pivot (a kayak's bow): the pivot keeps its
    /// place and the rest swings.
    fn drift(&mut self, dir: i32) {
        let at = v3(0, 0, self.length() / 2 * self.spec.pivot_pct / 100);
        let pivot = self.to_world(at);
        let step = self.spec.drift_deg_s * fx::TURN / 360 / TICKS_PER_SECOND as i32;
        self.heading = (self.heading + dir * step).rem_euclid(fx::TURN);
        let moved = self.to_world(at).sub(pivot);
        self.pos = self.pos.sub(moved);
        self.fine_pos = [(self.pos.x as i64) << FINE, (self.pos.z as i64) << FINE];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea::{SeaSpec, WaveSpec};

    pub fn spec() -> HullSpec {
        HullSpec {
            length_cm: 390,
            beam_cm: 86,
            depth_cm: 30,
            mass_kg: 130,
            probe_area_cm2: 1470,
            thrust_n: 120,
            reverse_pct: 40,
            drag_along: 50,
            drag_across: 500,
            drag_across_linear: 20,
            heave_damp: 675,
            pitch_damp: 500,
            roll_damp: 400,
            pitch_inertia_kg_m2: 165,
            roll_inertia_kg_m2: 17,
            weight_above_keel_cm: 30,
            max_roll_deg: 25,
            drift_deg_s: 30,
            windage: 480,
            wave_push_pct: 100,
            steady_pct: 50,
            pivot_pct: 100,
            planing_n: 0,
        }
    }

    #[test]
    fn a_hull_clear_of_the_water_falls_freely() {
        // dropped from 3 m over still water, half a second falls g t² / 2 =
        // 9.81 × 0.25 / 2 = 1.226 m, at 4.9 m/s: the water's damping acts only
        // on a hull that touches it
        let sea = still_sea();
        let mut h = Hull::new(spec(), v3(0, 3 * ONE, 0), 0);
        for _ in 0..30 {
            h.tick(&sea, HullInput::default(), V3::ZERO);
        }
        let fell = 3 * ONE - h.pos.y;
        assert!((fell - ONE * 1226 / 1000).abs() < ONE / 50, "fell {fell}");
        assert!(
            (h.vel.y + ONE * 4905 / 1000).abs() < ONE / 20,
            "speed {}",
            h.vel.y
        );
    }

    fn still_sea() -> Sea {
        let mut s = Sea::new(&SeaSpec {
            waves: vec![WaveSpec {
                toward_deg: 0,
                length_cm: 4000,
                amp_mm: 500,
                steep_pct: 0,
            }],
            tide_amp_mm: 600,
            tide_period_minutes: 20,
            current_toward_deg: 90,
            current_mm_s: 206,
            wind_toward_deg: 0,
            wind_mm_s: 0,
            density_kg_m3: 1025,
            depth_cm: 1800,
            river: None,
        });
        s.waves_on = false;
        s.tide_on = false;
        s.current_on = false;
        s
    }

    fn run(h: &mut Hull, sea: &Sea, input: HullInput, ticks: i32) {
        for _ in 0..ticks {
            h.tick(sea, input, V3::ZERO);
        }
    }

    #[test]
    fn a_still_hull_settles_where_it_displaces_its_own_mass() {
        // 130 kg ÷ (1025 kg/m³ × 16 × 0.147 m²) = 0.0539 m below the surface
        let sea = still_sea();
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        run(&mut h, &sea, HullInput::default(), 600);
        let draft = -h.pos.y;
        assert!(
            (draft - 3_533).abs() < ONE / 1000,
            "draft {draft} (want 3533 = 0.0539 m)"
        );
    }

    #[test]
    fn the_pedals_reach_three_knots_on_still_water() {
        // 120 N = 50 v² gives v = 1.549 m/s = 101 515 in Q16
        let sea = still_sea();
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        run(&mut h, &sea, HullInput { pedal: 1, drift: 0 }, 30 * 60);
        let v = h.speed();
        assert!((v - 101_515).abs() < 1_015, "speed {v}");
        assert!(
            h.pos.z > 0 && h.pos.x.abs() < ONE / 100,
            "it went straight ahead: {:?}",
            h.pos
        );
    }

    #[test]
    fn a_drift_turns_the_stern_about_a_still_bow() {
        let sea = still_sea();
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        run(&mut h, &sea, HullInput::default(), 120);
        let bow = h.bow();
        run(&mut h, &sea, HullInput { pedal: 0, drift: 1 }, 60);
        // 30 degrees is 30/360 of a turn: 5461 units, within a degree (182)
        assert!((h.heading - 5_461).abs() < 182, "heading {}", h.heading);
        let moved = h.bow().sub(bow).flat_len();
        assert!(moved < 5 * ONE / 100, "the bow moved {moved}");
    }

    #[test]
    fn the_current_carries_an_idle_kayak_and_stops_when_turned_off() {
        // current 0.206 m/s along +x, across a kayak facing +z
        let mut sea = still_sea();
        sea.current_on = true;
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        run(&mut h, &sea, HullInput::default(), 60 * 60);
        let want = 206 * ONE / 1000;
        assert!((h.vel.x - want).abs() < want / 20, "drift {}", h.vel.x);
        sea.current_on = false;
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        run(&mut h, &sea, HullInput::default(), 60 * 60);
        assert_eq!(h.vel.flat_len(), 0);
    }

    #[test]
    fn waves_lift_and_tilt_the_hull_and_do_not_when_turned_off() {
        let mut sea = still_sea();
        sea.waves_on = true;
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        let (mut lo, mut hi, mut pitch) = (0, 0, 0);
        for _ in 0..600 {
            sea.tick();
            h.tick(&sea, HullInput::default(), V3::ZERO);
            lo = lo.min(h.pos.y);
            hi = hi.max(h.pos.y);
            pitch = pitch.max(h.pitch.abs());
        }
        // a 0.5 m wave 40 m long lifts the hull through most of its 1 m height
        assert!(hi - lo > 7 * ONE / 10, "heave {}", hi - lo);
        assert!(pitch > ONE / 100, "pitch {pitch}");
        sea.waves_on = false;
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        for _ in 0..600 {
            sea.tick();
            h.tick(&sea, HullInput::default(), V3::ZERO);
        }
        assert!(h.pitch.abs() < 10, "pitch {}", h.pitch);
    }

    #[test]
    fn a_hard_push_on_one_side_cannot_capsize_the_hull() {
        let sea = still_sea();
        let mut h = Hull::new(spec(), V3::ZERO, 0);
        h.roll_rate = 20 * ONE;
        run(&mut h, &sea, HullInput::default(), 60);
        assert!(h.roll.abs() <= 25 * TAU / 360, "roll {}", h.roll);
    }
}
