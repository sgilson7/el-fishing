//! Trolling (T26): a boat that tows lures. Several rods stand in holders on
//! the boat, each with its own simulated line streaming behind; the boat's
//! speed and each lure's weight and lip decide how deep each runs. One rod
//! at a time is in play: the reel keys work its line, and a fish that
//! strikes another rod's lure puts that rod in play, as an angler grabs the
//! rod whose reel screams. The line is the same line as a cast rod's
//! (`line.rs`); only where its tip is and what hangs on its end differ.

use serde::Deserialize;

use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::line::{EndLoad, Line, LineSpec};

/// A rod holder on the boat.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RodSpot {
    /// The key of its name in the strings file.
    pub name: String,
    /// Where the line leaves the boat in hull space: a rod tip, or an
    /// outrigger's clip far out to the side. cm.
    pub tip_cm: [i32; 3],
    /// How far back its lure starts, m.
    pub back_m: i32,
    /// Which lure it pulls.
    pub lure: usize,
}

/// A trolling lure. A lipped plug dives: the water on its lip pushes it
/// down with a force that grows as the square of its speed through the water.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrollLure {
    pub name: String,
    pub mass_g: i32,
    pub volume_mm3: i32,
    pub area_mm2: i32,
    /// The lip's push down at 3 m/s through the water, tenths of a newton.
    pub dive_dn_at_3: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrollSpec {
    #[serde(default, rename = "_note")]
    pub note: String,
    pub line: LineSpec,
    pub rods: Vec<RodSpot>,
    pub lures: Vec<TrollLure>,
}

fn cm3(c: [i32; 3]) -> V3 {
    v3(c[0] * ONE / 100, c[1] * ONE / 100, c[2] * ONE / 100)
}

impl TrollSpec {
    pub fn tip_local(&self, rod: usize) -> V3 {
        self.rods.get(rod).map(|r| cm3(r.tip_cm)).unwrap_or(V3::ZERO)
    }

    /// The line of rod `rod`, its lure `back_m` behind the tip along `behind`
    /// (a unit vector), lying on the water.
    pub fn laid_line(&self, rod: usize, tip: V3, behind: V3) -> Line {
        let spot = &self.rods[rod];
        let lure = &self.lures[spot.lure.min(self.lures.len().saturating_sub(1))];
        let mut spec = self.line.clone();
        spec.start_out_cm = spot.back_m * 100;
        spec.lure_mass_g = lure.mass_g;
        spec.lure_volume_mm3 = lure.volume_mm3;
        spec.lure_area_mm2 = lure.area_mm2;
        let mut l = Line::new(spec, tip);
        let n = l.pts.len() as i32;
        let seg = l.out / (n - 1);
        for (i, p) in l.pts.iter_mut().enumerate() {
            let along = behind.scale(seg * i as i32);
            *p = v3(
                tip.x + along.x,
                (tip.y - ONE * i as i32 / 8).max(0),
                tip.z + along.z,
            );
        }
        l
    }

    /// What hangs on rod `rod`'s line: its lure, pushed down by its lip in
    /// proportion to the square of its speed through the water.
    pub fn load(&self, rod: usize, line: &Line, through_water: V3) -> EndLoad {
        let mut end = line.lure();
        let spot = &self.rods[rod];
        if let Some(lure) = self.lures.get(spot.lure) {
            let three = 3 * ONE;
            let k = fx::div(through_water.len(), three);
            end.force = v3(0, -fx::mul(lure.dive_dn_at_3 * ONE / 10, fx::mul(k, k)), 0);
        }
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::Fx;
    use crate::line::LineInput;
    use crate::sea::{Sea, SeaSpec};

    fn spec() -> TrollSpec {
        let gear: serde_json::Value = serde_json::from_str(include_str!("../../../data/gear.json")).unwrap();
        serde_json::from_value(gear["trolling"].clone()).unwrap()
    }

    fn calm() -> Sea {
        let mut s = Sea::new(&SeaSpec {
            waves: vec![],
            tide_amp_mm: 0,
            tide_period_minutes: 20,
            current_toward_deg: 0,
            current_mm_s: 0,
            wind_toward_deg: 0,
            wind_mm_s: 0,
            density_kg_m3: 1025,
            depth_cm: 10_000,
            river: None,
        });
        s.waves_on = false;
        s
    }

    /// Rod `rod` towed from a tip at the surface moving toward +z at `speed`
    /// for `ticks`; returns the line.
    fn towed(rod: usize, speed: Fx, ticks: i32) -> Line {
        let t = spec();
        let sea = calm();
        let mut tip = v3(0, ONE / 2, 0);
        let mut l = t.laid_line(rod, tip, v3(0, 0, -ONE));
        let mut prev = l.end();
        for _ in 0..ticks {
            tip = tip.add(v3(0, 0, speed / 60));
            let e = l.end();
            let vel = e.sub(prev).scale(60 * ONE);
            prev = e;
            let load = t.load(rod, &l, vel);
            l.tick(&sea, tip, LineInput::default(), load);
        }
        l
    }

    /// The angle below level of the chord from the rod tip to the lure: the
    /// line's end is held by its tie to the tip, so the forces on the lure
    /// line up along that chord.
    fn chord_deg(l: &Line) -> i32 {
        let d = l.end().sub(l.pts[0]);
        crate::geom::yaw_of(-d.y, d.flat_len()) * 360 / fx::TURN
    }

    #[test]
    fn a_towed_plug_settles_where_its_lip_and_weight_balance_its_drag() {
        // the diving plug (data/gear.json): 180 g in 90 cm³, so its weight in sea
        // water is (0.180 − 0.0923) × 9.81 = 0.86 N; its lip pushes down 6.0 N at
        // 3 m/s; its drag is ρ × ½ A × v² = 1025 × 0.0012 × 9 = 11.1 N. The chord
        // to it leans down at atan(6.86 / 11.1) = 31.7°, so 20 m back it runs
        // about 20 × sin 31.7° = 10.5 m down
        let l = towed(2, 3 * ONE, 1800);
        let angle = chord_deg(&l);
        assert!(
            (angle - 32).abs() <= 3,
            "the chord to the plug leans {angle} degrees"
        );
    }

    #[test]
    fn a_skirted_lure_rides_shallow_at_trolling_speed() {
        // 250 g in 120 cm³ weighs (0.250 − 0.123) × 9.81 = 1.25 N in the water and
        // drags 1025 × 0.0015 × 9 = 13.8 N at 3 m/s: the chord leans
        // atan(1.25 / 13.8) = 5.2°, so 45 m back it runs about 4 m down
        let l = towed(0, 3 * ONE, 1800);
        let angle = chord_deg(&l);
        assert!(
            (angle - 5).abs() <= 2,
            "the chord to the skirt leans {angle} degrees"
        );
        let behind = l.pts[0].z - l.end().z;
        assert!(behind > 40 * ONE, "and streams {behind} behind the tip");
    }
}
