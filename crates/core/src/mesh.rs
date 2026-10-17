//! The meshes everything is drawn from, after Mage Tower Battle's `mesh.rs`.
//! Each is a triangle list of (position, normal, colour) in Q16. Faces are
//! flat, each with its own normal, so shapes read as facets the way Ape
//! Race's do; the colour is a per-vertex tint the instance's colour
//! multiplies, which is how a fish carries its bands.
//!
//! Unit shapes sit on the origin and are one unit across, so an instance's
//! transform carries the size. The kayak's hull is built at its own size.
//! The water's grid and the sky's triangle are position-only meshes for
//! their own programs.

use crate::fish::FishSpec;
use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};

/// Values per vertex: position, normal, colour.
pub const VERTEX_VALUES: usize = 9;

/// The fixed meshes; each species' body follows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Cube = 0,
    /// eight sides, along y
    Cyl = 1,
    Cone = 2,
    /// an icosahedron with its corners nudged: rocks, clouds, sea stacks
    Rock = 3,
    /// a low globe: heads, eyes, the lure's glow
    Ball = 4,
    Hull = 5,
    /// one triangle, both faces: fins and tails
    Fin = 6,
}

pub const SHAPES: usize = 7;

const H: Fx = ONE / 2;
const WHITE: [Fx; 3] = [ONE, ONE, ONE];

fn push_tri(out: &mut Vec<i32>, a: V3, b: V3, c: V3, col: [Fx; 3]) {
    let n = b.sub(a).cross(c.sub(a)).with_len(ONE);
    for p in [a, b, c] {
        out.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, col[0], col[1], col[2]]);
    }
}

fn push_quad(out: &mut Vec<i32>, a: V3, b: V3, c: V3, d: V3, col: [Fx; 3]) {
    push_tri(out, a, b, c, col);
    push_tri(out, a, c, d, col);
}

/// A corner moved a little by a fixed pattern of its place, so faces that
/// share it still meet (Mage Tower Battle's `nudge`).
fn nudge(p: V3, amount: Fx) -> V3 {
    let h = |a: i32, b: i32, c: i32| {
        let k = (p.x / 97).wrapping_mul(a) ^ (p.y / 89).wrapping_mul(b) ^ (p.z / 83).wrapping_mul(c);
        (k.rem_euclid(2001) - 1000) * (amount / 1000)
    };
    v3(p.x + h(73, 151, 29), p.y + h(37, 61, 199), p.z + h(113, 17, 167))
}

fn ring(sides: i32, r: Fx, y: Fx, k: i32) -> V3 {
    let a = fx::TURN * k / sides;
    v3(fx::mul(fx::cos(a), r), y, -fx::mul(fx::sin(a), r))
}

