//! The line: a chain of points from the rod tip to the lure, solved with
//! XPBD (Macklin, Müller and Chentanez 2016). Neighbouring points are joined
//! by ropes that can go slack but never push, and each point is tied to the
//! tip by a long-range attachment no longer than the line between them
//! (Kim, Chentanez and Müller 2012), so a long chain cannot stretch however
//! heavy its end. The end's tie carries the whole line's give: a spring of
//! EA over the line out, so a long line is softer than a short one, as a
//! real one is. Weight, buoyancy and quadratic drag against the water (or
//! the wind, above the surface) act on every point; the current carries them.
//!
//! The reel: F opens the bail with a finger on the line; letting go of F
//! lets line run while the bail is open; Space closes the bail and winds.
//! With the bail closed, the drag lets line slip when the stretch says the
//! pull is over its setting. Above the breaking tension the line snaps.
//!
//! Tension is the end tie's multiplier over h² (the end's whole load) plus
//! the weight and drag of each point of line held at its full reach. While
//! the spool runs free the line is respaced along its new length instead of
//! being held, so it pulls nothing.

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::sea::{Sea, G};
use crate::ticks::TICKS_PER_SECOND;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineSpec {
    pub points: usize,
    pub start_out_cm: i32,
    pub shortest_out_cm: i32,
    pub longest_out_m: i32,
    /// Tenths of a newton.
    pub break_dn: i32,
    pub drag_dn: i32,
    pub diameter_um: i32,
    pub density_kg_m3: i32,
    /// The least mass a point is given, milligrams: a numerical floor so a
    /// light point next to the lure is not flung about (R2.5).
    pub point_floor_mg: i32,
    pub lure_mass_g: i32,
    pub lure_volume_mm3: i32,
    pub lure_area_mm2: i32,
    /// How far the line stretches at its breaking tension, percent of its
    /// length; this sets its stiffness EA (R2.6).
    pub stretch_at_break_pct: i32,
    /// The line's internal damping: XPBD's γ, in percent (R2.6).
    pub damping_pct: i32,
    pub wind_per_turn_cm: i32,
    /// How fast the electric motor brings line in while it pulls.
    pub power_wind_cm_s: i32,
    /// The most the motor pulls, tenths of a newton. While it pulls the drag
    /// holds up to this, so a jerk can lift a tired fish (T15).
    pub power_pull_dn: i32,
    pub turns_per_s_x10: i32,
    /// N per metre of tip bend, and the most the tip bends.
    pub rod_stiffness_n_m: i32,
    pub rod_bend_max_cm: i32,
    /// Gauss-Seidel passes over the line each substep.
    pub iterations: usize,
    /// Drag along the line as a share of drag across it, percent.
    pub along_drag_pct: i32,
    /// With the bail open, line leaves the spool only while the line out is
    /// shorter than the straight distance from the tip to the lure plus this
    /// share of it: the curve a flying or drifting line takes (T10).
    pub free_slack_pct: i32,
}

/// What the player does with the reel this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineInput {
    /// F: a finger on the line, which also opens the bail.
    pub finger: bool,
    /// Space pressed: close the bail (T24).
    pub close: bool,
    /// Space held: wind.
    pub wind: bool,
    /// Q: let line out with the bail closed, as at the kayak before a cast (T23).
    pub let_out: bool,
    /// E: close the bail and bring the line in on the reel's electric motor.
    /// The world pulses it and runs it from the battery (T15).
    pub power: bool,
}

/// What hangs on the end of the line: the lure, or a fish holding it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndLoad {
    /// kg, Q16.
    pub mass: Fx,
    /// Volume in litres (Q16), for buoyancy.
    pub volume: Fx,
    /// ½ ρ C_d A over ρ: square metres, Q16; times the fluid's density gives drag.
    pub area: Fx,
    /// Any other force on it (a fish swimming), N.
    pub force: V3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub spec: LineSpec,
    pub pts: Vec<V3>,
    pub vel: Vec<V3>,
    /// Line paid out, m.
    pub out: Fx,
    pub bail_open: bool,
    pub finger: bool,
    /// Pull at the tip this tick, N.
    pub tension: Fx,
    /// The tension the break is judged on (the same as `tension` while the
    /// line's stretch carries the load).
    pub load: Fx,
    pub snapped: bool,
    /// How far the tip bends under the tension, toward the line.
    pub bend: V3,
    /// True while the drag let line slip this tick.
    pub slipping: bool,
    prev_tip: V3,
}

