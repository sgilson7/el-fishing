//! The angler's arms and the rod, driven joint by joint in the manner of
//! QWOP and Vagrancy. Each arm has a shoulder and an elbow that swing in
//! the angler's upright plane; a motor turns a joint while its key is held
//! and lets it coast to a stop when the key is let go.
//!
//! The right hand holds the grip and the left hand the butt below it, so
//! the rod runs from the left hand through the right and on to the tip. The
//! steady-hands setting blends that line toward the right forearm's, so a
//! player can cast with the right arm alone.
//!
//! Everything here is in hull space (`hull.rs`): x across, y up, z toward
//! the bow. Angles are in turns (`fx::TURN`); a shoulder at 0 hangs the
//! arm straight down and a quarter turn holds it straight forward.

use serde::Deserialize;

use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::ticks::TICKS_PER_SECOND;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmSpec {
    /// The shoulders' middle in hull space, cm.
    pub shoulder_cm: [i32; 3],
    pub shoulder_half_width_cm: i32,
    /// How far each hand sits from the middle.
    pub hand_half_width_cm: i32,
    pub upper_arm_cm: i32,
    pub forearm_cm: i32,
    pub shoulder_min_deg: i32,
    pub shoulder_max_deg: i32,
    pub elbow_min_deg: i32,
    pub elbow_max_deg: i32,
    pub shoulder_speed_deg_s: i32,
    pub elbow_speed_deg_s: i32,
    pub motor_accel_deg_s2: i32,
    /// The share of a joint's speed kept each tick after its key is let go, percent.
    pub coast_pct: i32,
    /// The motors' speed at the slow setting, percent.
    pub slow_pct: i32,
    /// Where the arms start: left shoulder, left elbow, right shoulder, right elbow.
    pub start_deg: [i32; 4],
    pub rod_cm: i32,
    pub grip_from_butt_cm: i32,
    /// The angle the rod rises above the right forearm when held in one hand.
    pub rod_rise_deg: i32,
}

/// Joint order: left shoulder, left elbow, right shoulder, right elbow.
pub const JOINTS: usize = 4;

/// Each joint's two keys: the first raises (or bends), the second lowers (or straightens).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArmInput {
    /// Per joint: 1, -1 or 0.
    pub drive: [i32; JOINTS],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arms {
    pub spec: ArmSpec,
    pub angle: [i32; JOINTS],
    /// Angle units a tick.
    pub speed: [i32; JOINTS],
    /// 0 off, 1 some, 2 more.
    pub steady: i32,
    pub slow: bool,
    last_dir: V3,
}

/// The rod in hull space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rod {
    pub butt: V3,
    pub grip: V3,
    pub tip: V3,
    /// Unit direction from butt to tip.
    pub dir: V3,
}

/// Where an arm's joints are, in hull space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limb {
    pub shoulder: V3,
    pub elbow: V3,
    pub hand: V3,
}

fn deg(d: i32) -> i32 {
    d * fx::TURN / 360
}

fn cm(c: [i32; 3]) -> V3 {
    v3(c[0] * ONE / 100, c[1] * ONE / 100, c[2] * ONE / 100)
}

/// A unit direction in the upright plane, `a` turns from straight down toward forward.
fn swing(a: i32) -> V3 {
    v3(0, -fx::cos(a), fx::sin(a))
}

impl Arms {
    pub fn new(spec: ArmSpec) -> Arms {
        let angle = spec.start_deg.map(deg);
        let mut a = Arms {
            spec,
            angle,
            speed: [0; JOINTS],
            steady: 1,
            slow: false,
            last_dir: v3(0, 0, ONE),
        };
        a.last_dir = a.rod().dir;
        a
    }

    fn limits(&self, j: usize) -> (i32, i32) {
        let s = &self.spec;
        if j % 2 == 0 {
            (deg(s.shoulder_min_deg), deg(s.shoulder_max_deg))
        } else {
            (deg(s.elbow_min_deg), deg(s.elbow_max_deg))
        }
    }

    /// One tick of the four motors.
    pub fn tick(&mut self, input: ArmInput) {
        let s = &self.spec;
        let per_tick = |d: i32| deg(d) / TICKS_PER_SECOND as i32;
        let accel = deg(s.motor_accel_deg_s2) / (TICKS_PER_SECOND as i32 * TICKS_PER_SECOND as i32);
        for j in 0..JOINTS {
            let mut top = per_tick(if j % 2 == 0 {
                s.shoulder_speed_deg_s
            } else {
                s.elbow_speed_deg_s
            });
            if self.slow {
                top = top * s.slow_pct / 100;
            }
            let d = input.drive[j].signum();
            self.speed[j] = if d == 0 {
                self.speed[j] * s.coast_pct / 100
            } else {
                (self.speed[j] + d * accel.max(1)).clamp(-top, top)
            };
            let (lo, hi) = self.limits(j);
            let next = self.angle[j] + self.speed[j];
            if next <= lo || next >= hi {
                self.speed[j] = 0;
            }
            self.angle[j] = next.clamp(lo, hi);
        }
        self.last_dir = self.rod().dir;
    }

