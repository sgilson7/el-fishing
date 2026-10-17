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
use crate::mesh::Shape;
use crate::render::{Batch, Frame3, Pass, Rgb};
use crate::river::River;
use crate::ticks::TICKS_PER_SECOND;
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

/// How a frame is seen.
pub struct View {
    pub meshes: usize,
    /// False in the scope's view: the eye is the angler's, so the angler,
    /// the rod in their hands and the kayak under them are not drawn, and
    /// the scope sees a fish below the boat through it (T9).
    pub own_body: bool,
    /// The mesh number of this place's first species.
    pub fish_base: usize,
    /// A landed fish held up to be looked at: which, where, facing which way (T11).
    pub shown: Option<(usize, V3, i32)>,
    /// The eye, for the rain that falls round it.
    pub eye: V3,
}

pub fn build(w: &World, look: &Look, view: &View) -> Batch {
    let mut b = Batch::new(view.meshes);
    if view.own_body {
        if w.troll().is_some() {
            troller(&mut b, w, look);
        } else if w.fishing() {
            kayak(&mut b, w, look);
        } else {
            speedboat(&mut b, w, look);
        }
        angler(&mut b, w, look);
    }
    // the rod and line stay out of the close-up of a landed fish
    if w.troll().is_some() {
        spread(&mut b, w, look, view.shown.is_none());
    } else if w.fishing() && view.shown.is_none() {
        rod_and_line(&mut b, w, look, view.own_body);
    }
    for (i, f) in w.fish.iter().enumerate() {
        match view.shown {
            Some((j, at, heading)) if j == i => {
                let mut held = f.clone();
                held.pos = at;
                held.vel = V3::ZERO;
                held.heading = heading;
                held.mouth = 0;
                fish(&mut b, w, &held, i, view.fish_base);
            }
            _ => fish(&mut b, w, f, i, view.fish_base),
        }
    }
    scenery(&mut b, w, look);
    if let Some(r) = &w.sea.river {
        river(&mut b, r, look);
    }
    if w.place.rain > 0 {
        rain(&mut b, w, look, view.eye);
    }
    b
}

/// Rain (T18): streaks falling at 9 m/s and slanting with the wind, each
/// fixed in the world and wrapped into a box 40 m across round the eye, so
/// they neither swim with the camera nor run out. Only seen.
fn rain(b: &mut Batch, w: &World, look: &Look, eye: V3) {
    let col = look.colour("rain");
    let span = 40 * ONE;
    let tall = 24 * ONE;
    let fall = 9 * ONE;
    let wrap = |v: Fx, around: Fx, size: Fx| around + (v - around + size / 2).rem_euclid(size) - size / 2;
    let t = (w.tick % 6000) as i32 * ONE / TICKS_PER_SECOND as i32;
    // a streak shows a twentieth of a second of fall
    let streak = v3(w.sea.wind.x / 20, -fall / 20, w.sea.wind.z / 20);
    for k in 0..w.place.rain {
        let x = jitter(k * 3) * (span / 1000);
        let z = jitter(k * 3 + 1) * (span / 1000);
        let drop = fx::mul(fall, t) + jitter(k * 3 + 2) * (tall / 1000);
        let y = wrap(-drop, eye.y, tall);
        let drift = fx::mul(w.sea.wind.x, t);
        let drift_z = fx::mul(w.sea.wind.z, t);
        let at = v3(wrap(x + drift, eye.x, span), y, wrap(z + drift_z, eye.z, span));
        let end = at.add(streak);
        b.push(
            Shape::Cyl as usize,
            Pass::Glow,
            at.lerp(end, ONE / 2),
            &Frame3::between(at, end, ONE / 150),
            col,
            look.rain_glow * ONE / 1000,
        );
    }
}

