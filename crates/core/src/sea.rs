//! The sea: the layers a player feels (the water tab's L0 to L2), in Q16
//! metres. Level is the tide; waves are a Gerstner sum; current is one
//! water velocity for the place. The GPU draws the same waves from the
//! parameters `uniforms` returns, so what is drawn and what floats agree.
//!
//! Each wave keeps its phase as a wrapping 32-bit binary angle stepped
//! once a tick (SECOND-ORDER R1.10). Neither the core nor the shader keeps
//! a growing time: the shader is sent the phase.

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::river::{River, RiverSpec};
use crate::ticks::TICKS_PER_SECOND;

/// Gravity at the sea surface, 9.81 m/s², in Q16.
pub const G: Fx = 642_908;
/// A full turn in radians, 6.283185, in Q16.
pub const TAU: Fx = 411_775;

/// One wave as a place's data file gives it, in whole units.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveSpec {
    /// The compass direction the wave travels toward, in degrees.
    pub toward_deg: i32,
    pub length_cm: i32,
    pub amp_mm: i32,
    /// How sharp the crests are, 0 to 100: the share of the steepest a
    /// Gerstner wave can be before it loops over itself.
    pub steep_pct: i32,
}

/// A place's sea, as its data file gives it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeaSpec {
    pub waves: Vec<WaveSpec>,
    pub tide_amp_mm: i32,
    pub tide_period_minutes: i32,
    pub current_toward_deg: i32,
    pub current_mm_s: i32,
    pub wind_toward_deg: i32,
    pub wind_mm_s: i32,
    pub density_kg_m3: i32,
    pub depth_cm: i32,
    /// A river's banks, bed and rocks; its current runs at `current_mm_s`
    /// mid-channel, round the loop (T16). The open sea has none.
    #[serde(default)]
    pub river: Option<RiverSpec>,
}

/// One wave, ready to evaluate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    /// Unit direction of travel (x, z).
    pub dx: Fx,
    pub dz: Fx,
    pub length: Fx,
    pub amp: Fx,
    /// Horizontal amplitude of the crest's sideways motion.
    pub sway: Fx,
    /// k × A: the largest slope this wave gives.
    pub slope: Fx,
    /// ωA: how fast the water at the surface circles under this wave.
    pub orbit: Fx,
    /// Binary angle added each tick.
    pub step: u32,
    pub phase: u32,
}

/// The period of a deep-water wave of length `length`: √(2πL/g), Q16 s.
pub fn period(length: Fx) -> Fx {
    fx::sqrt(fx::div(fx::mul(TAU, length), G))
}

/// The binary-angle step a tick for a wave of period `period` (Q16 s).
pub fn step_for_period(period: Fx) -> u32 {
    if period <= 0 {
        return 0;
    }
    ((1u64 << 48) / (period as u64 * TICKS_PER_SECOND as u64)) as u32
}

/// Compass degrees to a unit (x, z): 0 is +z, 90 is +x.
pub fn heading(deg: i32) -> (Fx, Fx) {
    let a = deg * fx::TURN / 360;
    (fx::sin(a), fx::cos(a))
}

impl Wave {
    pub fn new(s: &WaveSpec, count: usize) -> Wave {
        let (dx, dz) = heading(s.toward_deg);
        let length = s.length_cm.max(1) * ONE / 100;
        let amp = s.amp_mm * ONE / 1000;
        let k = fx::div(TAU, length);
        let steep = s.steep_pct.clamp(0, 100) * ONE / 100;
        // sway = steep / (k N): N waves at their sharpest together still do not loop
        let sway = fx::div(steep, k * count.max(1) as i32);
        let t = period(length);
        Wave {
            dx,
            dz,
            length,
            amp,
            sway,
            slope: fx::mul(k, amp),
            orbit: if t > 0 { fx::div(fx::mul(TAU, amp), t) } else { 0 },
            step: step_for_period(t),
            phase: 0,
        }
    }

    /// The wave's angle at a horizontal point, in turns (`fx::TURN`).
    pub fn angle(&self, x: Fx, z: Fx) -> i32 {
        let along = fx::mul(self.dx, x) + fx::mul(self.dz, z);
        fx::div(along, self.length) - (self.phase >> 16) as i32
    }
}

/// The surface at a point: its height and its slope along x and z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    pub height: Fx,
    pub slope_x: Fx,
    pub slope_z: Fx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sea {
    pub waves: Vec<Wave>,
    pub tide_amp: Fx,
    pub tide_step: u32,
    pub tide_phase: u32,
    pub current: V3,
    pub wind: V3,
    pub density: Fx,
    pub depth: Fx,
    pub river: Option<River>,
    /// Layer switches, for the tests that turn one off.
    pub waves_on: bool,
    pub tide_on: bool,
    pub current_on: bool,
}