    /// The left arm (`right` false) or the right arm.
    pub fn limb(&self, right: bool) -> Limb {
        let s = &self.spec;
        let side = if right { 1 } else { -1 };
        let (a, e) = if right {
            (self.angle[2], self.angle[3])
        } else {
            (self.angle[0], self.angle[1])
        };
        let base = cm(s.shoulder_cm);
        let shoulder = base.add(v3(side * s.shoulder_half_width_cm * ONE / 100, 0, 0));
        let hand_x = side * s.hand_half_width_cm * ONE / 100;
        let upper = swing(a).scale(s.upper_arm_cm * ONE / 100);
        let fore = swing(a + e).scale(s.forearm_cm * ONE / 100);
        let mut elbow = shoulder.add(upper);
        elbow.x = (shoulder.x + hand_x) / 2;
        let mut hand = shoulder.add(upper).add(fore);
        hand.x = hand_x;
        Limb { shoulder, elbow, hand }
    }

    /// The rod: from the two hands, blended toward the right forearm's line
    /// by the steady-hands setting.
    pub fn rod(&self) -> Rod {
        let s = &self.spec;
        let left = self.limb(false).hand;
        let right = self.limb(true).hand;
        let between = right.sub(left);
        let hands = if between.len() < ONE / 20 {
            self.last_dir
        } else {
            between.with_len(ONE)
        };
        let one_hand = swing(self.angle[2] + self.angle[3] + deg(s.rod_rise_deg));
        let dir = hands
            .scale(ONE * (2 - self.steady.clamp(0, 2)) / 2)
            .add(one_hand.scale(ONE * self.steady.clamp(0, 2) / 2))
            .with_len(ONE);
        let grip = right;
        let butt = grip.sub(dir.scale(s.grip_from_butt_cm * ONE / 100));
        let tip = butt.add(dir.scale(s.rod_cm * ONE / 100));
        Rod { butt, grip, tip, dir }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn spec() -> ArmSpec {
        ArmSpec {
            shoulder_cm: [0, 90, -29],
            shoulder_half_width_cm: 20,
            hand_half_width_cm: 4,
            upper_arm_cm: 31,
            forearm_cm: 30,
            shoulder_min_deg: -30,
            shoulder_max_deg: 200,
            elbow_min_deg: 0,
            elbow_max_deg: 150,
            shoulder_speed_deg_s: 240,
            elbow_speed_deg_s: 300,
            motor_accel_deg_s2: 2400,
            coast_pct: 70,
            slow_pct: 60,
            start_deg: [20, 100, 65, 45],
            rod_cm: 210,
            grip_from_butt_cm: 35,
            rod_rise_deg: 20,
        }
    }

    #[test]
    fn a_shoulder_a_quarter_turn_up_holds_the_arm_straight_ahead() {
        let mut a = Arms::new(spec());
        a.angle = [deg(90), 0, deg(90), 0];
        // shoulder at (0.2, 0.9, -0.29); arm 0.31 + 0.30 = 0.61 m forward
        let l = a.limb(true);
        assert!((l.hand.z - (-29 + 61) * ONE / 100).abs() < 200, "{:?}", l.hand);
        assert!((l.hand.y - 90 * ONE / 100).abs() < 200, "{:?}", l.hand);
    }

    #[test]
    fn a_held_key_turns_its_joint_and_a_let_go_joint_coasts_to_rest() {
        let mut a = Arms::new(spec());
        let start = a.angle;
        let mut input = ArmInput::default();
        input.drive[2] = 1;
        for _ in 0..30 {
            a.tick(input);
        }
        // half a second at up to 240 degrees a second, after speeding up
        assert!(a.angle[2] - start[2] > deg(80), "{}", a.angle[2] - start[2]);
        assert_eq!(a.angle[0], start[0], "the other joints stay");
        for _ in 0..60 {
            a.tick(ArmInput::default());
        }
        assert_eq!(a.speed[2], 0, "the motor coasts to rest");
    }

    #[test]
    fn joints_stop_at_their_limits() {
        let mut a = Arms::new(spec());
        let mut input = ArmInput::default();
        input.drive[3] = 1;
        for _ in 0..240 {
            a.tick(input);
        }
        assert_eq!(a.angle[3], deg(150));
    }

    #[test]
    fn the_rod_runs_from_the_left_hand_through_the_right() {
        let mut a = Arms::new(spec());
        a.steady = 0;
        let r = a.rod();
        let l = a.limb(false).hand;
        // the left hand lies on the rod's line, behind the grip
        let off = l.sub(r.grip);
        assert!(off.cross(r.dir).len() < ONE / 100, "{off:?} {:?}", r.dir);
        assert!(off.dot(r.dir) < 0, "behind the grip");
        assert!((r.tip.sub(r.butt).len() - 210 * ONE / 100).abs() < ONE / 100);
    }
}
