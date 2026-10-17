//! What the WebGL page draws, as numbers the core builds (Builder D3),
//! after Mage Tower Battle's `render.rs`. A frame is a list of commands,
//! each of `COMMAND_VALUES` numbers: which program, which mesh, where its
//! instances start in the frame's values, how many, and which pass.
//!
//! - The sky: one triangle over the screen, its colour from the view ray.
//! - Shapes: instanced meshes, 16 values an instance (three columns of the
//!   transform with the translation in their w, then rgb and glow), lit the
//!   Ape Race way; a glow pass adds light with no depth writes.
//! - The water: the grid, displaced by the same waves the core floats things
//!   on, drawn see-through after the shapes so fish show beneath it.

use crate::fx::{self, Fx, ONE};
use crate::geom::{self, v3, M4, V3};

pub const INSTANCE_VALUES: usize = 16;
pub const COMMAND_VALUES: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    Sky = 0,
    Shapes = 1,
    Water = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pass {
    Solid = 0,
    Glow = 1,
}

/// A colour, 0..=ONE each.
pub type Rgb = [Fx; 3];

pub fn hex(s: &str) -> Rgb {
    let h = s.trim_start_matches('#');
    let p = |i: usize| i32::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128);
    [p(0) * ONE / 255, p(2) * ONE / 255, p(4) * ONE / 255]
}

/// The columns of a shape's transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame3 {
    pub x: V3,
    pub y: V3,
    pub z: V3,
}

impl Frame3 {
    pub fn upright(size: V3) -> Frame3 {
        Frame3 {
            x: v3(size.x, 0, 0),
            y: v3(0, size.y, 0),
            z: v3(0, 0, size.z),
        }
    }

    /// A unit cylinder or cone stood from `a` to `b`, `r` across.
    pub fn between(a: V3, b: V3, r: Fx) -> Frame3 {
        let y = b.sub(a);
        let up = if y.x.abs() < y.len() / 2 && y.z.abs() < y.len() / 2 {
            v3(ONE, 0, 0)
        } else {
            v3(0, ONE, 0)
        };
        let x = y.cross(up).with_len(r);
        let z = x.cross(y).with_len(r);
        Frame3 { x, y, z }
    }

    /// Columns turned by a yaw, pitch and roll (turns) and scaled.
    pub fn turned(yaw: i32, pitch: i32, roll: i32, size: V3) -> Frame3 {
        Frame3 {
            x: geom::rotate(v3(size.x, 0, 0), yaw, pitch, roll),
            y: geom::rotate(v3(0, size.y, 0), yaw, pitch, roll),
            z: geom::rotate(v3(0, 0, size.z), yaw, pitch, roll),
        }
    }

    /// Columns along three given axes (unit vectors), scaled.
    pub fn axes(x: V3, y: V3, z: V3, size: V3) -> Frame3 {
        Frame3 {
            x: x.scale(size.x),
            y: y.scale(size.y),
            z: z.scale(size.z),
        }
    }
}

/// Instances grouped by pass and mesh.
#[derive(Debug, Clone, Default)]
pub struct Batch {
    meshes: usize,
    lists: Vec<Vec<i32>>,
}

impl Batch {
    pub fn new(meshes: usize) -> Batch {
        Batch {
            meshes,
            lists: vec![Vec::new(); 2 * meshes],
        }
    }

    pub fn push(&mut self, mesh: usize, pass: Pass, at: V3, f: &Frame3, rgb: Rgb, glow: Fx) {
        let l = &mut self.lists[pass as usize * self.meshes + mesh];
        l.extend_from_slice(&[
            f.x.x, f.x.y, f.x.z, at.x, f.y.x, f.y.y, f.y.z, at.y, f.z.x, f.z.y, f.z.z, at.z, rgb[0], rgb[1],
            rgb[2], glow,
        ]);
    }

    pub fn solid(&mut self, mesh: usize, at: V3, f: &Frame3, rgb: Rgb) {
        self.push(mesh, Pass::Solid, at, f, rgb, 0);
    }

    pub fn count(&self) -> usize {
        self.lists.iter().map(|l| l.len() / INSTANCE_VALUES).sum()
    }

    /// Appends one pass's draws.
    pub fn emit(&self, pass: Pass, out: &mut FrameOut) {
        for mesh in 0..self.meshes {
            let l = &self.lists[pass as usize * self.meshes + mesh];
            if l.is_empty() {
                continue;
            }
            let offset = (out.values.len() * 4) as u32;
            out.values.extend_from_slice(l);
            out.commands.extend_from_slice(&[
                Program::Shapes as u32,
                mesh as u32,
                offset,
                (l.len() / INSTANCE_VALUES) as u32,
                pass as u32,
                0,
            ]);
        }
    }
}

/// A camera: where it is, where it looks, and its lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub at: V3,
    /// 1 / tan(half the vertical field of view), Q16.
    pub focal: Fx,
}

impl Camera {
    pub fn matrix(&self, aspect: Fx) -> M4 {
        let proj = geom::perspective(self.focal, aspect.max(1), ONE / 10, 2400 * ONE);
        geom::mat_mul(&proj, &geom::look_at(self.eye, self.at))
    }

    /// The sky's view rays: forward, and right and up scaled to the screen's edges.
    pub fn rays(&self, aspect: Fx) -> [V3; 3] {
        let f = self.at.sub(self.eye).with_len(ONE);
        let r = f.cross(v3(0, ONE, 0)).with_len(ONE);
        let u = r.cross(f);
        let tan = fx::div(ONE, self.focal.max(1));
        [f, r.scale(fx::mul(tan, aspect)), u.scale(tan)]
    }
}

/// One frame's draw commands and the values they read.
#[derive(Debug, Clone, Default)]
pub struct FrameOut {
    pub commands: Vec<u32>,
    pub values: Vec<i32>,
}

impl FrameOut {
    pub fn plain(&mut self, program: Program, mesh: u32) {
        self.commands
            .extend_from_slice(&[program as u32, mesh, 0, 1, 0, 0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_emits_one_draw_per_mesh_and_pass_with_byte_offsets() {
        let mut b = Batch::new(3);
        let f = Frame3::upright(v3(ONE, ONE, ONE));
        b.solid(2, V3::ZERO, &f, [ONE, 0, 0]);
        b.solid(2, v3(ONE, 0, 0), &f, [ONE, 0, 0]);
        b.solid(0, V3::ZERO, &f, [0, ONE, 0]);
        let mut out = FrameOut::default();
        out.plain(Program::Sky, 0);
        b.emit(Pass::Solid, &mut out);
        // the sky, then mesh 0 (one instance) and mesh 2 (two)
        assert_eq!(out.commands.len(), 3 * COMMAND_VALUES);
        assert_eq!(&out.commands[6..12], &[Program::Shapes as u32, 0, 0, 1, 0, 0]);
        // mesh 2's instances start after mesh 0's 16 values: byte 64
        assert_eq!(&out.commands[12..18], &[Program::Shapes as u32, 2, 64, 2, 0, 0]);
        assert_eq!(out.values.len(), 3 * INSTANCE_VALUES);
    }

    #[test]
    fn a_stood_cylinder_spans_its_two_ends() {
        let f = Frame3::between(v3(0, 0, 0), v3(0, 0, 2 * ONE), ONE / 10);
        assert_eq!(f.y, v3(0, 0, 2 * ONE));
        assert!((f.x.len() - ONE / 10).abs() < 20 && (f.z.len() - ONE / 10).abs() < 20);
        assert_eq!(f.x.dot(f.y), 0);
    }
}