fn along(deg: i32, mm_s: i32) -> V3 {
    let (x, z) = heading(deg);
    let speed = mm_s * ONE / 1000;
    v3(fx::mul(x, speed), 0, fx::mul(z, speed))
}

impl Sea {
    pub fn new(s: &SeaSpec) -> Sea {
        let n = s.waves.len();
        let tide_period = s.tide_period_minutes.max(1) * 60 * ONE;
        Sea {
            waves: s.waves.iter().map(|w| Wave::new(w, n)).collect(),
            tide_amp: s.tide_amp_mm * ONE / 1000,
            tide_step: step_for_period(tide_period),
            tide_phase: 0,
            current: along(s.current_toward_deg, s.current_mm_s),
            wind: along(s.wind_toward_deg, s.wind_mm_s),
            density: s.density_kg_m3 * ONE,
            depth: s.depth_cm * ONE / 100,
            river: s
                .river
                .as_ref()
                .map(|r| River::new(r, s.current_mm_s, s.depth_cm * ONE / 100)),
            waves_on: true,
            tide_on: true,
            current_on: true,
        }
    }

    /// Advances every phase by one tick.
    pub fn tick(&mut self) {
        for w in &mut self.waves {
            w.phase = w.phase.wrapping_add(w.step);
        }
        self.tide_phase = self.tide_phase.wrapping_add(self.tide_step);
    }

    /// The still-water level: the tide.
    pub fn level(&self) -> Fx {
        if !self.tide_on {
            return 0;
        }
        fx::mul(self.tide_amp, fx::sin((self.tide_phase >> 16) as i32))
    }

    /// True while the tide is rising.
    pub fn tide_rising(&self) -> bool {
        fx::cos((self.tide_phase >> 16) as i32) >= 0
    }

    /// The sideways shift the waves give the surface point that started
    /// above (x, z).
    fn sway(&self, x: Fx, z: Fx) -> (Fx, Fx) {
        let (mut sx, mut sz) = (0, 0);
        for w in &self.waves {
            let c = fx::mul(w.sway, fx::cos(w.angle(x, z)));
            sx += fx::mul(c, w.dx);
            sz += fx::mul(c, w.dz);
        }
        (sx, sz)
    }

    /// The surface over a horizontal point. A Gerstner surface moves its
    /// points sideways, so the point now above (x, z) started about one
    /// sway back; one step back is the same correction the water tab
    /// describes, and the shader draws the displaced grid it corrects for.
    pub fn surface(&self, x: Fx, z: Fx) -> Surface {
        let level = self.level();
        if !self.waves_on {
            return Surface {
                height: level,
                slope_x: 0,
                slope_z: 0,
            };
        }
        let (sx, sz) = self.sway(x, z);
        let (x0, z0) = (x - sx, z - sz);
        let mut s = Surface {
            height: level,
            slope_x: 0,
            slope_z: 0,
        };
        for w in &self.waves {
            let a = w.angle(x0, z0);
            s.height += fx::mul(w.amp, fx::sin(a));
            let c = fx::mul(w.slope, fx::cos(a));
            s.slope_x += fx::mul(c, w.dx);
            s.slope_z += fx::mul(c, w.dz);
        }
        s
    }

    pub fn height(&self, x: Fx, z: Fx) -> Fx {
        self.surface(x, z).height
    }

    /// The current at a point: the place's one current on the open sea, a
    /// river's flow round its banks and rocks.
    pub fn current_at(&self, x: Fx, z: Fx) -> V3 {
        if !self.current_on {
            return V3::ZERO;
        }
        match &self.river {
            Some(r) => r.current(v3(x, 0, z)),
            None => self.current,
        }
    }

    /// The height of the bottom under a point.
    pub fn bed_at(&self, x: Fx, z: Fx) -> Fx {
        match &self.river {
            Some(r) => r.bed(v3(x, 0, z)),
            None => -self.depth,
        }
    }

    /// How far to move a point to keep it `margin` clear of the banks and
    /// rocks; zero on the open sea.
    pub fn push_out(&self, p: V3, margin: Fx) -> V3 {
        match &self.river {
            Some(r) => r.push_out(p, margin),
            None => V3::ZERO,
        }
    }

    /// The water's velocity at the surface over (x, z): the current plus
    /// the waves' orbital motion, which runs forward under a crest and
    /// back under a trough (linear deep-water theory, u = ωA sin θ).
    pub fn surface_velocity(&self, x: Fx, z: Fx) -> V3 {
        let mut v = self.current_at(x, z);
        if self.waves_on {
            for w in &self.waves {
                let u = fx::mul(w.orbit, fx::sin(w.angle(x, z)));
                v.x += fx::mul(u, w.dx);
                v.z += fx::mul(u, w.dz);
            }
        }
        v
    }

