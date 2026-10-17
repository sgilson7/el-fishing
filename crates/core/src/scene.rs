//! The shapes a frame draws, from the world: the kayak and its angler, the
//! rod bending under the line, the line itself as the scene's one bright
//! colour, the lure, the fish, and the far sea stacks and clouds. After the
//! mockups' `kayak()`, `angler()`, `rod()` and `fish()`. Nothing here is a
//! rule; it reads the world and the look.

use crate::fish::Mood;
use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};
use crate::hull::Hull;
use crate::look::Look;
use crate::mesh::{Shape, SHAPES};
use crate::render::{Batch, Frame3, Pass, Rgb};
use crate::world::World;

fn cm(x: i32, y: i32, z: i32) -> V3 {
    v3(x * ONE / 100, y * ONE / 100, z * ONE / 100)
}

/// A box, cylinder or ball fixed to the hull: its middle and size in hull space.
fn part(b: &mut Batch, h: &Hull, shape: Shape, at: V3, size: V3, rgb: Rgb) {
    let f = Frame3 {
        x: h.rotate(v3(size.x, 0, 0)),
        y: h.rotate(v3(0, size.y, 0)),
        z: h.rotate(v3(0, 0, size.z)),
    };
    b.solid(shape as usize, h.to_world(at), &f, rgb);
}

/// A limb or a rod section: a cylinder between two world points.
fn bone(b: &mut Batch, a: V3, c: V3, r: Fx, rgb: Rgb) {
    if c.sub(a).len() == 0 {
        return;
    }
    b.solid(
        Shape::Cyl as usize,
        a.lerp(c, ONE / 2),
        &Frame3::between(a, c, r * 2),
        rgb,
    );
}

/// `own_body` is false in the scope's view: the eye is the angler's, so the
/// angler and the rod in their hands are not drawn.
pub fn build(w: &World, look: &Look, meshes: usize, own_body: bool) -> Batch {
    let mut b = Batch::new(meshes);
    let col = |n: &str| look.colour(n);
    let h = &w.hull;
    // the hull, lofted at its own size, and what is fixed to it
    let hf = Frame3 {
        x: h.rotate(v3(ONE, 0, 0)),
        y: h.rotate(v3(0, ONE, 0)),
        z: h.rotate(v3(0, 0, ONE)),
    };
    b.solid(Shape::Hull as usize, h.pos, &hf, col("hull"));
    part(&mut b, h, Shape::Cube, cm(0, 31, -5), cm(62, 4, 150), col("dark"));
    part(&mut b, h, Shape::Cube, cm(0, 36, -35), cm(50, 8, 50), col("seat"));
    part(&mut b, h, Shape::Cube, cm(0, 64, -60), cm(50, 50, 7), col("seat"));
    part(&mut b, h, Shape::Cube, cm(0, 48, 55), cm(10, 42, 10), col("dark"));
    part(
        &mut b,
        h,
        Shape::Cube,
        cm(0, 46, -120),
        cm(40, 30, 36),
        col("crate"),
    );
    part(
        &mut b,
        h,
        Shape::Cube,
        cm(0, 10, -190),
        cm(2, 40, 22),
        col("dark"),
    );
    if own_body {
        angler(&mut b, w, look);
    }
    rod_and_line(&mut b, w, look, own_body);
    for (i, f) in w.fish.iter().enumerate() {
        fish(&mut b, w, i, f.species);
    }
    scenery(&mut b, look);
    b
}

