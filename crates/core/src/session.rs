//! A visit: the screens, the HUD, the camera and the scope around one
//! world, and the numbers the page draws each frame. The world holds every
//! rule of the water and the fish; the session holds what is only seen, so
//! the scope, the camera and the HUD cannot change a rule (plan decision 17).

use serde_json::{json, Value};

use crate::fish::Mood;
use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::glossary;
use crate::look::{Look, Uniform};
use crate::mesh::{self, Shape, SHAPES, VERTEX_VALUES};
use crate::render::{Camera, FrameOut, Pass, Program, COMMAND_VALUES, INSTANCE_VALUES};
use crate::scene;
use crate::strings::Strings;
use crate::ticks::{Clock, TICKS_PER_SECOND};
use crate::world::{bits, Event, GearSpec, PlaceSpec, World};

/// The screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Fishing,
    Landed,
    Settings,
}

impl Screen {
    pub fn name(self) -> &'static str {
        match self {
            Screen::Title => "title",
            Screen::Fishing => "fishing",
            Screen::Landed => "landed",
            Screen::Settings => "settings",
        }
    }
}

/// The per-frame uniforms, by program: (program, name, size).
pub fn frame_layout(waves: usize) -> Vec<(Program, &'static str, usize)> {
    vec![
        (Program::Sky, "uFwd", 3),
        (Program::Sky, "uRight", 3),
        (Program::Sky, "uUp", 3),
        (Program::Shapes, "uVP", 16),
        (Program::Shapes, "uEye", 3),
        (Program::Shapes, "uLevel", 1),
        (Program::Shapes, "uClarity", 1),
        (Program::Water, "uVP", 16),
        (Program::Water, "uEye", 3),
        (Program::Water, "uLevel", 1),
        (Program::Water, "uClarity", 1),
        (Program::Water, "uOrigin", 2),
        (Program::Water, "uTime", 1),
        (Program::Water, "uKayak", 4),
        (Program::Water, "uRing", 4),
        (Program::Water, "uWaveA", 4 * waves),
        (Program::Water, "uWaveB", 4 * waves),
    ]
}

pub struct Session {
    pub world: World,
    pub look: Look,
    pub strings: Strings,
    glossary_md: String,
    pub screen: Screen,
    clock: Clock,
    /// The last frame's key bits, for the scope.
    keys: u32,
    pub scope_zoom: i32,
    clarity: Fx,
    camera: Camera,
    pixels: (i32, i32),
    out: FrameOut,
    uniforms: Vec<i32>,
    /// The latest message and the tick it came at.
    message: Option<(String, u64)>,
    meshes: Vec<Vec<i32>>,
}

fn decimal(v: Fx, places: u32) -> String {
    let scale = 10i64.pow(places);
    let n = (v as i64 * scale + if v >= 0 { ONE as i64 / 2 } else { -(ONE as i64) / 2 }) / ONE as i64;
    let (whole, frac) = (n / scale, (n % scale).abs());
    let sign = if n < 0 && whole == 0 { "-" } else { "" };
    if places == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{frac:0width$}", width = places as usize)
    }
}

impl Session {
    pub fn new(
        strings: &str,
        gear: &str,
        place: &str,
        look: &str,
        glossary_md: &str,
        seed: u64,
    ) -> Result<Session, String> {
        let strings = Strings::parse(strings)?;
        let gear: GearSpec = serde_json::from_str(gear).map_err(|e| format!("data/gear.json: {e}"))?;
        let place: PlaceSpec = serde_json::from_str(place).map_err(|e| format!("data/place.json: {e}"))?;
        let look: Look = serde_json::from_str(look).map_err(|e| format!("data/look.json: {e}"))?;
        glossary::parse(glossary_md).map_err(|e| e.message())?;
        for f in &place.fish {
            if !strings.has(&f.name) {
                return Err(format!("The strings file has no \"{}\".", f.name));
            }
        }
        let mut meshes: Vec<Vec<i32>> = (0..SHAPES).map(|i| mesh::shape(shape_of(i))).collect();
        for f in &place.fish {
            meshes.push(mesh::fish_body(f));
        }
        let world = World::new(gear, place, seed);
        let camera = Camera {
            eye: v3(0, 3 * ONE, 6 * ONE),
            at: V3::ZERO,
            focal: look.camera.focal_x1000 * ONE / 1000,
        };
        let mut s = Session {
            world,
            scope_zoom: look.camera.scope_start_zoom,
            look,
            strings,
            glossary_md: glossary_md.to_string(),
            screen: Screen::Title,
            clock: Clock::default(),
            keys: 0,
            clarity: 0,
            camera,
            pixels: (640, 360),
            out: FrameOut::default(),
            uniforms: Vec::new(),
            message: None,
            meshes,
        };
        s.place_camera(true);
        s.build_frame();
        Ok(s)
    }