    /// The significant wave height, Hs = 4 √(Σa²/2).
    pub fn significant_height(&self) -> Fx {
        let sum: Fx = self.waves.iter().map(|w| fx::mul(w.amp, w.amp)).sum();
        4 * fx::sqrt(sum / 2)
    }

    /// The period of the tallest wave, Q16 s.
    pub fn peak_period(&self) -> Fx {
        self.waves
            .iter()
            .max_by_key(|w| w.amp)
            .map(|w| period(w.length))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave(toward_deg: i32, length_cm: i32, amp_mm: i32, steep_pct: i32) -> WaveSpec {
        WaveSpec {
            toward_deg,
            length_cm,
            amp_mm,
            steep_pct,
        }
    }

    fn sea(waves: Vec<WaveSpec>) -> Sea {
        Sea::new(&SeaSpec {
            waves,
            tide_amp_mm: 600,
            tide_period_minutes: 20,
            current_toward_deg: 90,
            current_mm_s: 206,
            wind_toward_deg: 0,
            wind_mm_s: 0,
            density_kg_m3: 1025,
            depth_cm: 1800,
            river: None,
        })
    }

    #[test]
    fn a_ten_second_wave_steps_a_six_hundredth_of_a_turn_a_tick() {
        // L = g T² / 2π = 9.81 × 100 / 6.2832 = 156.13 m for T = 10 s
        let w = Wave::new(&wave(0, 15613, 1000, 0), 1);
        let p = period(w.length);
        assert!((p - 10 * ONE).abs() < ONE / 1000, "period {p}");
        // 2^32 / 600 = 7 158 278.8
        assert!((w.step as i64 - 7_158_279).abs() < 7_158, "step {}", w.step);
        // 600 ticks later the phase is back near zero: within 0.1% of a turn
        let mut s = sea(vec![wave(0, 15613, 1000, 0)]);
        for _ in 0..600 {
            s.tick();
        }
        let off = s.waves[0].phase.wrapping_add(1 << 31).wrapping_sub(1 << 31) as i32;
        assert!((off as i64).abs() < (1i64 << 32) / 1000, "phase {off}");
    }

    #[test]
    fn a_crest_stands_a_quarter_wavelength_along_at_phase_zero() {
        // A = 1 m, L = 10 m, flat crests: 2.5 m along is a quarter turn, sin = 1
        let mut s = sea(vec![wave(90, 1000, 1000, 0)]);
        s.tide_on = false;
        let h = s.height(5 * ONE / 2, 0);
        assert!((h - ONE).abs() < ONE / 500, "height {h}");
        // and a trough three quarters along
        let h = s.height(15 * ONE / 2, 0);
        assert!((h + ONE).abs() < ONE / 500, "height {h}");
    }

    #[test]
    fn the_slope_is_steepest_where_the_surface_crosses_its_level() {
        // slope at x = 0 is k A = 2π/10 × 1 = 0.6283
        let mut s = sea(vec![wave(90, 1000, 1000, 0)]);
        s.tide_on = false;
        let sf = s.surface(0, 0);
        assert!((sf.slope_x - 41_177).abs() < 200, "slope {}", sf.slope_x);
        assert_eq!(sf.slope_z, 0);
    }

    #[test]
    fn the_open_swell_components_give_a_two_metre_sea() {
        // Σa² = 0.3025 + 0.1225 + 0.0625 + 0.0324 + 0.01 + 0.0036 + 0.0009
        // + 0.000225 = 0.534625; Hs = 4 √(0.2673) = 2.068 m
        let s = sea([550, 350, 250, 180, 100, 60, 30, 15]
            .iter()
            .map(|&a| wave(0, 4000, a, 0))
            .collect());
        let hs = s.significant_height();
        assert!((hs - 135_528).abs() < 300, "Hs {hs}");
    }

    #[test]
    fn the_tide_reaches_its_amplitude_a_quarter_period_in() {
        let mut s = sea(vec![]);
        assert_eq!(s.level(), 0);
        // 20 minutes is 72 000 ticks; a quarter is 18 000
        for _ in 0..18_000 {
            s.tick();
        }
        let l = s.level();
        assert!((l - 600 * ONE / 1000).abs() < ONE / 500, "level {l}");
        s.tide_on = false;
        assert_eq!(s.level(), 0);
    }

    #[test]
    fn the_current_flows_toward_its_heading() {
        // 206 mm/s toward 90 degrees: along +x
        let s = sea(vec![]);
        let c = s.current_at(40 * ONE, -7 * ONE);
        assert!((c.x - 206 * ONE / 1000).abs() < 30 && c.z.abs() < 30, "{c:?}");
    }
}