fn angler(b: &mut Batch, w: &World, look: &Look) {
    let h = &w.hull;
    let col = |n: &str| look.colour(n);
    let to = |p: V3| h.to_world(p);
    // legs from the hips to the pedals
    for s in [-1, 1] {
        let hip = cm(s * 12, 44, -33);
        let knee = cm(s * 14, 70, 15);
        let foot = cm(s * 15, 62, 68);
        bone(b, to(hip), to(knee), ONE * 8 / 100, col("pants"));
        bone(b, to(knee), to(foot), ONE * 6 / 100, col("pants"));
        part(b, h, Shape::Cube, cm(s * 15, 62, 72), cm(11, 8, 24), col("dark"));
    }
    // a life vest over a shirt, the head, the cap and the scope over the right eye
    part(b, h, Shape::Cube, cm(0, 46, -33), cm(42, 22, 30), col("pants"));
    part(b, h, Shape::Cube, cm(0, 72, -30), cm(44, 50, 30), col("vest"));
    part(b, h, Shape::Cube, cm(0, 66, -30), cm(46, 5, 32), col("dark"));
    part(b, h, Shape::Cube, cm(0, 109, -26), cm(24, 26, 24), col("skin"));
    part(b, h, Shape::Cube, cm(0, 99, -19), cm(25, 12, 12), col("beard"));
    part(b, h, Shape::Cyl, cm(0, 123, -27), cm(28, 10, 30), col("cap"));
    part(b, h, Shape::Cube, cm(0, 119, -11), cm(24, 2, 16), col("cap"));
    part(b, h, Shape::Cube, cm(0, 112, -26), cm(27, 4, 26), col("dark"));
    let eye = cm(6, 112, -14);
    let tube_end = cm(6, 112, 16);
    bone(b, to(eye), to(tube_end), ONE * 5 / 100, col("brass"));
    let lens = to(cm(6, 112, 17));
    b.push(
        Shape::Ball as usize,
        Pass::Solid,
        lens,
        &Frame3::upright(cm(9, 9, 9)),
        col("lens"),
        ONE,
    );
    b.push(
        Shape::Ball as usize,
        Pass::Glow,
        lens,
        &Frame3::upright(cm(16, 16, 16)),
        col("lens"),
        ONE / 3,
    );
    // the arms, from the arm motors
    for right in [false, true] {
        let l = w.arms.limb(right);
        bone(b, to(l.shoulder), to(l.elbow), ONE * 6 / 100, col("shirt"));
        bone(b, to(l.elbow), to(l.hand), ONE * 5 / 100, col("skin"));
        b.solid(
            Shape::Ball as usize,
            to(l.shoulder),
            &Frame3::upright(cm(14, 14, 14)),
            col("vest"),
        );
        b.solid(
            Shape::Cube as usize,
            to(l.hand),
            &Frame3::upright(cm(8, 9, 10)),
            col("skin"),
        );
    }
}

fn rod_and_line(b: &mut Batch, w: &World, look: &Look, own_body: bool) {
    let h = &w.hull;
    let col = |n: &str| look.colour(n);
    let rod = w.arms.rod();
    let butt = h.to_world(rod.butt);
    let tip = h.to_world(rod.tip);
    let bend = w.line.bend;
    // the rod bends toward the line, most at the tip
    let n = 8;
    let at = |i: i32| {
        let t = ONE * i / n;
        butt.lerp(tip, t).add(bend.scale(fx::mul(t, t)))
    };
    if own_body {
        for i in 0..n {
            let r = ONE * (22 - 2 * i) / 1000;
            bone(
                b,
                at(i),
                at(i + 1),
                r,
                if i < 2 { col("cork") } else { col("rod") },
            );
        }
        let reel = butt.lerp(tip, ONE / 8).add(h.rotate(v3(0, -ONE * 8 / 100, 0)));
        b.solid(
            Shape::Ball as usize,
            reel,
            &Frame3::upright(cm(9, 9, 9)),
            col("reel"),
        );
    }
    // the line from the bent tip, the scene's one bright colour, with a glow
    let line = col("line");
    let glow = look.line_glow * ONE / 1000;
    let pts = &w.line.pts;
    let first = at(n);
    for i in 0..pts.len() - 1 {
        let a = if i == 0 { first } else { pts[i] };
        let c = pts[i + 1];
        if c.sub(a).len() == 0 {
            continue;
        }
        let mid = a.lerp(c, ONE / 2);
        b.push(
            Shape::Cyl as usize,
            Pass::Solid,
            mid,
            &Frame3::between(a, c, ONE / 160),
            line,
            ONE,
        );
        b.push(
            Shape::Cyl as usize,
            Pass::Glow,
            mid,
            &Frame3::between(a, c, ONE / 40),
            line,
            glow,
        );
    }
    // the lure, unless a fish has it in its mouth
    let held = w.holder.is_some();
    if !held {
        let e = w.line.end();
        let lure = col("lure");
        b.push(
            Shape::Ball as usize,
            Pass::Solid,
            e,
            &Frame3::upright(cm(4, 4, 8)),
            lure,
            ONE / 2,
        );
        b.push(
            Shape::Ball as usize,
            Pass::Glow,
            e,
            &Frame3::upright(cm(18, 18, 18)),
            lure,
            look.lure_glow * ONE / 1000,
        );
    }
}

