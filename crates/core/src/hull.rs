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
    /// A steerable outboard on the transom, in place of the pedals and the
    /// drift (T19). With one, the pedal keys move a throttle lever that stays
    /// where it is left, and the drift keys turn the outboard.
    #[serde(default)]
    pub outboard: Option<OutboardSpec>,
}

/// An outboard: its thrust turns with it, so it pushes the stern sideways
/// and the hull turns about its middle at a rate the water damps.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutboardSpec {
    /// The farthest the outboard turns each way, and how fast it turns and recentres.
    pub steer_max_deg: i32,
    pub steer_deg_s: i32,
    pub centre_deg_s: i32,
    /// How fast the throttle lever moves, percent of full a second.
    pub throttle_pct_s: i32,
    /// The hull's resistance to turning: its inertia about the vertical, and
    /// the water's damping of the turn, N·m per rad/s.
    pub yaw_inertia_kg_m2: i32,
    pub yaw_damp: i32,
    /// A planing hull leans into its turn: roll torque, N·m per (rad/s × m/s)
    /// of turn rate times speed through the water.
    pub bank: i32,
}

fn bow() -> i32 {
    100
}

/// What the player asks of the hull this tick: -1, 0 or 1 each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HullInput {
    pub pedal: i32,
    pub drift: i32,
    /// Snap the throttle to neutral (an outboard only).
    pub neutral: bool,
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
    /// An outboard's throttle lever, -reverse to ONE, its angle in turns
    /// (positive turns the bow right) and the hull's turn rate, rad/s.
    pub throttle: Fx,
    pub steer: i32,
    pub yaw_rate: Fx,
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
            throttle: 0,
            steer: 0,
            yaw_rate: 0,
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
        if let Some(o) = &s.outboard {
            self.helm(o, &s, input);
        }
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
            // leaning into a turn: a right turn puts the right side down
            let lean = match &s.outboard {
                Some(o) => {
                    let ahead = self
                        .vel
                        .sub(sea.current_at(self.pos.x, self.pos.z))
                        .flat()
                        .dot(self.forward()) as Fx;
                    -(o.bank as i64 * fx::mul(self.yaw_rate, ahead.max(0)) as i64 * wet as i64
                        / PROBES as i64)
                }
                None => 0,
            };
            let ar = div64(roll_t + tip as i64 + lean, s.roll_inertia_kg_m2 * ONE)
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
            } else if s.outboard.is_some() {
                fx::mul(s.thrust_n * ONE, self.throttle)
            } else {
                match input.pedal.signum() {
                    1 => s.thrust_n * ONE,
                    -1 => -s.thrust_n * ONE * s.reverse_pct / 100,
                    _ => 0,
                }
            };
            // an outboard turned by δ pushes along cos δ and the stern sideways
            // by sin δ; that sideways push turns the hull about its middle
            let (push_along, push_side) = if s.outboard.is_some() {
                (
                    fx::mul(thrust, fx::cos(self.steer)),
                    fx::mul(thrust, fx::sin(self.steer)),
                )
            } else {
                (thrust, 0)
            };
            if let Some(o) = &s.outboard {
                let torque = fx::mul(push_side, self.length() / 2);
                // in 64 bits: a heavy hull's damping times ONE passes i32
                let damp_t = o.yaw_damp as i64 * self.yaw_rate as i64;
                let alpha = div64(torque as i64 - damp_t, o.yaw_inertia_kg_m2 * ONE);
                self.yaw_rate += fx::mul(alpha, h);
                self.heading = (self.heading + rad_to_turns(fx::mul(self.yaw_rate, h))).rem_euclid(fx::TURN);
            }
            // the water drags only on a hull in it
            // in 64 bits: a hull sliding sideways at speed meets drag far past
            // what a Q16 value holds, and a wrapped sum pushed it on (T19)
            let wide = |v: Fx| v as i64;
            let q = |k: i32, v: Fx| (k as i64 * wide(v) * wide(v).abs()) >> 16;
            let clamp = |v: i64| v.clamp(-(i32::MAX as i64) / 2, i32::MAX as i64 / 2) as Fx;
            let along = clamp(wide(push_along) - ((q(s.drag_along, u) * wide(touch)) >> 16));
            let across =
                clamp(((-q(s.drag_across, w) - s.drag_across_linear as i64 * wide(w)) * wide(touch)) >> 16);
            // the outboard's sideways push acts at the stern, toward the left
            // when it turns the bow right
            let mut f = fwd
                .scale(along)
                .add(right.scale(across.saturating_sub(push_side)));
            if wet > 0 {
                // the waves' slope, felt only through the probes in the water
                let push = fx::mul(fx::mul(weight, s.wave_push_pct * ONE / 100), touch) / wet;
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
        if input.drift != 0 && s.outboard.is_none() {
            self.drift(input.drift.signum());
        }
    }

    /// The helm, once a tick: the throttle lever moves while a pedal key is
    /// held and stays where it is left; the outboard turns while a drift key
    /// is held and swings back to the middle when it is let go.
    fn helm(&mut self, o: &OutboardSpec, s: &HullSpec, input: HullInput) {
        let per = TICKS_PER_SECOND as i32;
        let reverse = -ONE * s.reverse_pct / 100;
        if input.neutral {
            self.throttle = 0;
        } else if input.pedal != 0 {
            let step = ONE * o.throttle_pct_s / 100 / per;
            self.throttle = (self.throttle + input.pedal.signum() * step).clamp(reverse, ONE);
        }
        let most = o.steer_max_deg * fx::TURN / 360;
        if input.drift != 0 {
            let step = o.steer_deg_s * fx::TURN / 360 / per;
            self.steer = (self.steer + input.drift.signum() * step).clamp(-most, most);
        } else {
            let step = o.centre_deg_s * fx::TURN / 360 / per;
            self.steer -= self.steer.signum() * step.min(self.steer.abs());
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
            // scraping along a bank or a rock costs speed: a tenth of what is
            // left each substep in contact
            let y = self.vel.y;
            self.vel = self.vel.scale(ONE * 9 / 10);
            self.vel.y = y;
            self.yaw_rate = self.yaw_rate * 9 / 10;
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
            outboard: None,
        }
    }

    fn speedboat() -> HullSpec {
        let gear: serde_json::Value = serde_json::from_str(include_str!("../../../data/gear.json")).unwrap();
        serde_json::from_value(gear["boats"][1]["hull"].clone()).unwrap()
    }

    #[test]
    fn the_throttle_lever_stays_where_it_is_left_and_the_outboard_recentres() {
        let sea = still_sea();
        let mut h = Hull::new(speedboat(), V3::ZERO, 0);
        let input = |pedal: i32, drift: i32, neutral: bool| HullInput {
            pedal,
            drift,
            neutral,
        };
        // 50% a second: one second of Up is half throttle, and it stays there
        run(&mut h, &sea, input(1, 0, false), 60);
        assert!((h.throttle - ONE / 2).abs() < ONE / 100, "{}", h.throttle);
        run(&mut h, &sea, input(0, 0, false), 120);
        assert!((h.throttle - ONE / 2).abs() < ONE / 100, "{}", h.throttle);
        run(&mut h, &sea, input(0, 0, true), 1);
        assert_eq!(h.throttle, 0);
        // 90 degrees a second for a fifth of a second is 18 degrees right;
        // let go, it swings back at 120 a second, so a sixth of a second centres it
        run(&mut h, &sea, input(0, 1, false), 12);
        assert!((h.steer * 360 / fx::TURN - 18).abs() <= 1, "{}", h.steer);
        run(&mut h, &sea, input(0, 0, false), 10);
        assert_eq!(h.steer, 0);
    }

    #[test]
    fn full_lock_at_full_throttle_turns_at_the_rate_the_water_allows() {
        // the outboard at 30 degrees pushes the stern sideways with
        // 5200 × sin 30 = 2600 N, 2.75 m behind the middle: 7150 N·m, against
        // 10 000 N·m per rad/s of damping, so the turn settles at 0.715 rad/s
        let mut sea = still_sea();
        sea.tide_on = false;
        let mut h = Hull::new(speedboat(), V3::ZERO, 0);
        h.throttle = ONE;
        run(&mut h, &sea, HullInput::default(), 600);
        run(
            &mut h,
            &sea,
            HullInput {
                pedal: 0,
                drift: 1,
                neutral: false,
            },
            300,
        );
        let want = ONE * 715 / 1000;
        assert!((h.yaw_rate - want).abs() < want / 10, "turn rate {}", h.yaw_rate);
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
        run(
            &mut h,
            &sea,
            HullInput {
                pedal: 1,
                drift: 0,
                neutral: false,
            },
            30 * 60,
        );
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
        run(
            &mut h,
            &sea,
            HullInput {
                pedal: 0,
                drift: 1,
                neutral: false,
            },
            60,
        );
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
