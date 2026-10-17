//! Fixed ticks from display frames, with taps latched between ticks. Part of
//! the realtime kit (`kits/realtime/README.md`), after Pursuer's `advance`.
//!
//! The page calls `Clock::frame` once per display frame with the frame's
//! timestamp and the input bits from `web/input.js`. The clock says which
//! ticks run, and with which bits. Rules run on ticks; the display frame
//! rate never changes a result.
//!
//! The page's timestamp is milliseconds with a fraction. The shim converts
//! it to whole microseconds (one multiplication and a cast, which decides
//! nothing), so the core sees an integer.
//!
//! A display frame can come sooner than a tick. A key pressed and released
//! inside such a frame reaches the core in that frame's bits, and no tick
//! runs to use it. The clock keeps those bits and adds them to the next
//! tick that runs, so a quick tap is never lost.

/// Ticks a second, and the length of one in microseconds.
pub const TICKS_PER_SECOND: i64 = 60;
pub const TICK_US: i64 = 1_000_000 / TICKS_PER_SECOND;

/// The most ticks one display frame may run. After a stall (a hidden tab, a
/// debugger) the rest is dropped, so the run does not race to catch up.
pub const MAX_TICKS_PER_FRAME: usize = 5;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Clock {
    last: Option<i64>,
    owed: i64,
    latched: u32,
}

impl Clock {
    /// The input bits for each tick to run this frame, in order. Often
    /// empty, often one, more after a slow frame.
    /// How far the display is toward the next tick, in sixty-five-thousand-
    /// five-hundred-and-thirty-sixths. Drawing only: a rule never reads it.
    pub fn between(&self) -> i32 {
        ((self.owed.clamp(0, TICK_US) << 16) / TICK_US) as i32
    }

    pub fn frame(&mut self, now_us: i64, bits: u32) -> Vec<u32> {
        let elapsed = match self.last {
            Some(prev) => (now_us - prev).clamp(0, TICK_US * MAX_TICKS_PER_FRAME as i64),
            // the first frame runs one tick
            None => TICK_US,
        };
        self.last = Some(now_us);
        self.owed += elapsed;
        let mut ticks = Vec::new();
        while self.owed >= TICK_US && ticks.len() < MAX_TICKS_PER_FRAME {
            self.owed -= TICK_US;
            ticks.push(bits | self.latched);
            self.latched = 0;
        }
        if ticks.is_empty() {
            self.latched |= bits;
        }
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tap_inside_a_short_frame_waits_for_the_next_tick() {
        let mut c = Clock::default();
        assert_eq!(c.frame(0, 0), vec![0]);
        assert!(c.frame(TICK_US / 4, 1).is_empty());
        assert_eq!(c.frame(TICK_US + 1, 0), vec![1]);
    }

    #[test]
    fn a_stall_runs_at_most_the_cap() {
        let mut c = Clock::default();
        c.frame(0, 0);
        assert_eq!(c.frame(10_000_000, 2).len(), MAX_TICKS_PER_FRAME);
    }

    #[test]
    fn the_same_second_runs_the_same_ticks_at_any_frame_rate() {
        let mut counts = Vec::new();
        for fps in [30i64, 60, 144] {
            let mut c = Clock::default();
            let n: usize = (0..=fps).map(|f| c.frame(f * 1_000_000 / fps, 0).len()).sum();
            counts.push(n);
        }
        // one tick on the first frame, then 60 whole ticks in the second
        assert_eq!(counts, vec![61, 61, 61]);
    }
}
