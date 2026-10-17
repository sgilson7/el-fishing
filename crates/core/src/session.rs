//! A visit: the screens, the HUD, the camera and the scope around one
//! world, and the numbers the page draws each frame. The world holds every
//! rule of the water and the fish; the session holds what is only seen, so
//! the scope, the camera and the HUD cannot change a rule (plan decision 17).

use serde_json::{json, Value};

use crate::fish::Mood;
use crate::flowmap::{self, FlowMap};
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
    Log,
}

impl Screen {
    pub fn name(self) -> &'static str {
        match self {
            Screen::Title => "title",
            Screen::Fishing => "fishing",
            Screen::Landed => "landed",
            Screen::Settings => "settings",
            Screen::Log => "log",
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

/// A landed fish, kept in the visit's fish log (T11). The log lasts the
/// visit: Builder keeps nothing in the browser's storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logged {
    pub place: usize,
    pub species: usize,
    pub length: Fx,
    pub mass: Fx,
}

pub struct Session {
    pub world: World,
    /// The look at this place from this boat.
    pub look: Look,
    /// The look as `data/look.json` gives it, before a place changes it.
    base_look: Look,
    places: Vec<PlaceSpec>,
    gear: GearSpec,
    seed: u64,
    /// Which place and which boat the world is.
    pub place: usize,
    pub boat: usize,
    /// Which of the place's weathers: 0 its own (T18).
    pub weather: usize,
    /// Counts up each time the world is rebuilt for another place or boat,
    /// so the page knows to send the GPU the new constants and flow map.
    pub generation: u32,
    /// The mesh number of each place's first species.
    fish_base: Vec<usize>,
    pub log: Vec<Logged>,
    /// Display frames since a fish was landed, to turn it in the angler's hands.
    shown: i32,
    flow: FlowMap,
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

/// `n` hundredths or thousandths (`per`) with one decimal place, or none
/// when the value is whole: 8 m, 0.6 m, 1 m/s.
fn plain(n: i32, per: i32) -> String {
    if n % per == 0 {
        (n / per).to_string()
    } else {
        decimal(n * (ONE / per) + n * (ONE % per) / per, 1)
    }
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
        places: &str,
        look: &str,
        glossary_md: &str,
        seed: u64,
    ) -> Result<Session, String> {
        let strings = Strings::parse(strings)?;
        let gear: GearSpec = serde_json::from_str(gear).map_err(|e| format!("data/gear.json: {e}"))?;
        let places: Vec<PlaceSpec> =
            serde_json::from_str(places).map_err(|e| format!("data/places.json: {e}"))?;
        let base_look: Look = serde_json::from_str(look).map_err(|e| format!("data/look.json: {e}"))?;
        glossary::parse(glossary_md).map_err(|e| e.message())?;
        if places.is_empty() || gear.boats.is_empty() {
            return Err("There must be at least one place and one boat.".into());
        }
        // the shaders are compiled once for one wave count
        let waves = places[0].sea.waves.len();
        let mut names: Vec<String> = Vec::new();
        for p in &places {
            for w in 0..p.weathers() {
                let pw = p.in_weather(w);
                if pw.sea.waves.len() != waves {
                    return Err(format!(
                        "data/places.json: {} in {} has {} waves; every place needs {waves}.",
                        p.name,
                        pw.weather,
                        pw.sea.waves.len()
                    ));
                }
                names.push(pw.weather.clone());
                for b in &gear.boats {
                    base_look.at(&p.name, &pw.weather, &b.name)?;
                }
            }
            names.push(p.name.clone());
            names.extend(kinds(p).iter().map(|f| f.name.clone()));
        }
        names.extend(gear.boats.iter().map(|b| b.name.clone()));
        for n in names {
            if !strings.has(&n) {
                return Err(format!("The strings file has no \"{n}\"."));
            }
        }
        let mut meshes: Vec<Vec<i32>> = (0..SHAPES).map(|i| mesh::shape(shape_of(i))).collect();
        let mut fish_base = Vec::new();
        for p in &places {
            fish_base.push(meshes.len());
            for f in kinds(p) {
                meshes.push(mesh::fish_body(f));
            }
        }
        let world = World::new(gear.clone(), places[0].clone(), 0, seed);
        let look = base_look.at(&places[0].name, &places[0].weather, &gear.boats[0].name)?;
        let camera = Camera {
            eye: v3(0, 3 * ONE, 6 * ONE),
            at: V3::ZERO,
            focal: look.camera.focal_x1000 * ONE / 1000,
        };
        let mut s = Session {
            world,
            scope_zoom: look.camera.scope_start_zoom,
            look,
            base_look,
            places,
            gear,
            seed,
            place: 0,
            boat: 0,
            weather: 0,
            generation: 0,
            fish_base,
            log: Vec::new(),
            shown: 0,
            flow: FlowMap {
                width: 1,
                height: 1,
                bytes: vec![128, 128, 0, 255],
                corner: (0, 0),
                size: (ONE, ONE),
            },
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
        s.flow = s.build_flow();
        s.place_camera(true);
        s.build_frame();
        Ok(s)
    }

    /// A species of the world's fish as one of the place's kinds: the
    /// trolling fish follow the casting fish in that list.
    fn kind(&self, species: usize) -> usize {
        if self.world.troll().is_some() && !self.places[self.place].troll_fish.is_empty() {
            self.places[self.place].fish.len() + species
        } else {
            species
        }
    }

    /// Builds the world afresh at the chosen place in the chosen boat.
    fn enter(&mut self) {
        let place = self.places[self.place].in_weather(self.weather);
        let boat = self.gear.boats[self.boat].name.clone();
        // checked for every place, weather and boat when the session was made
        if let Ok(l) = self.base_look.at(&place.name, &place.weather, &boat) {
            self.look = l;
        }
        let seed = self.seed.wrapping_add(self.generation as u64 + 1);
        self.world = World::new(self.gear.clone(), place, self.boat, seed);
        self.flow = self.build_flow();
        self.generation += 1;
        self.message = None;
        self.scope_zoom = self.look.camera.scope_start_zoom;
        self.place_camera(true);
    }

    fn build_flow(&self) -> FlowMap {
        let range = Look::number(&self.look.water, "uFlowK", 0);
        flowmap::build(&self.world.sea, range)
    }

    /// The flow map's size in texels, and its bytes, for the water shader.
    pub fn flow_size(&self) -> Vec<i32> {
        vec![self.flow.width as i32, self.flow.height as i32]
    }

    pub fn flow_map(&self) -> &[u8] {
        &self.flow.bytes
    }

    /// A choice from a list by its number.
    fn pick(v: &str, of: usize) -> Result<usize, String> {
        match v.parse::<usize>() {
            Ok(i) if i < of => Ok(i),
            _ => Err(format!("There is no choice \"{v}\".")),
        }
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
        self.screen == Screen::Fishing && self.keys & bits::SCOPE != 0 && self.world.fishing()
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
                    self.shown = 0;
                    break;
                }
            }
        }
        if self.screen == Screen::Landed {
            self.shown += 1;
        }
        let want = if self.scoping() { ONE } else { 0 };
        self.clarity += (want - self.clarity) / 6;
        self.place_camera(false);
        self.build_frame();
    }

    /// The fish whose mouth is nearest the lure, within ten metres of it.
    fn nearest_fish(&self) -> Option<usize> {
        let lure = self.world.line.end();
        self.world
            .fish
            .iter()
            .enumerate()
            .filter(|(_, f)| f.mouth_at().sub(lure).len() < 10 * ONE)
            .min_by_key(|(_, f)| f.mouth_at().sub(lure).len())
            .map(|(i, _)| i)
    }

    /// How deep the lure is under the surface, and how fast it moves through the water.
    fn lure_depth_and_speed(&self) -> (Fx, Fx) {
        let w = &self.world;
        let e = w.line.end();
        let depth = (w.sea.height(e.x, e.z) - e.y).max(0);
        (depth, w.lure_vel.sub(w.sea.current_at(e.x, e.z)).len())
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
            // on a trolling boat the hull's own speed sets the hook
            Event::Bit(_) if self.world.troll().is_some() => say(self, "event.troll_bit", None),
            Event::Bit(_) => say(self, "event.bit", None),
            Event::Missed(i) => say(self, "event.missed", Some(i)),
            Event::Spat(i) => say(self, "event.spat", Some(i)),
            Event::HookSet(i) => say(self, "event.hook_set", Some(i)),
            Event::Threw(_) => say(self, "event.threw", None),
            Event::Jumped(_) => say(self, "event.jumped", None),
            Event::Snapped => say(self, "event.snapped", None),
            Event::Landed => return,
            Event::Strike(r) => {
                let rod = self
                    .world
                    .troll()
                    .map(|t| self.strings.get(&t.rods[r].name))
                    .unwrap_or_default();
                self.strings.fill("event.strike", &[("rod", rod)])
            }
        };
        self.message = Some((text, self.world.tick));
    }

    /// Settings, screens and the catch, through one door.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match (name, value) {
            ("screen.show", "fishing") => {
                self.finish(false);
                self.screen = Screen::Fishing;
            }
            ("screen.show", "settings") => self.screen = Screen::Settings,
            ("screen.show", "title") => self.screen = Screen::Title,
            ("screen.show", "log") => self.screen = Screen::Log,
            ("screen.show", other) => return Err(format!("There is no screen \"{other}\".")),
            ("place.choose", v) => {
                self.place = Session::pick(v, self.places.len())?;
                self.weather = 0;
                self.enter();
            }
            ("weather.choose", v) => {
                self.weather = Session::pick(v, self.places[self.place].weathers())?;
                self.enter();
            }
            ("boat.choose", v) => {
                let b = Session::pick(v, self.gear.boats.len())?;
                if self.gear.boats[b].racing {
                    return Err(format!(
                        "The {} is kept for the racing maps.",
                        self.strings.get(&self.gear.boats[b].name)
                    ));
                }
                self.boat = b;
                self.enter();
            }
            ("catch.release", _) | ("catch.keep", _) => {
                if self.screen != Screen::Landed {
                    return Err("There is no landed fish.".into());
                }
                self.finish(name == "catch.keep");
                self.screen = Screen::Fishing;
                self.message = None;
            }
            ("settings.arm_speed", v) => self.world.arms.slow = v == "slow",
            ("settings.steady_hull", v) => self.world.hull.steady = level(v)?,
            ("menu.toggle", _) => {
                self.screen = match self.screen {
                    Screen::Fishing => Screen::Settings,
                    Screen::Settings | Screen::Log => Screen::Fishing,
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

    /// Lets the landed fish go or keeps it, and writes it in the fish log.
    fn finish(&mut self, keep: bool) {
        if let Some(c) = &self.world.landed {
            self.log.push(Logged {
                place: self.place,
                species: self.kind(c.species),
                length: c.length,
                mass: c.mass,
            });
        }
        self.world.finish_catch(keep);
    }

    /// Where a landed fish is held up to be looked at, in hull space, and
    /// the way it faces as it turns (T11).
    fn shown_fish(&self) -> Option<(usize, V3, i32)> {
        if self.screen != Screen::Landed {
            return None;
        }
        let i = self.world.holder?;
        let at = self.world.hull.to_world(v3(0, ONE * 105 / 100, ONE * 60 / 100));
        let turn = self.shown * fx::TURN / (6 * TICKS_PER_SECOND as i32);
        Some((i, at, self.world.hull.heading + turn))
    }

    fn place_camera(&mut self, snap: bool) {
        let c = &self.look.camera;
        let h = &self.world.hull;
        let cm = |v: i32| v * ONE / 100;
        if let Some((_, at, _)) = self.shown_fish() {
            // a close look at the fish held up in the angler's hands
            let eye = h.to_world(v3(ONE * 70 / 100, ONE * 135 / 100, ONE * 185 / 100));
            self.camera = Camera {
                eye,
                at,
                focal: c.focal_x1000 * ONE / 1000,
            };
            return;
        }
        if self.scoping() {
            // the angler's right eye, looking at the mouth of the fish nearest
            // the lure, or at the lure when no fish is near (T6)
            let eye = h.to_world(v3(
                cm(c.scope_eye_cm[0]),
                cm(c.scope_eye_cm[1]),
                cm(c.scope_eye_cm[2]),
            ));
            let at = self
                .nearest_fish()
                .map(|i| self.world.fish[i].mouth_at())
                .unwrap_or(self.world.line.end());
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
            .add(v3(0, h.pos.y.max(-ONE * 8), 0))
            .sub(fwd.scale(cm(c.behind_cm)))
            .add(right.scale(cm(c.aside_cm)))
            .add(v3(0, cm(c.above_cm), 0));
        let target_at = h
            .pos
            .flat()
            .add(fwd.scale(cm(c.look_ahead_cm)))
            .add(v3(0, ONE / 2, 0));
        // in a storm's troughs the hull drops metres below the level: the eye
        // follows it down but stays a metre over the water where it is
        let target_eye = v3(
            target_eye.x,
            target_eye
                .y
                .max(self.world.sea.height(target_eye.x, target_eye.z) + ONE),
            target_eye.z,
        );
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
        let view = scene::View {
            meshes: self.meshes.len(),
            own_body: !self.scoping(),
            fish_base: self.fish_base[self.place] + self.kind(0),
            shown: self.shown_fish(),
            eye: self.camera.eye,
        };
        let batch = scene::build(world, &self.look, &view);
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
        // the grid slides with the eye. Snapped to whole metres it jumped back
        // and forth as the camera swayed on the swell (T2)
        u.extend_from_slice(&[cam.eye.x, cam.eye.z]);
        let tick = world.tick as i64;
        u.push(((tick % (600 * TICKS_PER_SECOND)) * ONE as i64 / TICKS_PER_SECOND) as i32);
        let hl = &world.hull;
        // the wake shows the hull's speed through the water, not over the
        // ground, so a kayak drifting with a river leaves none (T14)
        let through = hl.vel.sub(world.sea.current_at(hl.pos.x, hl.pos.z)).flat_len();
        let full = self.look.wake_full_cm_s.max(1) * ONE / 100;
        let wake = fx::div(through, full).min(ONE);
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
        // each phase drawn where it will be between this tick and the next, so
        // the water moves on every display frame, not only on ticks (T2)
        let between = self.clock.between() as u64;
        for wv in &world.sea.waves {
            let drawn = wv.phase.wrapping_add(((wv.step as u64 * between) >> 16) as u32);
            let turns = (drawn >> 16) as i32;
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
        // the grid's radius and its spacing factor: a vertex r from the middle
        // of a square-law grid of n cells across sits 4√(rR)/n from the next
        let cells = l.grid_cells.max(1);
        out.push((
            Program::Water,
            ("uGrid".into(), vec![l.grid_radius_m * ONE, 4 * ONE / cells]),
        ));
        // the box the flow map covers: its corner and its size, metres
        let f = &self.flow;
        out.push((
            Program::Water,
            (
                "uFlowBox".into(),
                vec![f.corner.0, f.corner.1, f.size.0, f.size.1],
            ),
        ));
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
                "{reel.power_wind}",
                &format!("{} m/s", decimal(w.gear.line.power_wind_cm_s * ONE / 100, 0)),
            )
            .replace(
                "{tide.period_minutes}",
                &w.place.sea.tide_period_minutes.to_string(),
            )
            .replace("{battery.seconds}", &w.gear.power.battery_s.to_string())
            .replace("{battery.recharge}", &w.gear.power.recharge_s.to_string())
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
        let tide = if w.sea.tide_amp == 0 {
            s.get("tide.none")
        } else {
            s.fill(tide_key, &[("height", decimal(w.sea.level(), 2))])
        };
        let mut rows = Vec::new();
        if w.hull.spec.outboard.is_some() {
            // km/h: m/s × 3.6
            rows.push(json!([
                s.get("row.speed"),
                s.fill(
                    "value.kmh",
                    &[("kmh", decimal(fx::mul(w.hull.speed(), ONE * 36 / 10), 0))]
                )
            ]));
            let t = w.hull.throttle;
            let lever = if t < 0 {
                s.fill("value.reverse", &[("percent", decimal(-t * 100, 0))])
            } else if t == 0 {
                s.get("value.neutral")
            } else {
                s.fill("value.ahead", &[("percent", decimal(t * 100, 0))])
            };
            rows.push(json!([s.get("row.throttle"), lever]));
            let deg = w.hull.steer * 360 / fx::TURN;
            let side = match deg.signum() {
                1 => s.fill("value.right", &[("deg", deg.to_string())]),
                -1 => s.fill("value.left", &[("deg", (-deg).to_string())]),
                _ => s.get("value.centred"),
            };
            rows.push(json!([s.get("row.outboard"), side]));
        }
        rows.push(json!([s.get("row.tide"), tide]));
        rows.push(json!([
            s.get("row.current"),
            knots(w.sea.current_at(w.hull.pos.x, w.hull.pos.z).flat_len())
        ]));
        rows.push(json!([s.get("row.wind"), knots(w.sea.wind.flat_len())]));
        if w.sea.river.is_none() {
            rows.push(json!([s.get("row.swell"), swell]));
        }
        if w.fishing() && w.troll().is_none() {
            rows.push(json!([
                s.get("row.line_out"),
                s.fill("value.metres", &[("metres", decimal(w.line.out, 1))])
            ]));
            rows.push(json!([
                s.get("row.lure_depth"),
                s.fill(
                    "value.metres",
                    &[("metres", decimal(self.lure_depth_and_speed().0, 1))]
                )
            ]));
            rows.push(json!([
                s.get("row.drag"),
                s.fill("value.newtons", &[("newtons", decimal(w.line.drag(), 1))])
            ]));
        }
        rows
    }

    /// The coaching line when nothing has happened lately.
    fn hint(&self) -> String {
        let w = &self.world;
        let s = &self.strings;
        if !w.fishing() {
            return s.get("hint.drive");
        }
        if w.troll().is_some() && w.holder.is_none() {
            // the speed the trolled fish here like, through the water
            let lo = w
                .place
                .fish
                .iter()
                .map(|f| f.lure_speed_mm_s[0])
                .min()
                .unwrap_or(0);
            let hi = w
                .place
                .fish
                .iter()
                .map(|f| f.lure_speed_mm_s[1])
                .max()
                .unwrap_or(0);
            return s.fill(
                "hint.troll",
                &[
                    ("speed", decimal(w.hull.speed(), 1)),
                    ("slow", plain(lo, 1000)),
                    ("fast", plain(hi, 1000)),
                ],
            );
        }
        if let Some(i) = w.holder {
            let f = &w.fish[i];
            return match f.mood {
                Mood::Hooked if f.stamina < ONE / 4 => s.fill("hint.land", &[("fish", self.fish_name(i))]),
                Mood::Hooked => s.get("hint.fight"),
                _ if w.troll().is_some() => s.get("hint.troll_set"),
                _ => s.get("hint.set"),
            };
        }
        if let Some(f) = w.fish.iter().position(|f| f.mood == Mood::Follow) {
            return s.fill("hint.following", &[("fish", self.fish_name(f))]);
        }
        let (depth, speed) = self.lure_depth_and_speed();
        if w.line.bail_open {
            return s.fill("hint.running", &[("depth", decimal(depth, 1))]);
        }
        if depth > 0 {
            return s.fill(
                "hint.work",
                &[("depth", decimal(depth, 1)), ("speed", decimal(speed, 1))],
            );
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
        if self.world.troll().is_some() {
            return vec![
                json!(["↑ ↓", s.get("key.throttle")]),
                json!(["← →", s.get("key.steer")]),
                json!(["1–4", s.get("key.rods")]),
                json!(["Space", s.get("key.wind")]),
                json!(["Q", s.get("key.let_out")]),
                json!(["RMB", s.get("key.scope")]),
                json!(["Esc", s.get("key.menu")]),
            ];
        }
        if !self.world.fishing() {
            return vec![
                json!(["↑ ↓", s.get("key.throttle")]),
                json!(["← →", s.get("key.steer")]),
                json!(["Space", s.get("key.neutral")]),
                json!(["Esc", s.get("key.menu")]),
            ];
        }
        vec![
            json!(["↑ ↓", s.get("key.pedal")]),
            json!(["← →", s.get("key.drift")]),
            json!(["Space", s.get("key.wind")]),
            json!(["E", s.get("key.power")]),
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
            "rows": [
                [s.get("arms.shoulder"), [k("W", bits::SHOULDER_UP), k("S", bits::SHOULDER_DOWN)]],
                [s.get("arms.elbow"), [k("A", bits::ELBOW_BEND), k("D", bits::ELBOW_STRAIGHTEN)]],
                [s.get("arms.line"), [k("F", bits::FINGER), {"note": s.get("arms.finger")}]],
                [s.get("arms.reel"), [k("Space", bits::WIND), {"note": s.get("arms.wind")}]],
                [s.get("arms.let_out"), [k("Q", bits::LET_OUT), {"note": s.get("arms.let_out_note")}]],
            ],
        })
    }

    fn tension(&self) -> Value {
        let w = &self.world;
        let frac = fx::div(w.line.tension, w.line.break_tension().max(1)).clamp(0, ONE);
        // the same split as the fight cue: calm well under the drag,
        // strained near it or slipping, danger near the break (T13)
        let state = if frac > ONE * 3 / 4 {
            "danger"
        } else if w.line.slipping || w.line.tension >= w.line.drag() * 3 / 4 {
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
        let near = self.nearest_fish().map(|i| (i, &w.fish[i]));
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

    /// The fight cue (T13): wind while the pull is well under the drag,
    /// hold while it is near or over it, and power-wind a tired fish that is
    /// close; `None` with no fish hooked.
    fn cue(&self) -> Option<(&'static str, String)> {
        let w = &self.world;
        let i = w.holder?;
        let f = &w.fish[i];
        if f.mood != Mood::Hooked {
            return None;
        }
        let s = &self.strings;
        let near = f.pos.sub(w.angler()).flat_len() < 10 * ONE;
        if f.stamina < ONE / 4 && near && w.battery >= ONE / 10 {
            return Some(("haul", s.get("cue.haul")));
        }
        if w.line.slipping || w.line.tension >= w.line.drag() * 3 / 4 {
            return Some(("hold", s.get("cue.hold")));
        }
        Some(("wind", s.get("cue.wind")))
    }

    fn battery(&self) -> Value {
        let w = &self.world;
        let state = if w.pulling {
            "running"
        } else if w.battery < ONE / 10 {
            "flat"
        } else if w.battery >= ONE {
            "full"
        } else {
            "charging"
        };
        json!({
            "fraction": decimal(w.battery, 3),
            "percent": decimal(w.battery * 100, 0),
            "state": state,
            "label": self.strings.fill("battery.label", &[("percent", decimal(w.battery * 100, 0))]),
            "status": self.strings.get(&format!("battery.{state}")),
        })
    }

    /// A size word for a fish of this length among its kind: the quarter of
    /// the species' length range it falls in.
    fn size_word(&self, place: usize, species: usize, length: Fx) -> String {
        let spec = kinds(&self.places[place])[species];
        let lo = spec.length_cm[0] * ONE / 100;
        let hi = (spec.length_cm[1] * ONE / 100).max(lo + 1);
        let q = (fx::div(length - lo, hi - lo) * 4 / ONE).clamp(0, 3);
        self.strings
            .get(["size.small", "size.average", "size.large", "size.trophy"][q as usize])
    }

    /// One row a species, for every place: caught, how many and the
    /// biggest; or not yet, with how big they grow (T11).
    fn log_rows(&self) -> Vec<Value> {
        let s = &self.strings;
        let mut rows = Vec::new();
        for (pi, p) in self.places.iter().enumerate() {
            for (si, f) in kinds(p).into_iter().enumerate() {
                let mine: Vec<&Logged> = self
                    .log
                    .iter()
                    .filter(|l| l.place == pi && l.species == si)
                    .collect();
                let name = s.fill("log.name", &[("fish", s.get(&f.name)), ("place", s.get(&p.name))]);
                let value = match mine.iter().max_by_key(|l| l.length) {
                    Some(best) => s.fill(
                        "log.caught",
                        &[
                            ("count", mine.len().to_string()),
                            ("cm", decimal(best.length * 100, 0)),
                            ("kg", decimal(best.mass, 1)),
                        ],
                    ),
                    None => s.fill(
                        "log.not_yet",
                        &[
                            ("from", f.length_cm[0].to_string()),
                            ("to", f.length_cm[1].to_string()),
                        ],
                    ),
                };
                rows.push(json!([name, value]));
            }
        }
        rows
    }

    /// The title card: the place and boat to choose, and the way in.
    fn title_panels(&self) -> Vec<Value> {
        let s = &self.strings;
        let w = &self.world;
        let chips = |name: &str, labels: Vec<String>, at: usize| -> Value {
            json!({"name": name, "options": labels.iter().enumerate().map(|(i, l)| json!({"value": i.to_string(), "label": l, "on": i == at})).collect::<Vec<_>>()})
        };
        let places = self.places.iter().map(|p| s.get(&p.name)).collect();
        // craft kept for the racing maps to come are not offered here (T25)
        let offered: Vec<usize> = (0..self.gear.boats.len())
            .filter(|&i| !self.gear.boats[i].racing)
            .collect();
        let boats = json!({"name": "boat.choose", "options": offered.iter().map(|&i| json!({"value": i.to_string(), "label": s.get(&self.gear.boats[i].name), "on": i == self.boat})).collect::<Vec<_>>()});
        let start = if w.troll().is_some() {
            "start.troll"
        } else if w.fishing() {
            "start.fish"
        } else {
            "start.drive"
        };
        // a place with more than one weather offers them in a dropdown (T18)
        let here = &self.places[self.place];
        let weathers: Vec<Value> = if here.weathers() > 1 {
            vec![json!([
                s.get("choose.weather"),
                {"name": "weather.choose", "options": (0..here.weathers()).map(|i| json!({"value": i.to_string(), "label": s.get(&here.in_weather(i).weather), "on": i == self.weather})).collect::<Vec<_>>()},
            ])]
        } else {
            Vec::new()
        };
        let mut out = vec![json!({
            "id": "title-card",
            "heading": s.get("title"),
            "text": s.get("intro"),
            "choices": [
                [s.get("choose.place"), chips("place.choose", places, self.place)],
                [s.get("choose.boat"), boats],
            ],
            "selects": weathers,
            "actions": [
                {"name": "screen.show", "value": "fishing", "label": s.fill(start, &[("place", s.get(&w.place.name))]), "primary": true},
                {"name": "screen.show", "value": "log", "label": s.get("log.open"), "primary": false},
            ],
        })];
        let how: Vec<String> = if w.troll().is_some() {
            let rods = w.troll().map(|t| t.rods.len()).unwrap_or(0);
            let mut how = vec![
                s.get("how.troll"),
                s.fill("how.troll_spread", &[("rods", rods.to_string())]),
            ];
            how.extend(w.place.fish.iter().map(|f| {
                s.fill(
                    "how.troll_species",
                    &[
                        ("fish", s.get(&f.name)),
                        ("slow", plain(f.lure_speed_mm_s[0], 1000)),
                        ("fast", plain(f.lure_speed_mm_s[1], 1000)),
                        ("to", plain(f.depth_cm[1], 100)),
                    ],
                )
            }));
            how.extend(["how.troll_strike", "how.troll_fight"].iter().map(|k| s.get(k)));
            how
        } else if w.fishing() {
            // one line a species, from its data: where it swims and the lure speed it likes
            let species = w.place.fish.iter().map(|f| {
                s.fill(
                    "how.species",
                    &[
                        ("fish", s.get(&f.name)),
                        ("from", plain(f.depth_cm[0], 100)),
                        ("to", plain(f.depth_cm[1], 100)),
                        ("slow", plain(f.lure_speed_mm_s[0], 1000)),
                        ("fast", plain(f.lure_speed_mm_s[1], 1000)),
                    ],
                )
            });
            let hold = w.place.fish.iter().map(|f| f.hold_ms).min().unwrap_or(0);
            let p = &w.gear.power;
            let fill = |k: &str| {
                s.fill(
                    k,
                    &[
                        ("power", decimal(w.gear.line.power_wind_cm_s * ONE / 100, 0)),
                        ("battery", p.battery_s.to_string()),
                        ("recharge", p.recharge_s.to_string()),
                        ("hold", decimal(hold * ONE / 1000, 0)),
                        ("zoom", self.look.camera.scope_max_zoom.to_string()),
                    ],
                )
            };
            let mut how: Vec<String> = ["how.cast", "how.sink"].iter().map(|k| fill(k)).collect();
            how.extend(species);
            how.extend(
                [
                    "how.work",
                    "how.follow",
                    "how.set",
                    "how.fight",
                    "how.haul",
                    "how.scope",
                ]
                .iter()
                .map(|k| fill(k)),
            );
            if w.sea.river.is_some() {
                how.push(s.get("how.river"));
            }
            how
        } else {
            ["how.throttle", "how.steer", "how.neutral", "how.jump"]
                .iter()
                .map(|k| s.get(k))
                .collect()
        };
        out.push(json!({"id": "how", "heading": s.get("heading.controls"), "list": how}));
        out
    }

    /// Everything the page draws as HTML.
    pub fn ui(&self) -> Value {
        let s = &self.strings;
        let w = &self.world;
        let mut panels: Vec<Value> = Vec::new();
        match self.screen {
            Screen::Title => panels.extend(self.title_panels()),
            Screen::Fishing => {
                panels.push(json!({
                    "id": "place",
                    "eyebrow": format!("{} · {}", s.get(&w.place.name), s.get(&w.place.weather)),
                    "rows": self.place_rows(),
                }));
                if let Some(t) = w.troll() {
                    // the spread: each rod, its line out, its lure's depth, and which is in play
                    let rows: Vec<Value> = (0..t.rods.len())
                        .map(|r| {
                            let l = w.rod_line(r);
                            let e = l.end();
                            let depth = (w.sea.height(e.x, e.z) - e.y).max(0);
                            let state = if r == w.in_play {
                                if w.holder.is_some() {
                                    "spread.fish_on"
                                } else {
                                    "spread.in_play"
                                }
                            } else {
                                "spread.holder"
                            };
                            json!([
                                s.fill(
                                    "spread.rod",
                                    &[("n", (r + 1).to_string()), ("rod", s.get(&t.rods[r].name))]
                                ),
                                s.fill(
                                    "spread.value",
                                    &[
                                        ("out", decimal(l.out, 0)),
                                        ("depth", decimal(depth, 1)),
                                        ("state", s.get(state))
                                    ]
                                ),
                            ])
                        })
                        .collect();
                    panels.push(json!({"id": "spread", "eyebrow": s.get("spread.heading"), "rows": rows}));
                }
            }
            Screen::Landed => {
                if let Some(c) = &w.landed {
                    let spec = &w.place.fish[c.species];
                    let kind = self.kind(c.species);
                    let before = self
                        .log
                        .iter()
                        .filter(|l| l.place == self.place && l.species == kind)
                        .map(|l| l.length)
                        .max();
                    let best = match before {
                        Some(b) if b >= c.length => s.fill("landed.best_was", &[("cm", decimal(b * 100, 0))]),
                        _ => s.get("landed.new_best"),
                    };
                    panels.push(json!({
                        "id": "landed",
                        "eyebrow": s.get("landed.heading"),
                        "heading": s.get(&spec.name),
                        "rows": [
                            [s.get("landed.length"), s.fill("value.cm", &[("cm", decimal(c.length * 100, 0))])],
                            [s.get("landed.weight"), s.fill("value.kg", &[("kg", decimal(c.mass, 1))])],
                            [s.get("landed.size"), self.size_word(self.place, kind, c.length)],
                            [s.get("landed.best"), best],
                            [s.get("landed.fight"), s.fill("value.seconds", &[("seconds", (c.fight_ticks / TICKS_PER_SECOND as u32).to_string())])],
                            [s.get("landed.caught"), s.fill("value.count", &[("count", (self.log.len() + 1).to_string())])],
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
                        [s.get("settings.steady_hull"), choice("settings.steady_hull", &[("off", "settings.off"), ("some", "settings.some"), ("more", "settings.more")], lv(w.hull.steady))],
                    ],
                    "actions": [
                        {"name": "menu.toggle", "value": "", "label": s.get("resume"), "primary": true},
                        {"name": "screen.show", "value": "log", "label": s.get("log.open"), "primary": false},
                        {"name": "screen.show", "value": "title", "label": s.get("change.place"), "primary": false},
                    ],
                }));
            }
            Screen::Log => {
                let total = self.places.iter().map(|p| kinds(p).len()).sum::<usize>();
                let caught = self
                    .places
                    .iter()
                    .enumerate()
                    .flat_map(|(pi, p)| (0..kinds(p).len()).map(move |si| (pi, si)))
                    .filter(|(pi, si)| self.log.iter().any(|l| l.place == *pi && l.species == *si))
                    .count();
                panels.push(json!({
                    "id": "log",
                    "heading": s.get("log.heading"),
                    "text": s.fill("log.note", &[("caught", caught.to_string()), ("total", total.to_string())]),
                    "rows": self.log_rows(),
                    "actions": [
                        {"name": "screen.show", "value": "fishing", "label": s.get("resume"), "primary": true},
                        {"name": "screen.show", "value": "title", "label": s.get("change.place"), "primary": false},
                    ],
                }));
            }
        }
        let on_water = matches!(self.screen, Screen::Fishing | Screen::Settings);
        let fishing = on_water && w.fishing();
        let cue = self.cue();
        json!({
            "screen": self.screen.name(),
            "title": s.get("title"),
            "panels": panels,
            "hud": on_water,
            "fishing": fishing,
            "message": if on_water { self.message_now() } else { String::new() },
            "tension": self.tension(),
            "cue": match &cue {
                Some((state, text)) if fishing => json!({"state": state, "text": text}),
                _ => json!(null),
            },
            "battery": self.battery(),
            "keys": self.keys_bar(),
            "arms": self.arm_panel(),
            "arms_open": self.log.is_empty() && w.catches.is_empty() && w.troll().is_none(),
            "scope": self.scope(),
            "hints": self.hints(),
            "data": {
                "out": decimal(w.line.out, 2),
                "bail": if w.line.bail_open { "open" } else { "closed" },
                "hooked": w.holder.map(|i| w.fish[i].mood == Mood::Hooked).unwrap_or(false),
                "catches": self.log.len(),
                "tick": w.tick,
                "place": self.place,
                "boat": self.boat,
                "weather": self.weather,
                "battery": decimal(w.battery * 100, 0),
                "speed": decimal(fx::mul(w.hull.speed(), ONE * 36 / 10), 0),
                "generation": self.generation,
            },
        })
    }
}

/// Every kind of fish at a place: the ones cast for, then the ones trolled for.
fn kinds(p: &PlaceSpec) -> Vec<&crate::fish::FishSpec> {
    p.fish.iter().chain(p.troll_fish.iter()).collect()
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
            include_str!("../../../data/places.json"),
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
            60..=89 => bits::SHOULDER_UP | bits::FINGER,
            90..=105 => bits::SHOULDER_DOWN | bits::FINGER,
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
        assert_eq!(ui["panels"][0]["eyebrow"], "Open Swell · Fair afternoon");
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
        // two cast and three trolled species at the Open Swell, two on the Ledge River
        assert_eq!(n["meshes"].as_array().unwrap().len(), SHAPES + 7);
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
    fn choosing_a_place_and_a_craft_rebuilds_the_world() {
        let mut s = session(1);
        assert_eq!((s.place, s.boat, s.generation), (0, 0, 0));
        s.act("place.choose", "1").unwrap();
        assert!(s.world.sea.river.is_some());
        assert_eq!(s.generation, 1);
        // the speedboat is kept for the racing maps: not offered, and refused (T25)
        let offered = s.ui()["panels"][0]["choices"][1][1]["options"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(offered, s.gear.boats.iter().filter(|b| !b.racing).count());
        assert_eq!(
            s.act("boat.choose", "1"),
            Err("The Speedboat is kept for the racing maps.".into())
        );
        assert_eq!(s.generation, 1);
        assert_eq!(
            s.act("place.choose", "2"),
            Err("There is no choice \"2\".".into())
        );
        s.act("screen.show", "fishing").unwrap();
        let ui = s.ui();
        assert_eq!(ui["panels"][0]["eyebrow"], "Ledge River · Calm midday");
        assert_eq!(ui["fishing"], true);
    }

    /// A session fishing, with fish 0 hooked on a short line beside the kayak.
    fn hooked(stamina: Fx) -> Session {
        let mut s = session(3);
        s.act("screen.show", "fishing").unwrap();
        for f in 0..60 {
            s.frame(f * 16_667, 0);
        }
        let w = &mut s.world;
        let e = w.line.end();
        let f = &mut w.fish[0];
        f.pos = e.sub(f.facing().scale(f.length / 2));
        f.vel = V3::ZERO;
        f.mood = Mood::Hooked;
        f.stamina = stamina;
        w.holder = Some(0);
        s
    }

    #[test]
    fn the_fight_cue_says_wind_hold_or_haul() {
        let mut s = hooked(ONE);
        s.world.line.tension = 0;
        assert_eq!(s.cue().map(|c| c.0), Some("wind"));
        // three quarters of the 10 N drag is 7.5 N
        s.world.line.tension = 8 * ONE;
        assert_eq!(s.cue().map(|c| c.0), Some("hold"));
        s.world.fish[0].stamina = ONE / 5;
        assert_eq!(s.cue().map(|c| c.0), Some("haul"));
        s.world.holder = None;
        assert_eq!(s.cue(), None);
    }

    #[test]
    fn a_landed_fish_is_shown_and_written_in_the_log() {
        let mut s = hooked(0);
        let mut f = 60;
        while s.screen != Screen::Landed && f < 600 {
            s.frame(f * 16_667, bits::POWER_WIND);
            f += 1;
        }
        assert_eq!(s.screen, Screen::Landed);
        assert!(s.shown_fish().is_some(), "the fish is held up to be looked at");
        let rows = &s.ui()["panels"][0]["rows"];
        assert_eq!(rows[3][1], "this one", "the first of its kind is the best");
        s.act("catch.release", "").unwrap();
        assert_eq!(s.log.len(), 1);
        s.act("screen.show", "log").unwrap();
        let ui = s.ui();
        // seven kinds in data/places.json, one caught
        assert_eq!(
            ui["panels"][0]["text"],
            "1 of 7 kinds caught this visit. The log is kept until you close the page."
        );
        let caught = ui["panels"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r[1].as_str().unwrap().starts_with("1 caught"))
            .count();
        assert_eq!(caught, 1);
    }

    #[test]
    fn the_open_swell_offers_a_storm_with_a_mid_ocean_sea() {
        let mut s = session(1);
        let ui = s.ui();
        assert_eq!(
            ui["panels"][0]["selects"][0][1]["options"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        s.act("weather.choose", "1").unwrap();
        assert_eq!(s.world.place.weather, "weather.storm");
        // amplitudes 1.9 1.1 0.75 0.5 0.3 0.18 0.09 0.04 m: Σa² = 5.7653, so
        // Hs = 4 √(5.7653 / 2) = 6.79 m; the tallest is 264 m long, so its period
        // is √(2π × 264 / 9.81) = 13.0 s
        let hs = s.world.sea.significant_height();
        assert!((hs - ONE * 679 / 100).abs() < ONE / 50, "Hs {hs}");
        let t = s.world.sea.peak_period();
        assert!((t - 13 * ONE).abs() < ONE / 20, "T {t}");
        s.act("screen.show", "fishing").unwrap();
        let ui = s.ui();
        assert_eq!(ui["panels"][0]["eyebrow"], "Open Swell · Storm");
        assert_eq!(ui["panels"][0]["rows"][3][1], "6.8 m, 13 s");
        // another place starts in its own weather; a weather it lacks is refused
        s.act("place.choose", "1").unwrap();
        assert_eq!(s.weather, 0);
        assert_eq!(
            s.act("weather.choose", "2"),
            Err("There is no choice \"2\".".into())
        );
    }

    #[test]
    fn rain_falls_only_in_the_storm() {
        use crate::render::Pass;
        let mut s = session(1);
        let count = |s: &Session| {
            let view = scene::View {
                meshes: s.meshes.len(),
                own_body: true,
                fish_base: SHAPES,
                shown: None,
                eye: V3::ZERO,
            };
            scene::build(&s.world, &s.look, &view).count_of(Shape::Cyl as usize, Pass::Glow)
        };
        let fair = count(&s);
        s.act("weather.choose", "1").unwrap();
        // 900 drops in data/places.json
        assert_eq!(count(&s) - fair, 900);
    }

    #[test]
    fn the_white_water_runs_at_two_point_six_metres_a_second() {
        let mut s = session(1);
        s.act("place.choose", "1").unwrap();
        s.act("weather.choose", "1").unwrap();
        // mid-channel at the start: the full 2.6 m/s, less about 1% for the
        // nearest rocks 15 to 25 m off, whose doublets fall away as R²/d²
        let v = s.world.sea.current_at(0, 0);
        assert!((v.flat_len() - ONE * 26 / 10).abs() < ONE * 26 / 1000, "{v:?}");
        assert_eq!(s.world.sea.river.as_ref().unwrap().rocks.len(), 54);
    }

    #[test]
    fn whole_values_show_no_decimal_place() {
        assert_eq!(plain(800, 100), "8");
        assert_eq!(plain(60, 100), "0.6");
        assert_eq!(plain(1000, 1000), "1");
        assert_eq!(plain(6500, 1000), "6.5");
    }

    #[test]
    fn decimals_round_half_away_from_zero() {
        assert_eq!(decimal(ONE * 207 / 100, 1), "2.1");
        assert_eq!(decimal(ONE / 2, 0), "1");
        assert_eq!(decimal(-ONE / 4, 2), "-0.25");
        assert_eq!(decimal(ONE * 3, 0), "3");
    }
}
