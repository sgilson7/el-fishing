//! A scripted angler plays the Open Swell through the world's public door:
//! it casts, lets the lure sink, winds in bursts, sets the hook on a bite
//! and fights by winding while the pull is under the drag. If it cannot
//! land a fish in ten minutes of play, a person probably cannot either.

use tool_core::fish::Mood;
use tool_core::fx::{self, ONE};
use tool_core::world::{bits, Event, GearSpec, PlaceSpec, World};

fn world(seed: u64) -> World {
    let gear: GearSpec = serde_json::from_str(include_str!("../../../data/gear.json")).unwrap();
    let place: PlaceSpec = serde_json::from_str(include_str!("../../../data/place.json")).unwrap();
    World::new(gear, place, seed)
}

/// A scripted angler: casts, lets the lure sink, winds in bursts, sets the
/// hook on a bite, and fights by winding while the pull is under the drag.
/// Returns the tick of the first landed fish, if any.
pub fn fish_for(seed: u64, ticks: u64) -> Option<u64> {
    use bits::*;
    let mut w = world(seed);
    let mut phase = 0u32; // 0 cast back, 1 forward, 2 sink, 3 retrieve, 4 set, 5 fight
    let mut t_in = 0u32;
    for t in 0..ticks {
        t_in += 1;
        let hooked = w.holder.map(|i| w.fish[i].mood == Mood::Hooked).unwrap_or(false);
        let bit = w.events.iter().any(|e| matches!(e, Event::Bit(_)));
        if bit && phase == 3 {
            phase = 4;
            t_in = 0;
        }
        if hooked && phase != 5 {
            phase = 5;
            t_in = 0;
        }
        let keys = match phase {
            0 => {
                if t_in > 28 {
                    phase = 1;
                    t_in = 0;
                }
                FINGER | RIGHT_SHOULDER_UP
            }
            1 => {
                if t_in > 22 {
                    phase = 2;
                    t_in = 0;
                }
                if t_in < 9 {
                    FINGER | RIGHT_SHOULDER_DOWN
                } else {
                    RIGHT_SHOULDER_DOWN
                }
            }
            2 => {
                if t_in > 150 {
                    phase = 3;
                    t_in = 0;
                }
                0
            }
            3 => {
                if w.line.out < 4 * ONE {
                    // back at the kayak with nothing: cast again
                    phase = 0;
                    t_in = 0;
                }
                if t_in % 90 < 45 {
                    WIND
                } else {
                    0
                }
            }
            4 => {
                if t_in > 12 {
                    phase = if hooked { 5 } else { 3 };
                    t_in = 0;
                }
                RIGHT_SHOULDER_UP | LEFT_SHOULDER_UP
            }
            _ => {
                if w.holder.is_none() {
                    phase = 3;
                }
                let lower = if w.arms.angle[2] > 70 * fx::TURN / 360 {
                    RIGHT_SHOULDER_DOWN
                } else {
                    0
                };
                if w.line.tension < w.line.drag() * 3 / 4 {
                    WIND | lower
                } else {
                    lower
                }
            }
        };
        w.step(keys);
        if w.landed.is_some() {
            return Some(t);
        }
        if w.events.contains(&Event::Snapped) {
            phase = 0;
            t_in = 0;
        }
    }
    None
}

#[test]
fn a_scripted_angler_lands_a_fish_within_ten_minutes() {
    // ten minutes of play, on three seeds
    let landed: Vec<Option<u64>> = (1..=3).map(|s| fish_for(s, 36_000)).collect();
    assert!(landed.iter().all(|l| l.is_some()), "{landed:?}");
}

/// How long the scripted angler takes, for MEASUREMENTS.md:
/// `cargo test --test angler -- --ignored --nocapture`
#[test]
#[ignore]
fn time_to_land_on_eight_seeds() {
    for seed in 1..=8 {
        let t = fish_for(seed, 72_000);
        println!("seed {seed}: landed at {:?} s", t.map(|t| t / 60));
    }
}