pub const SUBSTEPS: i32 = 2;

/// The air's density, 1.2 kg/m³.
const AIR: Fx = 78_643;
const LINE_CD: Fx = ONE * 12 / 10;

fn deci(n: i32) -> Fx {
    n * ONE / 10
}

impl Line {
    pub fn new(spec: LineSpec, tip: V3) -> Line {
        let n = spec.points.max(2);
        let out = spec.start_out_cm * ONE / 100;
        let seg = out / (n as i32 - 1);
        let pts = (0..n).map(|i| tip.sub(v3(0, seg * i as i32, 0))).collect();
        Line {
            spec,
            pts,
            vel: vec![V3::ZERO; n],
            out,
            bail_open: false,
            finger: false,
            tension: 0,
            load: 0,
            snapped: false,
            bend: V3::ZERO,
            slipping: false,
            prev_tip: tip,
        }
    }

    pub fn end(&self) -> V3 {
        *self.pts.last().unwrap_or(&V3::ZERO)
    }

    pub fn break_tension(&self) -> Fx {
        deci(self.spec.break_dn)
    }

    /// EA: the force that would stretch the line by its whole length, N.
    pub fn stiffness(&self) -> Fx {
        fx::div(
            self.break_tension(),
            self.spec.stretch_at_break_pct.max(1) * ONE / 100,
        )
    }

    pub fn drag(&self) -> Fx {
        deci(self.spec.drag_dn)
    }

    /// The lure alone on the end.
    pub fn lure(&self) -> EndLoad {
        let s = &self.spec;
        EndLoad {
            mass: s.lure_mass_g * ONE / 1000,
            volume: s.lure_volume_mm3 * (ONE / 1000) / 1000,
            area: s.lure_area_mm2 * ONE / 1_000_000 / 2,
            force: V3::ZERO,
        }
    }

    /// The chain's length along its points.
    pub fn chain_length(&self) -> Fx {
        self.pts.windows(2).map(|w| w[1].sub(w[0]).len()).sum()
    }

    fn seg(&self) -> Fx {
        self.out / (self.pts.len() as i32 - 1)
    }

    /// The mass, volume and drag area of one point of line.
    fn point(&self) -> (Fx, Fx, Fx) {
        let s = &self.spec;
        let d = s.diameter_um * ONE / 1_000_000;
        let seg = self.seg();
        // cross-section π d²/4 times the segment; d² is tiny, so work in mm²
        let d_mm = s.diameter_um * ONE / 1000;
        let section_mm2 = fx::mul(fx::mul(d_mm, d_mm), ONE * 314 / 400);
        let volume_l = fx::mul(section_mm2, seg) / 1000;
        let mass = fx::mul(s.density_kg_m3 * ONE, volume_l) / 1000;
        let area = fx::mul(fx::mul(d, seg), LINE_CD) / 2;
        (mass, volume_l, area)
    }