    pub fn meshes(&self) -> &[Vec<i32>] {
        &self.meshes
    }

    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> Vec<i32> {
        let w = (css_w.max(1) * dpr_pct.clamp(100, 200) / 100).max(1);
        let h = (css_h.max(1) * dpr_pct.clamp(100, 200) / 100).max(1);
        self.pixels = (w, h);
        self.build_frame();
        vec![w, h]
    }

    fn scoping(&self) -> bool {
        self.screen == Screen::Fishing && self.keys & bits::SCOPE != 0
    }

    /// One display frame: the ticks it owes, then the camera and the draw.
    pub fn frame(&mut self, now_us: i64, keys: u32) {
        self.keys = keys;
        let ticks = self.clock.frame(now_us, keys);
        if self.screen == Screen::Fishing {
            for k in ticks {
                self.world.step(k & !bits::SCOPE);
                for e in self.world.events.clone() {
                    self.hear(e);
                }
                if self.world.landed.is_some() {
                    self.screen = Screen::Landed;
                    break;
                }
            }
        }
        let want = if self.scoping() { ONE } else { 0 };
        self.clarity += (want - self.clarity) / 6;
        self.place_camera(false);
        self.build_frame();
    }

    fn fish_name(&self, i: usize) -> String {
        let s = self.world.fish[i].species;
        self.strings.get(&self.world.place.fish[s].name)
    }

    fn hear(&mut self, e: Event) {
        let say = |s: &Session, key: &str, fish: Option<usize>| {
            let name = fish.map(|i| s.fish_name(i)).unwrap_or_default();
            s.strings.fill(key, &[("fish", name)])
        };
        let text = match e {
            Event::Noticed(i) => say(self, "event.noticed", Some(i)),
            Event::Struck(i) => say(self, "event.struck", Some(i)),
            Event::Bit(_) => say(self, "event.bit", None),
            Event::Missed(i) => say(self, "event.missed", Some(i)),
            Event::Spat(i) => say(self, "event.spat", Some(i)),
            Event::HookSet(i) => say(self, "event.hook_set", Some(i)),
            Event::Threw(_) => say(self, "event.threw", None),
            Event::Jumped(_) => say(self, "event.jumped", None),
            Event::Snapped => say(self, "event.snapped", None),
            Event::Landed => return,
        };
        self.message = Some((text, self.world.tick));
    }