fn fish(b: &mut Batch, w: &World, i: usize, species: usize) {
    let f = &w.fish[i];
    let spec = &w.place.fish[species];
    let fwd = f.facing();
    let side = fwd.cross(v3(0, ONE, 0)).with_len(ONE);
    let up = side.cross(fwd).with_len(ONE);
    let len = f.length;
    let frame = Frame3::axes(side, up, fwd, v3(len, len, len));
    b.solid(SHAPES + species, f.pos, &frame, [ONE, ONE, ONE]);
    let fin = crate::render::hex(&spec.colours[3]);
    // the tail, swept side to side faster as the fish works harder
    let beat = match f.mood {
        Mood::Hooked | Mood::Flee | Mood::Strike => 6,
        _ => 2,
    };
    let phase = ((w.tick % 600) as i32 * fx::TURN / 60 * beat + i as i32 * 9_000) % fx::TURN;
    let sweep = fx::mul(fx::sin(phase), ONE * 35 / 100);
    let tail_at = f.pos.sub(fwd.scale(len * 45 / 100));
    let tail_dir = fwd.scale(ONE).add(side.scale(sweep)).with_len(ONE);
    let tail_side = tail_dir.cross(up).with_len(ONE);
    let depth = spec.depth_pct * ONE / 100;
    b.solid(
        Shape::Fin as usize,
        tail_at,
        &Frame3::axes(tail_side, up, tail_dir, v3(len, fx::mul(len, depth) * 2, len / 4)),
        fin,
    );
    // the dorsal fin and the eyes
    let top = f
        .pos
        .add(up.scale(fx::mul(len, depth) * 4 / 10))
        .add(fwd.scale(len / 10));
    b.solid(
        Shape::Fin as usize,
        top,
        &Frame3::axes(
            side,
            fwd.scale(-ONE),
            up,
            v3(len, len / 3, fx::mul(len, depth) * 6 / 10),
        ),
        fin,
    );
    let width = spec.width_pct * ONE / 100;
    for s in [-1, 1] {
        let eye = f
            .pos
            .add(fwd.scale(len * 36 / 100))
            .add(up.scale(fx::mul(len, depth) / 8))
            .add(side.scale(s * fx::mul(len, width) * 7 / 10));
        let r = len / 22;
        b.solid(
            Shape::Ball as usize,
            eye,
            &Frame3::upright(v3(r, r, r)),
            [ONE * 95 / 100, ONE * 92 / 100, ONE * 86 / 100],
        );
        b.solid(
            Shape::Ball as usize,
            eye.add(side.scale(s * r / 3)),
            &Frame3::upright(v3(r * 6 / 10, r * 6 / 10, r * 6 / 10)),
            [ONE / 14, ONE / 14, ONE / 14],
        );
    }
    // the open mouth while striking
    if f.mouth > 0 {
        let m = f.mouth_at();
        let r = fx::mul(len, depth) * f.mouth / ONE / 3;
        b.solid(
            Shape::Ball as usize,
            m,
            &Frame3::upright(v3(r, r, r)),
            [ONE / 6, ONE / 12, ONE / 12],
        );
    }
}

/// Far sea stacks and low clouds, placed by fixed numbers so they never move.
fn scenery(b: &mut Batch, look: &Look) {
    let stack = look.colour("stack");
    let top = look.colour("stack_top");
    for (x, z, s) in [
        (-320, -520, 35),
        (-262, -560, 24),
        (420, -700, 30),
        (-600, 380, 28),
    ] {
        let s = s * ONE / 10;
        let mut y = -2 * ONE;
        for k in 0..4 {
            let r = fx::mul(s, (40 - 6 * k) * ONE / 10);
            let hgt = fx::mul(s, (5 + k % 2) * ONE);
            let at = v3(x * ONE + k * ONE / 2, y + hgt / 2, z * ONE);
            b.solid(
                Shape::Rock as usize,
                at,
                &Frame3::turned(k * 9_000, 0, 0, v3(r * 2, hgt * 13 / 10, r * 2)),
                stack,
            );
            y += hgt;
        }
        b.solid(
            Shape::Rock as usize,
            v3(x * ONE, y, z * ONE),
            &Frame3::upright(v3(fx::mul(s, 3 * ONE), s, fx::mul(s, 3 * ONE))),
            top,
        );
    }
    let cloud = look.colour("cloud");
    for (x, y, z, s) in [
        (-500, 120, -800, 22),
        (-150, 140, -900, 28),
        (300, 110, -760, 20),
        (650, 150, -500, 26),
        (-760, 130, 200, 24),
        (200, 160, 800, 30),
        (900, 120, 300, 18),
    ] {
        let s = s * ONE / 10;
        for k in 0..5 {
            let at = v3(
                x * ONE + (k - 2) * fx::mul(s, 12 * ONE),
                y * ONE + (k % 2) * s * 2,
                z * ONE + (k % 3 - 1) * s * 4,
            );
            let r = fx::mul(s, (9 + (k * 7) % 5) * ONE);
            b.push(
                Shape::Rock as usize,
                Pass::Solid,
                at,
                &Frame3::turned(k * 13_000, 0, 0, v3(r * 2, r, r * 2)),
                cloud,
                ONE / 3,
            );
        }
    }
}
