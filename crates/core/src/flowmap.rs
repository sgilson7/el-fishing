//! The flow map: the current sampled on a grid a metre square, packed as
//! bytes for the water shader (the water tab's "the GPU reads the same grid
//! as a texture"). Red and green carry the current across and along x and
//! z, blue the white water round the rocks. The shader draws streaks that
//! run with the current and foam where the blue is; nothing reads it back,
//! so it is only seen. The current itself is the river's (`river.rs`).

use crate::fx::{self, Fx, ONE};
use crate::geom::v3;
use crate::river::River;
use crate::sea::Sea;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowMap {
    pub width: usize,
    pub height: usize,
    /// RGBA, row by row from the box's corner.
    pub bytes: Vec<u8>,
    /// The box the map covers: its corner (x, z) and its size, metres.
    pub corner: (Fx, Fx),
    pub size: (Fx, Fx),
}

/// A value of ±`range` packed into a byte, 128 for zero.
fn pack(v: Fx, range: Fx) -> u8 {
    (128 + fx::div(v, range.max(1)) * 127 / ONE).clamp(0, 255) as u8
}

/// The white water at a point, 0 to ONE: a ring round each rock where the
/// current piles against it, and the two edges of its wake, fading downstream.
fn white(r: &River, x: Fx, z: Fx) -> Fx {
    let p = v3(x, 0, z);
    let mut most = 0;
    for k in &r.rocks {
        let d = p.sub(k.at).flat();
        let dist = d.flat_len();
        let reach = k.r * r.wake_radii;
        if dist > reach {
            continue;
        }
        let strength = fx::div(k.speed, r.speed.max(1)).min(ONE);
        let under = if k.top < 0 { ONE / 2 } else { ONE };
        // the ring against the rock
        let ring = (ONE - fx::div((dist - k.r).max(0), k.r * 4 / 5)).max(0);
        // the wake's edges
        let a = d.dot(k.flow) as Fx;
        let mut edge = 0;
        if a > 0 && a < reach {
            let b = ((d.x as i64 * k.flow.z as i64 - d.z as i64 * k.flow.x as i64).abs() >> 16) as Fx;
            let width = k.r + fx::mul(k.r, fx::div(a, reach));
            let off = (fx::div(b, width) - ONE).abs();
            edge = fx::mul((ONE - off * 3).max(0), ONE - fx::div(a, reach));
        }
        most = most.max(fx::mul(fx::mul(ring.max(edge), strength), under));
    }
    most
}

pub fn build(sea: &Sea, range: Fx) -> FlowMap {
    let Some(r) = &sea.river else {
        // the open sea: one current everywhere
        let c = sea.current_at(0, 0);
        return FlowMap {
            width: 1,
            height: 1,
            bytes: vec![pack(c.x, range), pack(c.z, range), 0, 255],
            corner: (0, 0),
            size: (ONE, ONE),
        };
    };
    let reach = r.radius + r.half + 4 * ONE;
    let n = (fx::floor(reach) * 2) as usize;
    let corner = (r.centre.x - reach, r.centre.z - reach);
    let mut bytes = Vec::with_capacity(n * n * 4);
    for j in 0..n {
        for i in 0..n {
            let x = corner.0 + i as i32 * ONE + ONE / 2;
            let z = corner.1 + j as i32 * ONE + ONE / 2;
            let c = sea.current_at(x, z);
            let w = white(r, x, z);
            bytes.extend_from_slice(&[pack(c.x, range), pack(c.z, range), (w * 255 / ONE) as u8, 255]);
        }
    }
    FlowMap {
        width: n,
        height: n,
        bytes,
        corner,
        size: (n as i32 * ONE, n as i32 * ONE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_carries_the_current_both_ways() {
        // ±3 m/s in a byte: zero is 128, +1.5 m/s is 128 + 63.5, -3 m/s is 1
        let range = 3 * ONE;
        assert_eq!(pack(0, range), 128);
        assert_eq!(pack(3 * ONE / 2, range), 191);
        assert_eq!(pack(-3 * ONE, range), 1);
        assert_eq!(pack(9 * ONE, range), 255);
    }
}