    /// One tick. `tip` is the rod tip in the world, unbent; returns the
    /// line's pull on the angler, N.
    pub fn tick(&mut self, sea: &Sea, tip: V3, input: LineInput, end: EndLoad) -> V3 {
        let n = self.pts.len();
        let s = self.spec.clone();
        let h = ONE / (TICKS_PER_SECOND as i32 * SUBSTEPS);
        // the reel
        if input.finger && !self.finger {
            self.bail_open = true;
        }
        self.finger = input.finger;
        if input.close || input.wind || input.power || input.let_out {
            self.bail_open = false;
        }
        let pinned = !self.bail_open || self.finger;
        // the rod's bend toward the line, for drawing; the line's own stretch
        // carries the give (a bending tip as a second spring rang against it)
        let toward = self.pts[1].sub(tip);
        let bend_len = fx::div(self.tension, s.rod_stiffness_n_m * ONE).min(s.rod_bend_max_cm * ONE / 100);
        self.bend = toward.with_len(bend_len);
        // which points are under water, once a tick
        let wet: Vec<bool> = self.pts.iter().map(|p| p.y < sea.height(p.x, p.z)).collect();
        // the current and the bottom under each point, once a tick: a river's
        // flow and bed change from place to place (T16)
        let water: Vec<V3> = self.pts.iter().map(|p| sea.current_at(p.x, p.z)).collect();

        let (pm, pv, pa) = self.point();
        let floor_mass = s.point_floor_mg * ONE / 1_000_000;
        let wind_per_s = fx::mul(s.wind_per_turn_cm * ONE / 100, s.turns_per_s_x10 * ONE / 10);
        let power_per_s = s.power_wind_cm_s * ONE / 100;
        let mut tension_sum: i64 = 0;
        let per_h2 = (TICKS_PER_SECOND * SUBSTEPS as i64).pow(2);
        self.slipping = false;
        for sub in 0..SUBSTEPS {
            let tip_now = self.prev_tip.lerp(tip, (sub + 1) * ONE / SUBSTEPS);
            let old = self.pts.clone();
            let mut pred = self.pts.clone();
            pred[0] = tip_now;
            // the forces on each point, kept to read the tension from
            let mut felt = vec![V3::ZERO; n];
            // where the bottom moved a point this substep: a correction of its
            // place, not a speed, so it is left out of the velocity after
            let mut shift = vec![V3::ZERO; n];
            for i in 1..n {
                let is_end = i == n - 1;
                let (mass, volume, area, force) = if is_end {
                    (end.mass, end.volume, end.area, end.force)
                } else {
                    (pm, pv, pa, V3::ZERO)
                };
                let inertia = mass.max(floor_mass).max(1);
                let (rho, flow) = if wet[i] {
                    (sea.density, water[i])
                } else {
                    (AIR, sea.wind)
                };
                // weight, less buoyancy under water, plus any other force
                // ρg per litre first: ρg × litres would pass Q16's range for a fish
                let lift = if wet[i] {
                    fx::mul(fx::mul(rho, G) / 1000, volume)
                } else {
                    0
                };
                let f = force.add(v3(0, lift - fx::mul(mass, G), 0));
                let mut v = self.vel[i].add(f.scale(fx::div(h, inertia)));
                // quadratic drag, applied implicitly so a light point cannot
                // overshoot; a point of line drags far less along the line than
                // across it (Morison: the along-line share is `along_drag_pct`)
                let rel = v.sub(flow);
                let c = fx::mul(rho, area);
                let along = if is_end || i + 1 >= n {
                    V3::ZERO
                } else {
                    self.pts[i + 1].sub(self.pts[i - 1]).with_len(ONE)
                };
                let rel_t = along.scale(rel.dot(along) as Fx);
                let rel_n = rel.sub(rel_t);
                let k_n = fx::mul(fx::div(fx::mul(c, rel_n.len()), inertia), h);
                let c_t = fx::mul(c, s.along_drag_pct * ONE / 100);
                let k_t = fx::mul(fx::div(fx::mul(c_t, rel_t.len()), inertia), h);
                let after = flow
                    .add(rel_n.scale(fx::div(ONE, ONE + k_n)))
                    .add(rel_t.scale(fx::div(ONE, ONE + k_t)));
                let drag = after.sub(v).scale(fx::div(inertia, h));
                felt[i] = if is_end { V3::ZERO } else { f.add(drag) };
                v = after;
                self.vel[i] = v;
                pred[i] = self.pts[i].add(v.scale(h));
                // the bottom: a point that comes down on it rests there; one that
                // runs into a bank's or a rock's side from the water is put back
                // in the water beside it, not lifted onto the top (T16)
                let bed = sea.bed_at(pred[i].x, pred[i].z);
                if pred[i].y < bed {
                    let push = if self.pts[i].y < bed {
                        sea.push_out(pred[i], 0).flat()
                    } else {
                        V3::ZERO
                    };
                    if push == V3::ZERO {
                        shift[i] = v3(0, bed - pred[i].y, 0);
                        pred[i].y = bed;
                        self.vel[i].y = 0;
                    } else {
                        shift[i] = push;
                        pred[i] = pred[i].add(push);
                    }
                }
            }
            let reach: Fx = pred.windows(2).map(|w| w[1].sub(w[0]).len()).sum();
            let longest = s.longest_out_m * ONE;
            if !pinned {
                // the bail is open and no finger holds the line: it comes off the
                // spool as the lure pulls it. Only the lure's own distance pulls:
                // line that waves, wind or current belly out does not turn the
                // spool, or it would ratchet off at every swell (T10)
                let straight = pred[n - 1].sub(tip_now).len();
                let allowed = straight + fx::mul(straight, s.free_slack_pct * ONE / 100);
                self.out = self.out.max(reach.min(allowed)).min(longest);
                if reach <= self.out {
                    // the chain is spread evenly along its new length, so nothing
                    // holds the lure back but the line's drag
                    let (p, v) = respace(&pred, &self.vel, self.out);
                    self.pts = p;
                    self.vel = v;
                    self.pts[0] = tip_now;
                    continue;
                }
                // a belly longer than the line out: the line holds it like a
                // pinned one, with no drag and no winding while the bail is open
            }
            let ea = self.stiffness();
            // while the motor pulls, the drag holds up to the motor's pull
            let motor = input.power && !self.bail_open;
            let slip_at = if motor {
                self.drag().max(deci(s.power_pull_dn))
            } else {
                self.drag()
            };
            if !self.bail_open && self.tension > slip_at {
                // the drag: when the pull passes its setting the spool gives the
                // stretch above it, (T − drag) × out / EA, so the pull falls back
                // to the setting
                let give = fx::div(fx::mul(self.tension - slip_at, self.out), ea);
                self.out += give / SUBSTEPS;
                self.slipping = true;
                // a run that empties the spool parts the line at the knot
                if self.out >= s.longest_out_m * ONE {
                    self.snapped = true;
                }
            }
            if motor && !self.slipping {
                // a stalled motor winds nothing
                self.out -= fx::mul(power_per_s, h);
            } else if input.wind && !self.bail_open && !self.slipping {
                self.out -= fx::mul(wind_per_s, h);
            } else if input.let_out && !self.bail_open {
                // backwinding: line goes out at the reel's own winding speed
                self.out += fx::mul(wind_per_s, h);
            }
            self.out = self.out.clamp(s.shortest_out_cm * ONE / 100, longest);
            let seg = self.seg();
            // XPBD: each segment a rope that can stretch (compliance α = seg / EA)
            // but never push. Multipliers in Q32: at these masses and steps a
            // hanging lure's is about 1e-5 kg·m, under one Q16 unit.
            let inv = |i: usize| -> i64 {
                if i == 0 {
                    0
                } else if i == n - 1 {
                    (1i64 << 32) / end.mass.max(1) as i64
                } else {
                    (1i64 << 32) / pm.max(floor_mass).max(1) as i64
                }
            };
            let w: Vec<i64> = (0..n).map(inv).collect();
            // the whole line's give, lumped on the end's tie to the tip: α = out / EA
            let alpha = (fx::div(self.out, ea) as i64 * per_h2) as i64; // Q16, per kg
            let gamma = (s.damping_pct as i64) * (ONE as i64) / 100; // Q16
            let mut seg_l = vec![0i64; n - 1]; // Q32 kg·m
            let mut tie_l = vec![0i64; n]; // Q32 kg·m
            for _ in 0..s.iterations {
                // neighbours: a rope between each pair of points, which can go slack
                for j in 0..n - 1 {
                    let d = pred[j + 1].sub(pred[j]);
                    let len = d.len();
                    let c = (len - seg) as i64;
                    if len == 0 || (c <= 0 && seg_l[j] == 0) {
                        continue;
                    }
                    let nrm = d.with_len(ONE);
                    // the last rope only moves the line: the end is held by its
                    // tie to the tip, which carries the line's whole give
                    let w_far = if j + 1 == n - 1 { 0 } else { w[j + 1] };
                    let wsum = w[j] + w_far;
                    if wsum == 0 {
                        continue;
                    }
                    let mut dl = (-(c << 16) << 16) / wsum;
                    if seg_l[j] + dl > 0 {
                        dl = -seg_l[j];
                    }
                    seg_l[j] += dl;
                    pred[j] = pred[j].sub(nrm.scale(((w[j] * dl) >> 32) as Fx));
                    pred[j + 1] = pred[j + 1].add(nrm.scale(((w_far * dl) >> 32) as Fx));
                    pred[0] = tip_now;
                }
                // long-range ties: no point farther from the tip than the line
                // between them; the end's tie stretches with the line
                for i in 1..n {
                    let d = pred[i].sub(tip_now);
                    let len = d.len();
                    let c = (len - seg * i as i32) as i64;
                    if len == 0 || (c <= 0 && tie_l[i] == 0) {
                        continue;
                    }
                    let nrm = d.with_len(ONE);
                    let (al, ga) = if i == n - 1 { (alpha, gamma) } else { (0, 0) };
                    let rel = if i == n - 1 {
                        pred[i].sub(old[i]).dot(nrm)
                    } else {
                        0
                    };
                    // dλ = (-C - α̃λ - γ ∇C·Δx) / ((1+γ) w + α̃), in Q32
                    let num = -(c << 16) - ((al * tie_l[i]) >> 16) - ga * rel;
                    let den = (((ONE as i64 + ga) * w[i]) >> 16) + al;
                    if den <= 0 {
                        continue;
                    }
                    let mut dl = (num << 16) / den;
                    if tie_l[i] + dl > 0 {
                        dl = -tie_l[i];
                    }
                    tie_l[i] += dl;
                    pred[i] = pred[i].add(nrm.scale(((w[i] * dl) >> 32) as Fx));
                }
            }
            for i in 1..n {
                let moved = pred[i].sub(old[i]).sub(shift[i]);
                self.vel[i] = v3(fx::div(moved.x, h), fx::div(moved.y, h), fx::div(moved.z, h));
            }
            // the pull on the tip: the end's tie, which carries the end's whole
            // load, plus the weight and drag of each point of line held at its
            // full reach from the tip (the ties of the line's own points also
            // fight the last rope, so their multipliers are not a force)
            let mut sum = (-tie_l[n - 1] * per_h2) >> 16;
            for i in 1..n - 1 {
                let d = pred[i].sub(tip_now);
                if d.len() >= seg * i as i32 - seg / 50 {
                    sum += felt[i].dot(d.with_len(ONE)).max(0);
                }
            }
            tension_sum += sum;
            self.pts = pred;
            self.pts[0] = tip_now;
        }
        self.prev_tip = tip;
        self.tension = (tension_sum / SUBSTEPS as i64).min(i32::MAX as i64) as Fx;
        self.load = self.tension;
        if self.load > self.break_tension() {
            self.snapped = true;
        }
        let along = self.pts[1].sub(self.pts[0]);
        along.with_len(self.tension)
    }
}

