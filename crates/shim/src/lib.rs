//! tool_shim: forwards values between the page and `tool_core::session`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion: Q16 integers to floats for WebGL, the
//! page's milliseconds to microseconds. `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here.

use tool_core::session::Session;
use wasm_bindgen::prelude::*;

fn floats(v: &[i32]) -> Vec<f32> {
    v.iter().map(|&x| x as f32 / 65536.0).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: Session,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(
        strings: &str,
        gear: &str,
        places: &str,
        look: &str,
        glossary: &str,
        seed: u32,
    ) -> Result<Game, JsError> {
        Session::new(strings, gear, places, look, glossary, seed as u64)
            .map(|inner| Game { inner })
            .map_err(|e| JsError::new(&e))
    }
    pub fn frame(&mut self, now_ms: f64, keys: u32) {
        self.inner.frame((now_ms * 1000.0) as i64, keys)
    }
    pub fn resize(&mut self, css_w: u32, css_h: u32, dpr: f64) -> Vec<i32> {
        self.inner
            .resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
    }
    /// An action's error message, or an empty string.
    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.inner.act(name, value).err().unwrap_or_default()
    }
    pub fn ui(&self) -> String {
        self.inner.ui().to_string()
    }
    pub fn numbers(&self) -> String {
        self.inner.numbers().to_string()
    }
    pub fn mesh(&self, id: usize) -> Vec<f32> {
        floats(self.inner.meshes().get(id).map(|m| m.as_slice()).unwrap_or(&[]))
    }
    pub fn water_mesh(&self) -> Vec<f32> {
        floats(&self.inner.water_mesh())
    }
    pub fn sky_mesh(&self) -> Vec<f32> {
        floats(&tool_core::mesh::sky_triangle())
    }
    pub fn constants(&self) -> Vec<f32> {
        floats(
            &self
                .inner
                .constants()
                .into_iter()
                .flat_map(|(_, (_, v))| v)
                .collect::<Vec<i32>>(),
        )
    }
    pub fn commands(&self) -> Vec<u32> {
        self.inner.commands().to_vec()
    }
    pub fn values(&self) -> Vec<f32> {
        floats(self.inner.values())
    }
    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.frame_uniforms())
    }
    /// Counts up when the place or boat changes: the GPU's constants and
    /// flow map are then out of date.
    pub fn generation(&self) -> u32 {
        self.inner.generation
    }
    pub fn flow_size(&self) -> Vec<i32> {
        self.inner.flow_size()
    }
    pub fn flow_map(&self) -> Vec<u8> {
        self.inner.flow_map().to_vec()
    }
    pub fn checksum(&self) -> String {
        self.inner.checksum().to_string()
    }
}
