//! The look: `data/look.json`'s presentation numbers, turned into shader
//! uniforms (name, size, Q16 values) for each program, the colours the
//! scene's shapes use, and the camera's distances. Nothing here is a rule.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::render::{hex, Rgb};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Colour(String),
    Number(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sun {
    pub toward_deg: i32,
    pub elevation_deg: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraLook {
    pub behind_cm: i32,
    pub above_cm: i32,
    pub aside_cm: i32,
    pub look_ahead_cm: i32,
    pub focal_x1000: i32,
    pub follow_pct: i32,
    pub scope_eye_cm: [i32; 3],
    pub scope_min_zoom: i32,
    pub scope_max_zoom: i32,
    pub scope_start_zoom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    #[serde(default, rename = "_note")]
    pub note: String,
    pub sun: Sun,
    pub shapes: BTreeMap<String, Vec<Value>>,
    pub water: BTreeMap<String, Vec<Value>>,
    pub sky: BTreeMap<String, Vec<Value>>,
    pub camera: CameraLook,
    pub colours: BTreeMap<String, String>,
    pub line_glow: i32,
    pub lure_glow: i32,
    /// How bright a rain streak is, thousandths.
    pub rain_glow: i32,
    pub grid_cells: i32,
    pub grid_radius_m: i32,
    /// The speed through the water at which the wake is drawn whole.
    pub wake_full_cm_s: i32,
    /// Each place's changes to the look above, by the place's name. A
    /// change may only replace a value the look already has.
    #[serde(default)]
    pub places: BTreeMap<String, LookPatch>,
    /// A boat's own camera, by the boat's name; the others use `camera`.
    #[serde(default)]
    pub boat_cameras: BTreeMap<String, CameraLook>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LookPatch {
    #[serde(default)]
    pub sun: Option<Sun>,
    #[serde(default)]
    pub shapes: BTreeMap<String, Vec<Value>>,
    #[serde(default)]
    pub water: BTreeMap<String, Vec<Value>>,
    #[serde(default)]
    pub sky: BTreeMap<String, Vec<Value>>,
    #[serde(default)]
    pub colours: BTreeMap<String, String>,
}

/// A uniform's name and its Q16 values.
pub type Uniform = (String, Vec<i32>);

impl Look {
    /// The look at a place, from a boat: the place's changes laid over the
    /// common look, and the boat's camera. A change to a name the common look
    /// lacks is an error, so every place sends the shaders the same uniforms.
    pub fn at(&self, place: &str, weather: &str, boat: &str) -> Result<Look, String> {
        let mut l = self.clone();
        // the place's changes, then its weather's over them
        for p in [place, weather].iter().filter_map(|k| self.places.get(*k)) {
            if let Some(sun) = &p.sun {
                l.sun = sun.clone();
            }
            for (table, patch, what) in [
                (&mut l.shapes, &p.shapes, "shapes"),
                (&mut l.water, &p.water, "water"),
                (&mut l.sky, &p.sky, "sky"),
            ] {
                for (k, v) in patch {
                    match table.get(k) {
                        Some(old) if old.len() == v.len() => {
                            table.insert(k.clone(), v.clone());
                        }
                        _ => return Err(format!("data/look.json: {place} or {weather} changes {what}.{k}, which the common look does not have in that size.")),
                    }
                }
            }
            for (k, v) in &p.colours {
                if !l.colours.contains_key(k) {
                    return Err(format!("data/look.json: {place} or {weather} changes the colour {k}, which the common look does not have."));
                }
                l.colours.insert(k.clone(), v.clone());
            }
        }
        if let Some(c) = self.boat_cameras.get(boat) {
            l.camera = c.clone();
        }
        Ok(l)
    }

    /// The direction toward the sun.
    pub fn sun_dir(&self) -> V3 {
        let yaw = self.sun.toward_deg * fx::TURN / 360;
        let up = self.sun.elevation_deg * fx::TURN / 360;
        let flat = fx::cos(up);
        v3(
            fx::mul(fx::sin(yaw), flat),
            fx::sin(up),
            fx::mul(fx::cos(yaw), flat),
        )
    }

    /// One program's constant uniforms, with the sun's direction added.
    pub fn constants(&self, table: &BTreeMap<String, Vec<Value>>) -> Vec<Uniform> {
        let mut out: Vec<Uniform> = table
            .iter()
            .map(|(name, vals)| {
                let mut v = Vec::new();
                for x in vals {
                    match x {
                        Value::Colour(h) => v.extend_from_slice(&hex(h)),
                        Value::Number(n) => v.push(n * (ONE / 1000) + n * (ONE % 1000) / 1000),
                    }
                }
                (name.clone(), v)
            })
            .collect();
        let d = self.sun_dir();
        out.push(("uSunDir".into(), vec![d.x, d.y, d.z]));
        out
    }

    pub fn colour(&self, name: &str) -> Rgb {
        self.colours.get(name).map(|h| hex(h)).unwrap_or([ONE, 0, ONE])
    }

    /// A list's first value, Q16, or zero.
    pub fn number(table: &BTreeMap<String, Vec<Value>>, name: &str, i: usize) -> Fx {
        match table.get(name).and_then(|v| v.get(i)) {
            Some(Value::Number(n)) => n * (ONE / 1000) + n * (ONE % 1000) / 1000,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousandths_and_colours_become_q16_uniforms() {
        let look: Look = serde_json::from_str(include_str!("../../../data/look.json")).unwrap();
        let c = look.constants(&look.shapes);
        let mix = c.iter().find(|(n, _)| n == "uMix").unwrap();
        // 640 thousandths is 0.64: 41 943 in Q16 (0.64 × 65536 = 41 943.04)
        assert!((mix.1[0] - 41_943).abs() <= 2, "{:?}", mix.1);
        let sky = c.iter().find(|(n, _)| n == "uSkyLight").unwrap();
        assert_eq!(sky.1.len(), 3);
        let sun = c.iter().find(|(n, _)| n == "uSunDir").unwrap();
        // 24 degrees up: sin 24° = 0.4067
        assert!((sun.1[1] - 26_655).abs() < 150, "{:?}", sun.1);
    }
}