/// Points spread evenly along a chain's length, `length` long in all; each
/// point keeps the velocity of the place it came from.
fn respace(pts: &[V3], vel: &[V3], length: Fx) -> (Vec<V3>, Vec<V3>) {
    let n = pts.len();
    let seg = length / (n as i32 - 1);
    let mut out_p = vec![pts[0]; n];
    let mut out_v = vec![vel[0]; n];
    let (mut j, mut walked) = (0usize, 0);
    for (i, (op, ov)) in out_p.iter_mut().zip(out_v.iter_mut()).enumerate().skip(1) {
        let want = seg * i as i32;
        // walk the old chain to the segment that holds `want`
        loop {
            if j + 1 >= n {
                break;
            }
            let l = pts[j + 1].sub(pts[j]).len();
            if walked + l >= want || j + 2 >= n {
                let t = if l > 0 {
                    fx::div(want - walked, l).clamp(0, ONE)
                } else {
                    0
                };
                *op = pts[j].lerp(pts[j + 1], t);
                *ov = vel[j].lerp(vel[j + 1], t);
                break;
            }
            walked += l;
            j += 1;
        }
    }
    // the lure stays where it is
    out_p[n - 1] = pts[n - 1];
    out_v[n - 1] = vel[n - 1];
    (out_p, out_v)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::sea::{SeaSpec, WaveSpec};

    pub fn spec() -> LineSpec {
        LineSpec {
            points: 40,
            start_out_cm: 120,
            shortest_out_cm: 30,
            longest_out_m: 150,
            break_dn: 356,
            drag_dn: 71,
            diameter_um: 280,
            density_kg_m3: 1780,
            point_floor_mg: 400,
            lure_mass_g: 20,
            lure_volume_mm3: 3000,
            lure_area_mm2: 400,
            stretch_at_break_pct: 20,
            damping_pct: 20,
            wind_per_turn_cm: 60,
            power_wind_cm_s: 600,
            power_pull_dn: 260,
            turns_per_s_x10: 20,
            rod_stiffness_n_m: 25,
            rod_bend_max_cm: 60,
            iterations: 12,
            along_drag_pct: 2,
            free_slack_pct: 20,
        }
    }

    pub fn calm() -> Sea {
        let mut s = Sea::new(&SeaSpec {
            waves: vec![WaveSpec {
                toward_deg: 0,
                length_cm: 4000,
                amp_mm: 500,
                steep_pct: 0,
            }],
            tide_amp_mm: 0,
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
        s.current_on = false;
        s
    }

    fn high() -> V3 {
        v3(0, 3 * ONE, 0)
    }

    /// A tip under water, so a test's whole line is wet.
    fn deep() -> V3 {
        v3(0, -2 * ONE, 0)
    }

    /// A line laid out straight from a deep tip along +z, ready to be pulled.
    fn laid_out() -> Line {
        let mut l = Line::new(spec(), deep());
        let n = l.pts.len() as i32;
        let seg = l.out / (n - 1);
        for (i, p) in l.pts.iter_mut().enumerate() {
            *p = deep().add(v3(0, 0, seg * i as i32));
        }
        l
    }

    #[test]
    fn a_lure_hanging_still_in_air_pulls_its_own_weight() {
        // 0.020 kg × 9.81 = 0.196 N; with the rod's bend at 25 N/m it settles
        let sea = calm();
        let mut l = Line::new(spec(), high());
        let lure = l.lure();
        for _ in 0..240 {
            l.tick(&sea, high(), LineInput::default(), lure);
        }
        let want = 12_858;
        assert!((l.tension - want).abs() < want / 20, "tension {}", l.tension);
    }

    #[test]
    fn the_line_never_stretches_past_what_is_paid_out() {
        let sea = calm();
        let mut l = laid_out();
        // a fish of 3 kg swimming away hard; the line is pinned by a finger
        let fish = EndLoad {
            mass: 3 * ONE,
            volume: 3 * ONE,
            area: ONE / 100,
            force: v3(0, 0, 30 * ONE),
        };
        let input = LineInput {
            finger: true,
            wind: false,
            power: false,
            ..LineInput::default()
        };
        for t in 0..240 {
            let fish = EndLoad {
                force: fish.force.ratio(t.min(120), 120),
                ..fish
            };
            l.tick(&sea, deep(), input, fish);
            // stiff line and a pinned spool: at most the stretch at the
            // breaking tension, 20% of the line out, and only while it holds
            let most = l.out + l.out / 5;
            assert!(
                l.snapped || l.chain_length() <= most,
                "{} > {}",
                l.chain_length(),
                most
            );
        }
    }

    #[test]
    fn a_pull_over_the_breaking_tension_snaps_the_line_and_one_under_does_not() {
        let sea = calm();
        let input = LineInput {
            finger: true,
            wind: false,
            power: false,
            ..LineInput::default()
        };
        // a heavy end pulled sideways by a steady force: the tension settles at that force
        let held = |newtons: i32| {
            let mut l = laid_out();
            let end = EndLoad {
                mass: 2 * ONE,
                volume: 2 * ONE,
                area: ONE / 50,
                force: v3(0, 0, newtons * ONE),
            };
            // a fish's pull builds over two seconds; a load dropped on all at
            // once would ring the line to twice its size
            for t in 0..600 {
                let ramp = EndLoad {
                    force: end.force.ratio(t.min(120), 120),
                    ..end
                };
                l.tick(&sea, deep(), input, ramp);
            }
            l
        };
        // 35.6 N breaks; a 2 kg end of 2 litres in sea water floats up with 0.49 N
        assert!(!held(34).snapped, "34 N held");
        assert!(held(37).snapped, "37 N snapped");
    }

    #[test]
    fn the_drag_lets_line_slip_above_its_setting() {
        let sea = calm();
        let mut l = laid_out();
        let end = EndLoad {
            mass: 2 * ONE,
            volume: 2 * ONE,
            area: ONE / 50,
            force: v3(0, 0, 15 * ONE),
        };
        let start = l.out;
        for t in 0..300 {
            let e = EndLoad {
                force: end.force.ratio(t.min(120), 120),
                ..end
            };
            l.tick(&sea, deep(), LineInput::default(), e);
        }
        assert!(l.out > start + ONE / 2, "line out {} from {start}", l.out);
        assert!(!l.snapped);
        // below the drag's 7.1 N it holds
        let mut l = laid_out();
        let end = EndLoad {
            force: v3(0, 0, 4 * ONE),
            ..end
        };
        for t in 0..300 {
            let e = EndLoad {
                force: end.force.ratio(t.min(120), 120),
                ..end
            };
            l.tick(&sea, deep(), LineInput::default(), e);
        }
        assert_eq!(l.out, start);
    }

    #[test]
    fn a_lure_let_go_at_speed_flies_out_and_a_held_one_does_not() {
        // the lure leaves at 12 m/s, 45 degrees up, from 1.8 m above the water;
        // in a vacuum it would land v²/g = 144 / 9.81 = 14.7 m out, a little
        // more for the height; the line's air drag takes some of that
        let cast = |finger: bool| {
            let sea = calm();
            let mut l = Line::new(spec(), high());
            let lure = l.lure();
            l.bail_open = true;
            let last = l.vel.len() - 1;
            l.vel[last] = v3(0, 12 * 7071 * (ONE / 10_000), 12 * 7071 * (ONE / 10_000));
            for _ in 0..240 {
                l.tick(
                    &sea,
                    high(),
                    LineInput {
                        finger,
                        ..LineInput::default()
                    },
                    lure,
                );
            }
            (l.end().z, l.out)
        };
        let (far, out) = cast(false);
        assert!(far > 10 * ONE, "lure at {far}, line out {out}");
        assert!(out >= far, "the line out covers the lure's distance");
        let (near, out) = cast(true);
        assert!(
            near < 2 * ONE && out == 120 * ONE / 100,
            "a held line keeps the lure close: {near}"
        );
    }

    #[test]
    fn the_power_wind_brings_in_a_pull_the_drag_would_give_to() {
        // a steady 15 N pull: over the 7.1 N drag, so winding by hand slips,
        // but under the motor's 26 N, so the power wind still brings line in
        let pulled = |input: LineInput| {
            let sea = calm();
            let mut l = laid_out();
            let start = l.out;
            let end = EndLoad {
                mass: 2 * ONE,
                volume: 2 * ONE,
                area: ONE / 50,
                force: v3(0, 0, 15 * ONE),
            };
            for t in 0..240 {
                let e = EndLoad {
                    force: end.force.ratio(t.min(120), 120),
                    ..end
                };
                l.tick(&sea, deep(), input, e);
            }
            l.out - start
        };
        let by_hand = pulled(LineInput {
            finger: false,
            wind: true,
            power: false,
            ..LineInput::default()
        });
        let motor = pulled(LineInput {
            finger: false,
            wind: false,
            power: true,
            ..LineInput::default()
        });
        assert!(by_hand > 0, "by hand the drag gives line: {by_hand}");
        assert!(motor < 0, "the motor brings line in: {motor}");
    }

    #[test]
    fn a_run_that_empties_the_spool_parts_the_line() {
        // a 15 N run against the 7.1 N drag takes line until the spool, here
        // only 2 m, is empty; then the line parts
        let sea = calm();
        let mut l = laid_out();
        l.spec.longest_out_m = 2;
        let end = EndLoad {
            mass: 2 * ONE,
            volume: 2 * ONE,
            area: ONE / 50,
            force: v3(0, 0, 15 * ONE),
        };
        for t in 0..600 {
            let e = EndLoad {
                force: end.force.ratio(t.min(120), 120),
                ..end
            };
            l.tick(&sea, deep(), LineInput::default(), e);
            if l.snapped {
                break;
            }
        }
        assert!(l.snapped, "line out {}", l.out);
    }

    #[test]
    fn winding_shortens_the_line_out() {
        // 0.6 m a turn at 2 turns a second: 1.2 m a second
        let sea = calm();
        let mut l = Line::new(spec(), high());
        l.out = 10 * ONE;
        let lure = l.lure();
        for _ in 0..60 {
            l.tick(
                &sea,
                high(),
                LineInput {
                    finger: false,
                    wind: true,
                    power: false,
                    ..LineInput::default()
                },
                lure,
            );
        }
        let wound = 10 * ONE - l.out;
        assert!((wound - 12 * ONE / 10).abs() < ONE / 50, "wound {wound}");
    }
}