/// A shape's vertex data.
pub fn shape(s: Shape) -> Vec<i32> {
    let mut o = Vec::new();
    match s {
        Shape::Cube => {
            let p = |x: i32, y: i32, z: i32| v3(x * H, y * H, z * H);
            push_quad(&mut o, p(1, -1, 1), p(1, -1, -1), p(1, 1, -1), p(1, 1, 1), WHITE);
            push_quad(
                &mut o,
                p(-1, -1, -1),
                p(-1, -1, 1),
                p(-1, 1, 1),
                p(-1, 1, -1),
                WHITE,
            );
            push_quad(&mut o, p(-1, 1, 1), p(1, 1, 1), p(1, 1, -1), p(-1, 1, -1), WHITE);
            push_quad(
                &mut o,
                p(-1, -1, -1),
                p(1, -1, -1),
                p(1, -1, 1),
                p(-1, -1, 1),
                WHITE,
            );
            push_quad(&mut o, p(-1, -1, 1), p(1, -1, 1), p(1, 1, 1), p(-1, 1, 1), WHITE);
            push_quad(
                &mut o,
                p(1, -1, -1),
                p(-1, -1, -1),
                p(-1, 1, -1),
                p(1, 1, -1),
                WHITE,
            );
        }
        Shape::Cyl | Shape::Cone => {
            let sides = if s == Shape::Cyl { 8 } else { 7 };
            let top_r = if s == Shape::Cyl { H } else { 0 };
            for k in 0..sides {
                let (a0, a1) = (ring(sides, H, -H, k), ring(sides, H, -H, k + 1));
                let (b0, b1) = (ring(sides, top_r, H, k), ring(sides, top_r, H, k + 1));
                if s == Shape::Cyl {
                    push_quad(&mut o, a0, a1, b1, b0, WHITE);
                    push_tri(&mut o, v3(0, H, 0), b0, b1, WHITE);
                } else {
                    push_tri(&mut o, a0, a1, v3(0, H, 0), WHITE);
                }
                push_tri(&mut o, v3(0, -H, 0), a1, a0, WHITE);
            }
        }
        Shape::Rock | Shape::Ball => {
            // an icosahedron from its twelve corners
            let t = ONE * 1618 / 1000;
            let raw = [
                (-1, t, 0),
                (1, t, 0),
                (-1, -t, 0),
                (1, -t, 0),
                (0, -1, t),
                (0, 1, t),
                (0, -1, -t),
                (0, 1, -t),
                (t, 0, -1),
                (t, 0, 1),
                (-t, 0, -1),
                (-t, 0, 1),
            ];
            let unit = |(x, y, z): (i32, i32, i32)| {
                let f = |c: i32| if c.abs() == 1 { c * ONE } else { c };
                v3(f(x), f(y), f(z)).with_len(H)
            };
            let c: Vec<V3> = raw
                .iter()
                .map(|&q| {
                    if s == Shape::Rock {
                        nudge(unit(q), ONE / 6)
                    } else {
                        unit(q)
                    }
                })
                .collect();
            let faces = [
                (0, 11, 5),
                (0, 5, 1),
                (0, 1, 7),
                (0, 7, 10),
                (0, 10, 11),
                (1, 5, 9),
                (5, 11, 4),
                (11, 10, 2),
                (10, 7, 6),
                (7, 1, 8),
                (3, 9, 4),
                (3, 4, 2),
                (3, 2, 6),
                (3, 6, 8),
                (3, 8, 9),
                (4, 9, 5),
                (2, 4, 11),
                (6, 2, 10),
                (8, 6, 7),
                (9, 8, 1),
            ];
            for (a, b, d) in faces {
                push_tri(&mut o, c[a], c[b], c[d], WHITE);
            }
        }
        Shape::Hull => hull(&mut o),
        Shape::Fin => {
            push_tri(&mut o, v3(0, 0, 0), v3(0, H, -ONE), v3(0, -H, -ONE), WHITE);
            push_tri(&mut o, v3(0, 0, 0), v3(0, -H, -ONE), v3(0, H, -ONE), WHITE);
        }
    }
    o
}