/// The trolling boat (T26): a centre-console hull with a T-top, outriggers
/// swung out to the sides and rods in holders at the stern.
fn troller(b: &mut Batch, w: &World, look: &Look) {
    let col = |n: &str| look.colour(n);
    let h = &w.hull;
    let s = &h.spec;
    let k = |cm: i32, kayak: i32| ONE * cm / kayak;
    let hf = Frame3 {
        x: h.rotate(v3(k(s.beam_cm, 86) * 3 / 4, 0, 0)),
        y: h.rotate(v3(0, k(s.depth_cm, 30) * 2 / 3, 0)),
        z: h.rotate(v3(0, 0, k(s.length_cm, 390))),
    };
    b.solid(Shape::Hull as usize, h.pos, &hf, col("boat_hull"));
    part(
        b,
        h,
        Shape::Cube,
        cm(0, 60, -40),
        cm(190, 6, 560),
        col("boat_deck"),
    );
    // the console and the T-top over it
    part(
        b,
        h,
        Shape::Cube,
        cm(0, 100, 30),
        cm(90, 80, 70),
        col("boat_deck"),
    );
    part(b, h, Shape::Cube, cm(0, 132, 66), cm(80, 36, 4), col("lens"));
    part(
        b,
        h,
        Shape::Cube,
        cm(0, 260, 20),
        cm(200, 8, 180),
        col("boat_trim"),
    );
    for sx in [-1, 1] {
        for sz in [-1, 1] {
            let foot = h.to_world(cm(sx * 90, 60, 20 + sz * 80));
            let top = h.to_world(cm(sx * 90, 256, 20 + sz * 80));
            bone(b, foot, top, ONE * 3 / 100, col("reel"));
        }
    }
    // two outboards on the transom
    for sx in [-1, 1] {
        part(
            b,
            h,
            Shape::Cube,
            cm(sx * 45, 80, -385),
            cm(40, 70, 45),
            col("dark"),
        );
        part(
            b,
            h,
            Shape::Cube,
            cm(sx * 45, 20, -400),
            cm(12, 80, 20),
            col("dark"),
        );
    }
    // the outriggers: from the T-top's sides out to their clips
    if let Some(t) = w.troll() {
        for r in 0..t.rods.len() {
            let tip = t.tip_local(r);
            if tip.x.abs() > 2 * ONE {
                let side = if tip.x > 0 { 1 } else { -1 };
                bone(
                    b,
                    h.to_world(cm(side * 100, 260, 20)),
                    h.to_world(tip),
                    ONE * 3 / 100,
                    col("rod"),
                );
            }
        }
    }
}

/// The speedboat: a planing hull with a deck, a windscreen, a console, a
/// seat and an outboard on the transom. Driven, not fished from.
fn speedboat(b: &mut Batch, w: &World, look: &Look) {
    let col = |n: &str| look.colour(n);
    let h = &w.hull;
    let s = &h.spec;
    // the hull mesh is lofted at the kayak's size; stretch it to this hull's
    let k = |cm: i32, kayak: i32| ONE * cm / kayak;
    let hf = Frame3 {
        x: h.rotate(v3(k(s.beam_cm, 86) * 3 / 4, 0, 0)),
        y: h.rotate(v3(0, k(s.depth_cm, 30) * 2 / 3, 0)),
        z: h.rotate(v3(0, 0, k(s.length_cm, 390))),
    };
    b.solid(Shape::Hull as usize, h.pos, &hf, col("boat_hull"));
    part(
        b,
        h,
        Shape::Cube,
        cm(0, 38, -40),
        cm(150, 6, 380),
        col("boat_deck"),
    );
    part(b, h, Shape::Cube, cm(0, 62, 40), cm(70, 40, 40), col("boat_trim"));
    part(b, h, Shape::Cube, cm(0, 92, 62), cm(120, 30, 4), col("lens"));
    part(b, h, Shape::Cube, cm(0, 52, -40), cm(60, 20, 60), col("seat"));
    part(b, h, Shape::Cube, cm(0, 80, -68), cm(60, 50, 8), col("seat"));
    part(b, h, Shape::Cube, cm(0, 60, -270), cm(36, 56, 40), col("dark"));
    part(b, h, Shape::Cube, cm(0, 10, -290), cm(10, 70, 18), col("dark"));
}

