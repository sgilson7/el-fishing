//! tool_core: every rule of Extremely Lifelike Fishing.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! The rules: `sea` (level, waves, current), `hull` (the kayak), `arms`
//! (the arms and the rod), `line`, `fish`, and `world`, which steps them
//! together. What is only seen: `session` (screens, HUD, camera, scope),
//! `scene`, `mesh`, `render` and `look`.

pub mod arms;
pub mod fish;
pub mod flowmap;
pub mod fx;
pub mod geom;
pub mod glossary;
pub mod hull;
pub mod line;
pub mod look;
pub mod mesh;
pub mod render;
pub mod river;
pub mod rng;
pub mod scene;
pub mod sea;
pub mod session;
pub mod strings;
pub mod ticks;
pub mod world;