/// The kayak's hull, lofted from cross-sections along z (the bow is +z),
/// 3.9 m long, its keel at y = 0. After the mockup's `kayak()`.
fn hull(o: &mut Vec<i32>) {
    let half = ONE * 195 / 100;
    let n = 14;
    let section = |z: Fx| -> [V3; 7] {
        let t = fx::div(z, half);
        let t2 = fx::mul(t, t);
        let w = fx::mul(ONE * 43 / 100, fx::sqrt(ONE - t2).max(0)) + ONE / 200;
        let t3 = fx::mul(t2, t.abs());
        let d = fx::mul(ONE * 22 / 100, fx::sqrt((ONE - t3).max(0))) + ONE / 50;
        let sheer = fx::mul(ONE * 6 / 100, t2);
        let keel = ONE * 24 / 100 - d;
        let pts = [
            (-w, ONE * 30 / 100 + sheer),
            (-fx::mul(w, ONE * 98 / 100), ONE * 12 / 100),
            (-fx::mul(w, ONE * 62 / 100), keel + d / 4),
            (0, keel),
            (fx::mul(w, ONE * 62 / 100), keel + d / 4),
            (fx::mul(w, ONE * 98 / 100), ONE * 12 / 100),
            (w, ONE * 30 / 100 + sheer),
        ];
        pts.map(|(x, y)| v3(x, y, z))
    };
    let side = [ONE * 23 / 100, ONE * 56 / 100, ONE * 55 / 100];
    let dark = [ONE * 18 / 100, ONE * 46 / 100, ONE * 45 / 100];
    let deck = [ONE * 17 / 100, ONE * 41 / 100, ONE * 39 / 100];
    // tints relative to the instance's colour: the hull uses white light and these shades
    let shade = |c: [Fx; 3]| c.map(|v| fx::div(v, side[1]).min(ONE * 2));
    let rings: Vec<[V3; 7]> = (0..=n).map(|i| section(-half + 2 * half * i / n)).collect();
    for i in 0..n as usize {
        for k in 0..6 {
            let col = if k == 0 || k == 5 {
                shade(side)
            } else {
                shade(dark)
            };
            push_quad(
                o,
                rings[i][k],
                rings[i + 1][k],
                rings[i + 1][k + 1],
                rings[i][k + 1],
                col,
            );
        }
        push_quad(
            o,
            rings[i][0],
            rings[i][6],
            rings[i + 1][6],
            rings[i + 1][0],
            shade(deck),
        );
    }
}

