//! A river that loops back on itself (T16): a ring of water between two
//! banks, flowing the same way all the way round, so a kayak that drifts
//! with it comes back to where it started and the current never stops.
//! This is the water tab's L2 for a river: an authored flow, not a solved
//! one. The water runs fastest down the middle and slows to nothing at the
//! banks; each rock turns the flow round it the way flow passes a cylinder
//! (the potential-flow doublet) and leaves a wake behind it, slack near the
//! rock and turning back on itself in an eddy, where trout hold.
//!
//! Place space: the kayak starts on the centreline at the origin, and the
//! loop's centre lies one radius toward -x, so the river there runs toward
//! -z. Rocks are placed by how far downstream of the start they lie, in
//! degrees round the loop, and how far across from the centreline.

use serde::Deserialize;

use crate::fx::{self, Fx, ONE};
use crate::geom::{v3, V3};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RockSpec {
    /// Degrees downstream of the start, round the loop.
    pub along_deg: i32,
    /// From the centreline, toward the outer bank positive.
    pub across_cm: i32,
    pub r_cm: i32,
    /// How far its top stands above the water; below it when negative.
    pub top_cm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverSpec {
    /// The centreline's radius round the loop.
    pub radius_cm: i32,
    pub width_cm: i32,
    /// The depth beside each bank; the sea's depth is the depth mid-channel.
    pub bank_depth_cm: i32,
    /// How high the land stands above the water beyond the banks.
    pub land_cm: i32,
    /// A rock's wake runs this many of its radii downstream.
    pub wake_radii: i32,
    /// How fast the eddy just behind a rock runs back upstream, as a share
    /// of the flow beside the rock, percent.
    pub eddy_pct: i32,
    pub rocks: Vec<RockSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rock {
    pub at: V3,
    pub r: Fx,
    pub top: Fx,
    /// The flow's direction past the rock, and its speed there without it.
    pub flow: V3,
    pub speed: Fx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct River {
    pub centre: V3,
    pub radius: Fx,
    pub half: Fx,
    /// Mid-channel speed.
    pub speed: Fx,
    pub mid_depth: Fx,
    pub bank_depth: Fx,
    pub land: Fx,
    pub wake_radii: i32,
    pub eddy: Fx,
    pub rocks: Vec<Rock>,
}

fn cm(v: i32) -> Fx {
    v * ONE / 100
}

impl River {
    pub fn new(s: &RiverSpec, speed_mm_s: i32, mid_depth: Fx) -> River {
        let radius = cm(s.radius_cm);
        let mut river = River {
            centre: v3(-radius, 0, 0),
            radius,
            half: cm(s.width_cm) / 2,
            speed: speed_mm_s * ONE / 1000,
            mid_depth,
            bank_depth: cm(s.bank_depth_cm),
            land: cm(s.land_cm),
            wake_radii: s.wake_radii.max(1),
            eddy: s.eddy_pct * ONE / 100,
            rocks: Vec::new(),
        };
        river.rocks = s
            .rocks
            .iter()
            .map(|r| {
                let at = river.point(r.along_deg * fx::TURN / 360, cm(r.across_cm));
                let flow = river.along(at);
                Rock {
                    at,
                    r: cm(r.r_cm),
                    top: cm(r.top_cm),
                    flow,
                    speed: river.base_speed(at),
                }
            })
            .collect();
        river
    }

    /// The point `turns` downstream of the start, `across` from the centreline.
    pub fn point(&self, turns: i32, across: Fx) -> V3 {
        let r = self.radius + across;
        self.centre
            .add(v3(fx::mul(r, fx::cos(turns)), 0, -fx::mul(r, fx::sin(turns))))
    }

    /// How far across the channel a point lies: -ONE at the inner bank, ONE at the outer.
    pub fn across(&self, p: V3) -> Fx {
        let r = p.sub(self.centre).flat_len();
        fx::div(r - self.radius, self.half)
    }

    /// The unit direction the river runs at a point: round the loop, clockwise seen from above.
    pub fn along(&self, p: V3) -> V3 {
        let d = p.sub(self.centre).flat();
        if d.flat_len() == 0 {
            return v3(0, 0, -ONE);
        }
        v3(d.z, 0, -d.x).with_len(ONE)
    }

    /// The flow's speed at a point with no rocks: the mid-channel speed
    /// times 1 − s⁴, where s runs from -1 at one bank to 1 at the other.
    pub fn base_speed(&self, p: V3) -> Fx {
        let s = self.across(p);
        if s.abs() >= ONE {
            return 0;
        }
        let s2 = fx::mul(s, s);
        fx::mul(self.speed, ONE - fx::mul(s2, s2))
    }

    /// The water's velocity at a point: the channel's flow, turned round
    /// each rock and slowed (or turned back) in each rock's wake.
    pub fn current(&self, p: V3) -> V3 {
        let mut v = self.along(p).scale(self.base_speed(p));
        if v == V3::ZERO {
            return v;
        }
        let mut slow = ONE;
        for k in &self.rocks {
            let d = p.sub(k.at).flat();
            let dist = d.flat_len();
            if dist <= k.r {
                return V3::ZERO;
            }
            let reach = k.r * self.wake_radii;
            if dist > reach * 2 {
                continue;
            }
            // round the rock: v' = U [ŵ − (R²/d²)(2 (ŵ·d̂) d̂ − ŵ)], added as the
            // change from the plain flow
            let dn = d.with_len(ONE);
            let q = fx::div(fx::mul(k.r, k.r), fx::mul(dist, dist));
            let wd = k.flow.dot(dn) as Fx;
            let bend = dn.scale(2 * wd).sub(k.flow);
            v = v.sub(bend.scale(fx::mul(k.speed, q)));
            // the wake: from the rock's back downstream, widening from one
            // radius to two, slack in the middle and reversed near the rock
            let a = d.dot(k.flow) as Fx;
            if a > 0 && a < reach {
                let b = (d.x as i64 * k.flow.z as i64 - d.z as i64 * k.flow.x as i64).abs() >> 16;
                let width = k.r + fx::mul(k.r, fx::div(a, reach));
                if (b as Fx) < width {
                    let lat = fx::div(b as Fx, width);
                    let f = fx::mul(ONE - fx::div(a, reach), ONE - fx::mul(lat, lat));
                    slow = fx::mul(slow, ONE - fx::mul(ONE + self.eddy, f));
                }
            }
        }
        v.scale(slow)
    }

    /// The height of the bottom at a point: the channel's bed, deepest mid-channel
    /// (bank depth plus the difference times 1 − s²); the land beyond the
    /// banks; a rock's top where a rock stands.
    pub fn bed(&self, p: V3) -> Fx {
        for k in &self.rocks {
            if p.sub(k.at).flat_len() < k.r {
                return k.top;
            }
        }
        self.channel_bed(p)
    }

    /// The bottom at a point with the rocks left out.
    pub fn channel_bed(&self, p: V3) -> Fx {
        let s = self.across(p);
        if s.abs() >= ONE {
            return self.land;
        }
        let deep = self.bank_depth + fx::mul(self.mid_depth - self.bank_depth, ONE - fx::mul(s, s));
        -deep
    }

    /// How far to move a point across the water to keep it `margin` clear
    /// of the banks and the rocks; zero when it is clear already.
    pub fn push_out(&self, p: V3, margin: Fx) -> V3 {
        let d = p.sub(self.centre).flat();
        let r = d.flat_len();
        let inner = self.radius - self.half + margin;
        let outer = self.radius + self.half - margin;
        let mut push = if r < inner && r > 0 {
            d.with_len(inner - r)
        } else if r > outer {
            d.with_len(outer - r)
        } else {
            V3::ZERO
        };
        let q = p.add(push);
        for k in &self.rocks {
            let d = q.sub(k.at).flat();
            let dist = d.flat_len();
            let clear = k.r + margin;
            if dist < clear {
                let away = if dist == 0 { k.flow } else { d.with_len(ONE) };
                push = push.add(away.scale(clear - dist));
            }
        }
        push
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A loop 100 m round its centreline and 40 m wide, 1 m/s mid-channel,
    /// 3.5 m deep mid-channel and 0.6 m at the banks.
    fn river(rocks: Vec<RockSpec>) -> River {
        River::new(
            &RiverSpec {
                radius_cm: 10_000,
                width_cm: 4000,
                bank_depth_cm: 60,
                land_cm: 150,
                wake_radii: 8,
                eddy_pct: 40,
                rocks,
            },
            1000,
            7 * ONE / 2,
        )
    }

    fn near(a: Fx, b: Fx, tol: Fx) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn the_flow_is_fastest_mid_channel_and_still_at_the_banks() {
        let r = river(vec![]);
        // at the start the river runs toward -z at the full 1 m/s
        let v = r.current(V3::ZERO);
        assert!(near(v.z, -ONE, ONE / 100) && near(v.x, 0, ONE / 100), "{v:?}");
        // halfway to the outer bank, s = 0.5: 1 − 0.5⁴ = 0.9375 m/s
        let v = r.current(v3(10 * ONE, 0, 0));
        assert!(near(v.flat_len(), ONE * 9375 / 10_000, ONE / 200), "{v:?}");
        // at the bank and beyond it, none
        assert_eq!(r.current(v3(20 * ONE, 0, 0)), V3::ZERO);
        assert_eq!(r.current(v3(30 * ONE, 0, 0)), V3::ZERO);
    }

    #[test]
    fn the_river_runs_round_the_loop_back_to_the_start() {
        let r = river(vec![]);
        // a quarter of the way round, at (-100, -100), it runs toward -x
        let v = r.current(v3(-100 * ONE, 0, -100 * ONE));
        assert!(near(v.x, -ONE, ONE / 100) && near(v.z, 0, ONE / 100), "{v:?}");
        // half way, at (-200, 0), toward +z; three quarters, at (-100, 100), toward +x
        let v = r.current(v3(-200 * ONE, 0, 0));
        assert!(near(v.z, ONE, ONE / 100), "{v:?}");
        let v = r.current(v3(-100 * ONE, 0, 100 * ONE));
        assert!(near(v.x, ONE, ONE / 100), "{v:?}");
    }

    #[test]
    fn a_rock_slows_the_flow_ahead_speeds_it_beside_and_turns_it_back_behind() {
        // a rock of 1 m radius on the centreline, a little downstream of the start
        let r = river(vec![RockSpec {
            along_deg: 6,
            across_cm: 0,
            r_cm: 100,
            top_cm: 50,
        }]);
        let k = &r.rocks[0];
        let u = k.speed;
        // two radii upstream: U (1 − R²/d²) = 1 × (1 − 1/4) = 0.75 m/s
        let ahead = r.current(k.at.sub(k.flow.scale(2 * ONE)));
        assert!(near(ahead.flat_len(), u * 3 / 4, ONE / 50), "ahead {ahead:?}");
        // two radii aside, outside the wake: U (1 + 1/4) = 1.25 m/s
        let side = v3(k.flow.z, 0, -k.flow.x);
        let beside = r.current(k.at.add(side.scale(2 * ONE)));
        assert!(near(beside.flat_len(), u * 5 / 4, ONE / 50), "beside {beside:?}");
        // one and a half radii behind: the doublet leaves U (1 − 1/2.25) = 0.556,
        // and the wake scales it by 1 − 1.4 × (1 − 1.5/8) = −0.1375, so the
        // water runs back upstream at 0.076 m/s
        let behind = r.current(k.at.add(k.flow.scale(3 * ONE / 2)));
        let along = behind.dot(k.flow) as Fx;
        assert!(near(along, -u * 76 / 1000, ONE / 100), "behind {behind:?}");
        // inside the rock, none
        assert_eq!(r.current(k.at), V3::ZERO);
    }

    #[test]
    fn the_bed_is_deepest_mid_channel_and_the_land_stands_above_the_water() {
        let r = river(vec![]);
        assert_eq!(r.bed(V3::ZERO), -7 * ONE / 2);
        // s = 0.5: 0.6 + 2.9 × (1 − 0.25) = 2.775 m down
        assert!(near(r.bed(v3(10 * ONE, 0, 0)), -ONE * 2775 / 1000, ONE / 200));
        assert_eq!(r.bed(v3(25 * ONE, 0, 0)), ONE * 150 / 100);
    }

    #[test]
    fn a_point_on_the_bank_or_in_a_rock_is_pushed_back_into_the_water() {
        let r = river(vec![RockSpec {
            along_deg: 0,
            across_cm: -1000,
            r_cm: 200,
            top_cm: 50,
        }]);
        // 25 m out with a 1 m margin: back to 19 m, 6 m inward
        let push = r.push_out(v3(25 * ONE, 0, 0), ONE);
        assert!(
            near(push.x, -6 * ONE, ONE / 100) && near(push.z, 0, ONE / 100),
            "{push:?}"
        );
        // 1 m from the rock's middle at (-10, 0): out to its 2 m radius
        let push = r.push_out(v3(-9 * ONE, 0, 0), 0);
        assert!(near(push.x, ONE, ONE / 100), "{push:?}");
        assert_eq!(r.push_out(v3(5 * ONE, 0, 0), ONE), V3::ZERO);
    }
}
