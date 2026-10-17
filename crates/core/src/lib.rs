//! tool_core: every rule of Extremely Lifelike Fishing.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.

pub mod api;
pub mod arms;
pub mod fx;
pub mod geom;
pub mod glossary;
pub mod hull;
pub mod line;
pub mod page;
pub mod rng;
pub mod sea;
pub mod ticks;