    /// Settings, screens and the catch, through one door.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match (name, value) {
            ("screen.show", "fishing") => {
                self.world.finish_catch(false);
                self.screen = Screen::Fishing;
            }
            ("screen.show", "settings") => self.screen = Screen::Settings,
            ("screen.show", "title") => self.screen = Screen::Title,
            ("screen.show", other) => return Err(format!("There is no screen \"{other}\".")),
            ("catch.release", _) | ("catch.keep", _) => {
                if self.screen != Screen::Landed {
                    return Err("There is no landed fish.".into());
                }
                self.world.finish_catch(name == "catch.keep");
                self.screen = Screen::Fishing;
                self.message = None;
            }
            ("settings.arm_speed", v) => self.world.arms.slow = v == "slow",
            ("settings.steady_hands", v) => self.world.arms.steady = level(v)?,
            ("settings.steady_hull", v) => self.world.hull.steady = level(v)?,
            ("menu.toggle", _) => {
                self.screen = match self.screen {
                    Screen::Fishing => Screen::Settings,
                    Screen::Settings => Screen::Fishing,
                    s => s,
                }
            }
            ("scope.zoom", v) => {
                let c = &self.look.camera;
                let step = if v.starts_with('-') { -1 } else { 1 };
                self.scope_zoom = (self.scope_zoom + step).clamp(c.scope_min_zoom, c.scope_max_zoom);
            }
            (other, _) => return Err(format!("Unknown action \"{other}\".")),
        }
        self.build_frame();
        Ok(())
    }

    fn place_camera(&mut self, snap: bool) {
        let c = &self.look.camera;
        let h = &self.world.hull;
        let cm = |v: i32| v * ONE / 100;
        if self.scoping() {
            // the angler's right eye, looking down the line at the lure or the fish on it
            let eye = h.to_world(v3(
                cm(c.scope_eye_cm[0]),
                cm(c.scope_eye_cm[1]),
                cm(c.scope_eye_cm[2]),
            ));
            let at = self.world.line.end();
            self.camera = Camera {
                eye,
                at,
                focal: c.focal_x1000 * ONE / 1000 * self.scope_zoom,
            };
            return;
        }
        let fwd = h.forward();
        let right = h.right();
        // the camera rides the hull's heave, a little behind and to the side
        let target_eye = h
            .pos
            .flat()
            .add(v3(0, h.pos.y.max(-ONE * 2), 0))
            .sub(fwd.scale(cm(c.behind_cm)))
            .add(right.scale(cm(c.aside_cm)))
            .add(v3(0, cm(c.above_cm), 0));
        let target_at = h
            .pos
            .flat()
            .add(fwd.scale(cm(c.look_ahead_cm)))
            .add(v3(0, ONE / 2, 0));
        let k = if snap { ONE } else { c.follow_pct * ONE / 100 };
        self.camera = Camera {
            eye: self.camera.eye.lerp(target_eye, k),
            at: self.camera.at.lerp(target_at, k),
            focal: c.focal_x1000 * ONE / 1000,
        };
    }

    fn build_frame(&mut self) {
        let (w, h) = self.pixels;
        let aspect = (w as i64 * ONE as i64 / h.max(1) as i64) as Fx;
        let cam = self.camera;
        let world = &self.world;
        let mut out = FrameOut::default();
        out.plain(Program::Sky, 0);
        let batch = scene::build(world, &self.look, self.meshes.len(), !self.scoping());
        batch.emit(Pass::Solid, &mut out);
        out.plain(Program::Water, 0);
        batch.emit(Pass::Glow, &mut out);
        self.out = out;
        // the per-frame uniforms, in frame_layout's order
        let rays = cam.rays(aspect);
        let vp = cam.matrix(aspect);
        let level = world.sea.level();
        let mut u: Vec<i32> = Vec::new();
        for r in rays {
            u.extend_from_slice(&[r.x, r.y, r.z]);
        }
        let eye = [cam.eye.x, cam.eye.y, cam.eye.z];
        u.extend_from_slice(&vp);
        u.extend_from_slice(&eye);
        u.extend_from_slice(&[level, self.clarity]);
        u.extend_from_slice(&vp);
        u.extend_from_slice(&eye);
        u.extend_from_slice(&[level, self.clarity]);
        // the grid follows the eye in whole metres
        u.extend_from_slice(&[cam.eye.x & !(ONE - 1), cam.eye.z & !(ONE - 1)]);
        let tick = world.tick as i64;
        u.push(((tick % (600 * TICKS_PER_SECOND)) * ONE as i64 / TICKS_PER_SECOND) as i32);
        let hl = &world.hull;
        let wake = fx::div(hl.speed(), ONE * 3 / 2).min(ONE);
        u.extend_from_slice(&[hl.pos.x, hl.pos.z, hl.heading, wake]);
        let ring = match world.last_splash {
            Some((at, t)) if world.tick - t < 4 * TICKS_PER_SECOND as u64 => [
                at.x,
                at.z,
                ((world.tick - t) as i32) * ONE / TICKS_PER_SECOND as i32,
                ONE,
            ],
            _ => [0, 0, 0, 0],
        };
        u.extend_from_slice(&ring);
        for wv in &world.sea.waves {
            let turns = (wv.phase >> 16) as i32;
            u.extend_from_slice(&[wv.dx, wv.dz, fx::div(ONE, wv.length), turns]);
        }
        for wv in &world.sea.waves {
            u.extend_from_slice(&[wv.amp, wv.sway, wv.slope, 0]);
        }
        self.uniforms = u;
    }

    pub fn commands(&self) -> &[u32] {
        &self.out.commands
    }

    pub fn values(&self) -> &[i32] {
        &self.out.values
    }

    pub fn frame_uniforms(&self) -> &[i32] {
        &self.uniforms
    }

    /// Every program's constant uniforms, in `numbers()`'s order.
    pub fn constants(&self) -> Vec<(Program, Uniform)> {
        let l = &self.look;
        let mut out = Vec::new();
        for (p, table) in [
            (Program::Sky, &l.sky),
            (Program::Shapes, &l.shapes),
            (Program::Water, &l.water),
        ] {
            for u in l.constants(table) {
                out.push((p, u));
            }
        }
        out
    }

    /// What the page needs to know once: layouts, sizes and names.
    pub fn numbers(&self) -> Value {
        let waves = self.world.sea.waves.len();
        json!({
            "meshes": self.meshes.iter().map(|m| m.len() / VERTEX_VALUES).collect::<Vec<_>>(),
            "vertex": {"stride": VERTEX_VALUES * 4, "attributes": [["aPos", 3, 0], ["aNormal", 3, 12], ["aColor", 3, 24]]},
            "plain": {"stride": 12, "attributes": [["aPos", 3, 0]]},
            "instance": {"stride": INSTANCE_VALUES * 4, "attributes": [["iA", 4, 0], ["iB", 4, 16], ["iC", 4, 32], ["iD", 4, 48]]},
            "command_values": COMMAND_VALUES,
            "programs": {"sky": Program::Sky as u32, "shapes": Program::Shapes as u32, "water": Program::Water as u32},
            "passes": {"solid": Pass::Solid as u32, "glow": Pass::Glow as u32},
            "waves": waves,
            // the wave arrays are four-vectors whatever their length (four waves would make sixteen)
            "frame": frame_layout(waves).iter().map(|(p, n, s)| json!([*p as u32, n, s, if n.starts_with("uWave") { "uniform4fv" } else { setter(*s) }])).collect::<Vec<_>>(),
            "constants": self.constants().iter().map(|(p, (n, v))| json!([*p as u32, n, v.len(), setter(v.len())])).collect::<Vec<_>>(),
            "keys": bits::NAMES.iter().map(|(n, b)| json!([n, b])).collect::<Vec<_>>(),
            "water_vertices": self.look.grid_cells * self.look.grid_cells * 6,
            "sky_vertices": 3,
            "matrix": "uniformMatrix4fv",
        })
    }

    pub fn water_mesh(&self) -> Vec<i32> {
        mesh::water_grid(self.look.grid_cells, self.look.grid_radius_m * ONE)
    }

    /// A number the browser check reads.
    pub fn checksum(&self) -> u64 {
        self.world.checksum()
    }

    // ---------------------------------------------------------------- the HUD

    fn hints(&self) -> Vec<Value> {
        let w = &self.world;
        let fill = |text: &str| -> String {
            text.replace(
                "{line.break_tension}",
                &format!("{} N", decimal(w.line.break_tension(), 1)),
            )
            .replace(
                "{reel.wind_per_turn}",
                &format!("{} m", decimal(w.gear.line.wind_per_turn_cm * ONE / 100, 2)),
            )
            .replace("{scope.max_zoom}", &self.look.camera.scope_max_zoom.to_string())
            .replace(
                "{tide.period_minutes}",
                &w.place.sea.tide_period_minutes.to_string(),
            )
        };
        glossary::parse(&self.glossary_md)
            .map(|es| {
                es.into_iter()
                    .map(|e| json!({"term": e.term, "definition": fill(&e.definition)}))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn place_rows(&self) -> Vec<Value> {
        let w = &self.world;
        let s = &self.strings;
        let knots = |v: Fx| {
            s.fill(
                "value.knots",
                &[("knots", decimal(fx::mul(v, ONE * 1944 / 1000), 1))],
            )
        };
        let tide_key = if w.sea.tide_rising() {
            "tide.rising"
        } else {
            "tide.falling"
        };
        let swell = s.fill(
            "value.swell",
            &[
                ("height", decimal(w.sea.significant_height(), 1)),
                ("period", decimal(w.sea.peak_period(), 0)),
            ],
        );
        vec![
            json!([
                s.get("row.tide"),
                s.fill(tide_key, &[("height", decimal(w.sea.level(), 2))])
            ]),
            json!([s.get("row.current"), knots(w.sea.water_velocity().flat_len())]),
            json!([s.get("row.wind"), knots(w.sea.wind.flat_len())]),
            json!([s.get("row.swell"), swell]),
            json!([
                s.get("row.line_out"),
                s.fill("value.metres", &[("metres", decimal(w.line.out, 1))])
            ]),
            json!([
                s.get("row.drag"),
                s.fill("value.newtons", &[("newtons", decimal(w.line.drag(), 1))])
            ]),
        ]
    }

    /// The coaching line when nothing has happened lately.
    fn hint(&self) -> String {
        let w = &self.world;
        let s = &self.strings;
        if let Some(i) = w.holder {
            if w.fish[i].mood == Mood::Hooked {
                return s.get("hint.fight");
            }
        }
        if let Some(f) = w.fish.iter().position(|f| f.mood == Mood::Follow) {
            return s.fill("hint.following", &[("fish", self.fish_name(f))]);
        }
        if w.line.bail_open {
            return s.get("hint.running");
        }
        let e = w.line.end();
        if e.y < w.sea.height(e.x, e.z) {
            return s.get("hint.work");
        }
        s.get("hint.cast")
    }

    fn message_now(&self) -> String {
        match &self.message {
            Some((m, t)) if self.world.tick - t < 4 * TICKS_PER_SECOND as u64 => m.clone(),
            _ => self.hint(),
        }
    }

    fn keys_bar(&self) -> Vec<Value> {
        let s = &self.strings;
        vec![
            json!(["↑ ↓", s.get("key.pedal")]),
            json!(["← →", s.get("key.drift")]),
            json!(["Space", s.get("key.wind")]),
            json!(["RMB", s.get("key.scope")]),
            json!(["Esc", s.get("key.menu")]),
        ]
    }

    fn arm_panel(&self) -> Value {
        let s = &self.strings;
        let held = |b: u32| self.keys & b != 0;
        let k = |label: &str, b: u32| json!({"key": label, "on": held(b)});
        json!({
            "heading": s.get("arms.heading"),
            "left": s.get("arms.left"),
            "right": s.get("arms.right"),
            "rows": [
                [s.get("arms.shoulder"), [k("Q", bits::LEFT_SHOULDER_UP), k("A", bits::LEFT_SHOULDER_DOWN)], [k("I", bits::RIGHT_SHOULDER_UP), k("K", bits::RIGHT_SHOULDER_DOWN)]],
                [s.get("arms.elbow"), [k("W", bits::LEFT_ELBOW_BEND), k("S", bits::LEFT_ELBOW_STRAIGHTEN)], [k("O", bits::RIGHT_ELBOW_BEND), k("L", bits::RIGHT_ELBOW_STRAIGHTEN)]],
                [s.get("arms.line"), [k("F", bits::FINGER), {"note": s.get("arms.finger")}], [k("Space", bits::WIND), {"note": s.get("arms.wind")}]],
            ],
        })
    }

    fn tension(&self) -> Value {
        let w = &self.world;
        let frac = fx::div(w.line.tension, w.line.break_tension().max(1)).clamp(0, ONE);
        let state = if frac > ONE * 3 / 4 {
            "danger"
        } else if frac > ONE / 2 {
            "strained"
        } else {
            "calm"
        };
        json!({
            "fraction": decimal(frac, 3),
            "percent": decimal(frac * 100, 0),
            "break": decimal(ONE, 3),
            "state": state,
            "label": self.strings.get("ring.label"),
            "of_break": self.strings.get("ring.of_break"),
        })
    }

    fn scope(&self) -> Value {
        let s = &self.strings;
        let on = self.scoping();
        let w = &self.world;
        let (w_px, h_px) = self.pixels;
        let aspect = (w_px as i64 * ONE as i64 / h_px.max(1) as i64) as Fx;
        let lure = w.line.end();
        // the nearest fish to the lure within ten metres, and its mouth zone on screen
        let near = w
            .fish
            .iter()
            .enumerate()
            .filter(|(_, f)| f.mouth_at().sub(lure).len() < 10 * ONE)
            .min_by_key(|(_, f)| f.mouth_at().sub(lure).len());
        let mut out =
            json!({"on": on, "zoom": s.fill("scope.zoom", &[("zoom", self.scope_zoom.to_string())])});
        match near {
            Some((i, f)) => {
                let spec = &w.place.fish[f.species];
                let inside = f.in_zone(spec, lure);
                let m = self.camera.matrix(aspect);
                if let Some((x, y)) = crate::geom::project(&m, f.mouth_at()) {
                    let edge = f.mouth_at().add(v3(f.zone(spec), 0, 0));
                    let r = crate::geom::project(&m, edge)
                        .map(|(ex, ey)| (ex - x).abs().max((ey - y).abs()))
                        .unwrap_or(0);
                    out["ring"] = json!({"x": decimal(x * ONE / 10, 1), "y": decimal(y * ONE / 10, 1), "r": decimal(r * ONE / 10, 1)});
                }
                out["fish"] = json!(self.fish_name(i));
                out["status"] = json!(s.get(if inside { "scope.in_zone" } else { "scope.outside" }));
                out["inside"] = json!(inside);
            }
            None => out["status"] = json!(s.get("scope.nothing")),
        }
        out
    }

    /// Everything the page draws as HTML.
    pub fn ui(&self) -> Value {
        let s = &self.strings;
        let w = &self.world;
        let mut panels: Vec<Value> = Vec::new();
        match self.screen {
            Screen::Title => {
                panels.push(json!({
                    "id": "title-card",
                    "heading": s.get("title"),
                    "text": s.get("intro"),
                    "actions": [{"name": "screen.show", "value": "fishing", "label": s.get("start"), "primary": true}],
                }));
                let how: Vec<String> = [
                    "how.cast",
                    "how.sink",
                    "how.work",
                    "how.set",
                    "how.fight",
                    "how.scope",
                ]
                .iter()
                .map(|k| s.get(k))
                .collect();
                panels.push(json!({
                    "id": "how",
                    "heading": s.get("heading.controls"),
                    "list": how,
                }));
            }
            Screen::Fishing => {
                panels.push(json!({
                    "id": "place",
                    "eyebrow": format!("{} · {}", s.get(&w.place.name), s.get("place.when")),
                    "rows": self.place_rows(),
                }));
            }
            Screen::Landed => {
                if let Some(c) = &w.landed {
                    let spec = &w.place.fish[c.species];
                    panels.push(json!({
                        "id": "landed",
                        "eyebrow": s.get("landed.heading"),
                        "heading": s.get(&spec.name),
                        "rows": [
                            [s.get("landed.length"), s.fill("value.cm", &[("cm", decimal(c.length * 100, 0))])],
                            [s.get("landed.weight"), s.fill("value.kg", &[("kg", decimal(c.mass, 1))])],
                            [s.get("landed.fight"), s.fill("value.seconds", &[("seconds", (c.fight_ticks / TICKS_PER_SECOND as u32).to_string())])],
                            [s.get("landed.caught"), s.fill("value.count", &[("count", (w.catches.len() + 1).to_string())])],
                        ],
                        "actions": [
                            {"name": "catch.release", "value": "", "label": s.get("release"), "primary": true},
                            {"name": "catch.keep", "value": "", "label": s.get("keep"), "primary": false},
                        ],
                    }));
                }
            }
            Screen::Settings => {
                let choice = |name: &str, opts: &[(&str, &str)], at: &str| -> Value {
                    json!({"name": name, "options": opts.iter().map(|(v, l)| json!({"value": v, "label": s.get(l), "on": *v == at})).collect::<Vec<_>>()})
                };
                let lv = |n: i32| ["off", "some", "more"][n.clamp(0, 2) as usize];
                panels.push(json!({
                    "id": "settings",
                    "heading": s.get("settings.heading"),
                    "text": s.get("settings.note"),
                    "choices": [
                        [s.get("settings.arm_speed"), choice("settings.arm_speed", &[("normal", "settings.normal"), ("slow", "settings.slow")], if w.arms.slow { "slow" } else { "normal" })],
                        [s.get("settings.steady_hands"), choice("settings.steady_hands", &[("off", "settings.off"), ("some", "settings.some"), ("more", "settings.more")], lv(w.arms.steady))],
                        [s.get("settings.steady_hull"), choice("settings.steady_hull", &[("off", "settings.off"), ("some", "settings.some"), ("more", "settings.more")], lv(w.hull.steady))],
                    ],
                    "actions": [{"name": "menu.toggle", "value": "", "label": s.get("resume"), "primary": true}],
                }));
            }
        }
        let fishing = matches!(self.screen, Screen::Fishing | Screen::Settings);
        json!({
            "screen": self.screen.name(),
            "title": s.get("title"),
            "panels": panels,
            "hud": fishing,
            "message": if fishing { self.message_now() } else { String::new() },
            "tension": self.tension(),
            "keys": self.keys_bar(),
            "arms": self.arm_panel(),
            "arms_open": w.catches.is_empty(),
            "scope": self.scope(),
            "hints": self.hints(),
            "data": {
                "out": decimal(w.line.out, 2),
                "bail": if w.line.bail_open { "open" } else { "closed" },
                "hooked": w.holder.map(|i| w.fish[i].mood == Mood::Hooked).unwrap_or(false),
                "catches": w.catches.len(),
                "tick": w.tick,
            },
        })
    }
}

/// The WebGL call that sets a uniform of this many floats: a 4×4 matrix at
/// sixteen, a vector up to four, and an array of four-vectors beyond.
fn setter(size: usize) -> &'static str {
    match size {
        1 => "uniform1fv",
        2 => "uniform2fv",
        3 => "uniform3fv",
        16 => "uniformMatrix4fv",
        _ => "uniform4fv",
    }
}

fn level(v: &str) -> Result<i32, String> {
    match v {
        "off" => Ok(0),
        "some" => Ok(1),
        "more" => Ok(2),
        other => Err(format!("There is no setting \"{other}\".")),
    }
}

fn shape_of(i: usize) -> Shape {
    match i {
        0 => Shape::Cube,
        1 => Shape::Cyl,
        2 => Shape::Cone,
        3 => Shape::Rock,
        4 => Shape::Ball,
        5 => Shape::Hull,
        _ => Shape::Fin,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn session(seed: u64) -> Session {
        Session::new(
            include_str!("../../../data/strings.json"),
            include_str!("../../../data/gear.json"),
            include_str!("../../../data/place.json"),
            include_str!("../../../data/look.json"),
            include_str!("../../../GLOSSARY.md"),
            seed,
        )
        .unwrap()
    }

    fn play(s: &mut Session, keys: impl Fn(u32) -> u32) {
        s.act("screen.show", "fishing").unwrap();
        for f in 0..3600u32 {
            s.frame(f as i64 * 16_667, keys(f));
        }
    }

    #[test]
    fn the_scope_changes_no_rule() {
        let script = |f: u32| match f % 600 {
            0..=59 => bits::PEDAL,
            60..=89 => bits::RIGHT_SHOULDER_UP | bits::FINGER,
            90..=105 => bits::RIGHT_SHOULDER_DOWN | bits::FINGER,
            300..=599 => bits::WIND,
            _ => 0,
        };
        let mut plain = session(5);
        play(&mut plain, script);
        let mut scoped = session(5);
        play(&mut scoped, |f| script(f) | bits::SCOPE);
        assert_eq!(plain.checksum(), scoped.checksum());
        assert_ne!(plain.camera, scoped.camera, "the scope did move the camera");
    }

    #[test]
    fn the_title_starts_the_open_swell_and_the_hud_names_it() {
        let mut s = session(1);
        assert_eq!(s.ui()["screen"], "title");
        assert_eq!(s.ui()["panels"][0]["heading"], "Extremely Lifelike Fishing");
        s.act("screen.show", "fishing").unwrap();
        let ui = s.ui();
        assert_eq!(ui["screen"], "fishing");
        assert_eq!(ui["panels"][0]["eyebrow"], "Open Swell · afternoon");
        // the swell row is computed: Hs 2.07 m rounds to 2.1; T 8.98 s to 9
        assert_eq!(ui["panels"][0]["rows"][3][1], "2.1 m, 9 s");
        assert_eq!(
            s.act("screen.jump", "x"),
            Err("Unknown action \"screen.jump\".".into())
        );
    }

    #[test]
    fn numbers_name_a_mesh_for_every_shape_and_species() {
        let s = session(1);
        let n = s.numbers();
        assert_eq!(n["meshes"].as_array().unwrap().len(), SHAPES + 2);
        let frame: usize = n["frame"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e[2].as_u64().unwrap() as usize)
            .sum();
        assert_eq!(
            frame,
            s.frame_uniforms().len(),
            "the frame's uniforms match their layout"
        );
    }

    #[test]
    fn decimals_round_half_away_from_zero() {
        assert_eq!(decimal(ONE * 207 / 100, 1), "2.1");
        assert_eq!(decimal(ONE / 2, 0), "1");
        assert_eq!(decimal(-ONE / 4, 2), "-0.25");
        assert_eq!(decimal(ONE * 3, 0), "3");
    }
}