fn kayak(b: &mut Batch, w: &World, look: &Look) {
    let col = |n: &str| look.colour(n);
    let h = &w.hull;
    // the hull, lofted at its own size, and what is fixed to it
    let hf = Frame3 {
        x: h.rotate(v3(ONE, 0, 0)),
        y: h.rotate(v3(0, ONE, 0)),
        z: h.rotate(v3(0, 0, ONE)),
    };
    b.solid(Shape::Hull as usize, h.pos, &hf, col("hull"));
    part(b, h, Shape::Cube, cm(0, 31, -5), cm(62, 4, 150), col("dark"));
    part(b, h, Shape::Cube, cm(0, 36, -35), cm(50, 8, 50), col("seat"));
    part(b, h, Shape::Cube, cm(0, 64, -60), cm(50, 50, 7), col("seat"));
    part(b, h, Shape::Cube, cm(0, 48, 55), cm(10, 42, 10), col("dark"));
    part(b, h, Shape::Cube, cm(0, 46, -120), cm(40, 30, 36), col("crate"));
    part(b, h, Shape::Cube, cm(0, 10, -190), cm(2, 40, 22), col("dark"));
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

/// One line from `from` along its points, the scene's one bright colour with a
/// glow, and its lure unless a fish has it.
fn draw_line(b: &mut Batch, look: &Look, pts: &[V3], from: V3, lure_shown: bool) {
    let line = look.colour("line");
    let glow = look.line_glow * ONE / 1000;
    for i in 0..pts.len() - 1 {
        let a = if i == 0 { from } else { pts[i] };
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
    if lure_shown {
        let e = pts[pts.len() - 1];
        let lure = look.colour("lure");
        b.push(
            Shape::Ball as usize,
            Pass::Solid,
            e,
            &Frame3::upright(cm(6, 6, 14)),
            lure,
            ONE / 2,
        );
        b.push(
            Shape::Ball as usize,
            Pass::Glow,
            e,
            &Frame3::upright(cm(24, 24, 24)),
            lure,
            look.lure_glow * ONE / 1000,
        );
    }
}

/// A trolling boat's rods in their holders and every line of its spread.
fn spread(b: &mut Batch, w: &World, look: &Look, lines: bool) {
    let Some(t) = w.troll() else {
        return;
    };
    let h = &w.hull;
    let col = |n: &str| look.colour(n);
    for r in 0..t.rods.len() {
        let tip = h.to_world(t.tip_local(r));
        // a flat line's rod stands in its holder; an outrigger's rod stands
        // at the gunwale and its line runs out to the clip
        let base = t.tip_local(r);
        let butt = h.to_world(v3(
            base.x.signum() * ONE * 85 / 100,
            ONE * 70 / 100,
            -ONE * 300 / 100,
        ));
        let rod_tip = if base.x.abs() > 2 * ONE {
            h.to_world(v3(
                base.x.signum() * ONE * 120 / 100,
                ONE * 230 / 100,
                -ONE * 380 / 100,
            ))
        } else {
            tip
        };
        bone(b, butt, rod_tip, ONE * 15 / 1000, col("rod"));
        if lines {
            let line = w.rod_line(r);
            let held = r == w.in_play && w.holder.is_some();
            draw_line(b, look, &line.pts, tip, !held);
        }
    }
}

fn fish(b: &mut Batch, w: &World, f: &crate::fish::Fish, i: usize, base: usize) {
    let species = f.species;
    let spec = &w.place.fish[species];
    let fwd = f.facing();
    let side = fwd.cross(v3(0, ONE, 0)).with_len(ONE);
    let up = side.cross(fwd).with_len(ONE);
    let len = f.length;
    let frame = Frame3::axes(side, up, fwd, v3(len, len, len));
    b.solid(base + species, f.pos, &frame, [ONE, ONE, ONE]);
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

/// Far sea stacks and low clouds, placed by the place's fixed numbers so they never move.
fn scenery(b: &mut Batch, w: &World, look: &Look) {
    let stack = look.colour("stack");
    let top = look.colour("stack_top");
    for &[x, z, s] in &w.place.scenery.stacks {
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
    for &[x, y, z, s] in &w.place.scenery.clouds {
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

/// A whole number from 0 to 999 that looks random but is fixed by `k`:
/// the banks' sizes come from it, so they are the same every frame.
fn jitter(k: i32) -> i32 {
    let mut x = (k as u32).wrapping_mul(2_654_435_761) ^ 0x9e37_79b9;
    x ^= x >> 15;
    x = x.wrapping_mul(2_246_822_519);
    x ^= x >> 13;
    (x % 1000) as i32
}

/// The river's banks, bed and rocks, after the river mockup: ledges of rock
/// in terraces up from each bank with pines along them, a gravel bed, and
/// boulders standing out of the current. Nothing here is a rule; the banks
/// that stop the kayak are the river's own (`river.rs`).
fn river(b: &mut Batch, r: &River, look: &Look) {
    let rock_a = look.colour("ledge");
    let rock_b = look.colour("ledge_dark");
    let pine = look.colour("pine");
    let trunk = look.colour("trunk");
    let bed = look.colour("bed");
    let boulder = look.colour("boulder");
    let m = |cm: i32| cm * ONE / 100;
    // terraces: each bank, three steps back from the water and up
    for side in [-1, 1] {
        for step in 0..3 {
            let edge = r.half + m([0, 600, 1500][step]);
            let depth = m([900, 1300, 2600][step]);
            let across = side * (edge + depth / 2);
            let radius = r.radius + across;
            // blocks about 10 m long round the loop
            let count = (fx::mul(radius, crate::sea::TAU) / (10 * ONE)).max(12);
            for i in 0..count {
                let key = side * 10_000 + step as i32 * 1000 + i;
                let turns = i * fx::TURN / count + jitter(key) * 20;
                let at = r.point(turns, across + side * m(jitter(key + 7) / 2));
                let rise = m([150, 450, 1000][step] + jitter(key + 3) * [1, 3, 6][step]);
                let bottom = -r.mid_depth - ONE;
                let tall = rise - bottom;
                let along = r.along(at);
                let yaw = crate::geom::yaw_of(along.x, along.z);
                let long = m(1100 + jitter(key + 11));
                let size = v3(depth + m(200), tall, long);
                b.solid(
                    Shape::Cube as usize,
                    v3(at.x, bottom + tall / 2, at.z),
                    &Frame3::turned(yaw, 0, 0, size),
                    if (i + step as i32) % 3 == 0 {
                        rock_b
                    } else {
                        rock_a
                    },
                );
                // pines on the upper steps
                if step > 0 && jitter(key + 5) < 450 {
                    let t = m(500 + jitter(key + 9) * 9);
                    let off = r.point(turns, across + side * m(jitter(key + 13) / 3 - 150));
                    b.solid(
                        Shape::Cyl as usize,
                        v3(off.x, rise + t / 8, off.z),
                        &Frame3::upright(v3(m(40), t / 4, m(40))),
                        trunk,
                    );
                    b.solid(
                        Shape::Cone as usize,
                        v3(off.x, rise + t * 6 / 10, off.z),
                        &Frame3::upright(v3(t / 3, t * 8 / 10, t / 3)),
                        pine,
                    );
                }
            }
        }
    }
    // the bed: nine strips across, in pieces round the loop, each at its
    // depth; fewer strips showed their steps through clear water as bands
    let pieces = 72;
    for strip in 0..9 {
        let s = (strip * 2 - 8) * ONE / 9;
        let across = fx::mul(s, r.half);
        for i in 0..pieces {
            let turns = i * fx::TURN / pieces;
            let at = r.point(turns, across);
            let y = r.channel_bed(at);
            let along = r.along(at);
            let yaw = crate::geom::yaw_of(along.x, along.z);
            let long = fx::mul(r.radius + across, crate::sea::TAU) / pieces + ONE;
            b.solid(
                Shape::Cube as usize,
                v3(at.x, y - ONE / 4, at.z),
                &Frame3::turned(yaw, 0, 0, v3(r.half * 2 / 9 + ONE / 2, ONE / 2, long)),
                bed,
            );
        }
    }
    // the boulders, from the bed to their tops
    for (i, k) in r.rocks.iter().enumerate() {
        let bottom = r.channel_bed(k.at).min(-ONE / 2);
        let tall = k.top - bottom;
        b.solid(
            Shape::Rock as usize,
            v3(k.at.x, bottom + tall / 2, k.at.z),
            &Frame3::turned(i as i32 * 9_973, 0, 0, v3(k.r * 2, tall, k.r * 2)),
            boulder,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::SHAPES;
    use crate::session::tests::session;

    #[test]
    fn the_scope_sees_through_the_kayak() {
        // from the angler's eye a fish under the boat would be hidden by the
        // hull, so the scope's view draws no hull (T9)
        let s = session(1);
        let n = s.meshes().len();
        let view = |own_body: bool| View {
            meshes: n,
            own_body,
            fish_base: SHAPES,
            shown: None,
            eye: V3::ZERO,
        };
        let seen = build(&s.world, &s.look, &view(true));
        let scoped = build(&s.world, &s.look, &view(false));
        assert_eq!(seen.count_of(Shape::Hull as usize, Pass::Solid), 1);
        assert_eq!(scoped.count_of(Shape::Hull as usize, Pass::Solid), 0);
    }
}