fn hex(s: &str) -> [Fx; 3] {
    let h = s.trim_start_matches('#');
    let p = |i: usize| i32::from_str_radix(h.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(128);
    [p(0) * ONE / 255, p(2) * ONE / 255, p(4) * ONE / 255]
}

/// A species' body, one unit long along +z (the nose), lofted in rings and
/// coloured top to belly, with the species' pattern. The instance scales it
/// to the fish's length.
pub fn fish_body(spec: &FishSpec) -> Vec<i32> {
    let (top, side, belly) = (
        hex(&spec.colours[0]),
        hex(&spec.colours[1]),
        hex(&spec.colours[2]),
    );
    let depth = spec.depth_pct * ONE / 100;
    let width = spec.width_pct * ONE / 100;
    let n = 16;
    let k = 10;
    // the body's half-depth at t (0 nose, 1 tail): a rounded profile
    let prof = |t: Fx| -> Fx {
        let a = fx::mul(fx::sqrt(t.max(0)), ONE * 9 / 10) + ONE / 25;
        let s = fx::sin(fx::mul(a.min(ONE), fx::HALF_TURN));
        fx::mul(s.max(0), ONE / 2) + ONE / 70
    };
    let colour = |t: Fx, up: Fx| -> [Fx; 3] {
        let mix = |a: [Fx; 3], b: [Fx; 3], w: Fx| -> [Fx; 3] {
            [
                a[0] + fx::mul(b[0] - a[0], w),
                a[1] + fx::mul(b[1] - a[1], w),
                a[2] + fx::mul(b[2] - a[2], w),
            ]
        };
        let mut c = if up > ONE * 3 / 10 {
            mix(side, top, (up - ONE * 3 / 10) * 10 / 7)
        } else if up > -ONE * 35 / 100 {
            side
        } else {
            mix(side, belly, ((-up - ONE * 35 / 100) * 2).min(ONE))
        };
        if spec.pattern == "spots" && fx::mul(fx::sin(t * 9), fx::sin(up * 3)) > ONE * 7 / 10 && up > -ONE / 3
        {
            c = mix(c, [ONE * 16 / 100, ONE * 41 / 100, ONE * 85 / 100], ONE * 8 / 10);
        }
        c
    };
    let rings: Vec<Vec<(V3, Fx, Fx)>> = (0..=n)
        .map(|i| {
            let t = ONE * i / n;
            let r = prof(t);
            (0..k)
                .map(|j| {
                    let a = fx::TURN * j / k;
                    let (up, across) = (fx::cos(a), fx::sin(a));
                    let p = v3(
                        fx::mul(fx::mul(across, r), width) * 2,
                        fx::mul(fx::mul(up, r), depth) * 2,
                        H - t,
                    );
                    (p, t, up)
                })
                .collect()
        })
        .collect();
    let mut o = Vec::new();
    for i in 0..n as usize {
        for j in 0..k as usize {
            let j1 = (j + 1) % k as usize;
            let (a, b, c, d) = (rings[i][j], rings[i][j1], rings[i + 1][j1], rings[i + 1][j]);
            let col = colour((a.1 + c.1) / 2, (a.2 + b.2) / 2);
            push_quad(&mut o, a.0, d.0, c.0, b.0, col);
        }
    }
    // the nose
    let nose = v3(0, 0, H + ONE / 50);
    for j in 0..k as usize {
        let (a, b) = (rings[0][j], rings[0][(j + 1) % k as usize]);
        push_tri(&mut o, nose, b.0, a.0, colour(0, a.2));
    }
    o
}

/// The water's grid: dense under the camera and sparse at the rim, pushed
/// out by a square law (the mockup's `waterSheet`). Positions only.
pub fn water_grid(cells: i32, radius: Fx) -> Vec<i32> {
    let at = |i: i32| -> Fx {
        let u = ONE * 2 * i / cells - ONE;
        fx::mul(fx::mul(u, u.abs()), radius)
    };
    let mut o = Vec::with_capacity((cells * cells * 18) as usize);
    for j in 0..cells {
        for i in 0..cells {
            let (x0, x1, z0, z1) = (at(i), at(i + 1), at(j), at(j + 1));
            for (x, z) in [(x0, z0), (x0, z1), (x1, z0), (x1, z0), (x0, z1), (x1, z1)] {
                o.extend_from_slice(&[x, 0, z]);
            }
        }
    }
    o
}

/// One triangle that covers the screen, for the sky. Positions only.
pub fn sky_triangle() -> Vec<i32> {
    vec![-ONE, -ONE, 0, 3 * ONE, -ONE, 0, -ONE, 3 * ONE, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shape_is_whole_triangles_with_unit_normals() {
        for s in [
            Shape::Cube,
            Shape::Cyl,
            Shape::Cone,
            Shape::Rock,
            Shape::Ball,
            Shape::Hull,
            Shape::Fin,
        ] {
            let v = shape(s);
            assert!(!v.is_empty());
            assert_eq!(v.len() % (3 * VERTEX_VALUES), 0, "{s:?}");
            for c in v.chunks(VERTEX_VALUES) {
                let n = v3(c[3], c[4], c[5]).len();
                assert!((n - ONE).abs() < ONE / 50 || n == 0, "{s:?} normal {n}");
            }
        }
        // a cube is six faces of two triangles
        assert_eq!(shape(Shape::Cube).len(), 36 * VERTEX_VALUES);
    }

    #[test]
    fn the_hull_is_its_real_length_with_the_keel_at_zero() {
        let v = shape(Shape::Hull);
        let zs: Vec<i32> = v.chunks(VERTEX_VALUES).map(|c| c[2]).collect();
        let ys: Vec<i32> = v.chunks(VERTEX_VALUES).map(|c| c[1]).collect();
        let len = zs.iter().max().unwrap() - zs.iter().min().unwrap();
        assert!((len - 390 * ONE / 100).abs() < ONE / 100, "{len}");
        assert!(
            *ys.iter().min().unwrap() >= -ONE / 100,
            "keel {}",
            ys.iter().min().unwrap()
        );
    }

    #[test]
    fn the_water_grid_reaches_its_radius() {
        let g = water_grid(4, 100 * ONE);
        // 4 × 4 cells, 6 corners each, 3 values a corner
        assert_eq!(g.len(), 4 * 4 * 6 * 3);
        assert_eq!(*g.iter().step_by(3).max().unwrap(), 100 * ONE);
    }
}
