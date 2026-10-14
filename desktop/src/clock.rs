//! The board clock: always Belgian time, optionally offset or simulated.

use crate::model::{ClockMode, Settings, parse_hm};
use chrono::{Timelike, Utc};
use chrono_tz::Europe::Brussels;
use std::time::Instant;

pub struct Clock {
    anchor_real: Instant,
    anchor_sec: f64,
}

impl Clock {
    pub fn new(settings: &Settings) -> Self {
        let mut c = Self {
            anchor_real: Instant::now(),
            anchor_sec: 0.0,
        };
        c.restart(settings);
        c
    }

    /// Restarts the simulated clock at the configured start time.
    pub fn restart(&mut self, s: &Settings) {
        self.anchor_real = Instant::now();
        self.anchor_sec = parse_hm(&s.custom_start).unwrap_or(0.0) * 60.0;
    }

    /// Keeps the current simulated time, e.g. before changing the speed.
    pub fn reanchor(&mut self, s: &Settings) {
        if s.clock_mode == ClockMode::Custom {
            self.anchor_sec = self.now_seconds(s);
            self.anchor_real = Instant::now();
        }
    }

    /// Current time as seconds of the day.
    pub fn now_seconds(&self, s: &Settings) -> f64 {
        match s.clock_mode {
            ClockMode::Custom => {
                let elapsed = self.anchor_real.elapsed().as_secs_f64() * s.clock_speed;
                (self.anchor_sec + elapsed).rem_euclid(86400.0)
            }
            ClockMode::Live | ClockMode::Offset => {
                let now = Utc::now().with_timezone(&Brussels);
                let secs = now.num_seconds_from_midnight() as f64 + now.nanosecond() as f64 / 1e9;
                let offset = if s.clock_mode == ClockMode::Offset {
                    s.clock_offset * 60.0
                } else {
                    0.0
                };
                (secs + offset).rem_euclid(86400.0)
            }
        }
    }

    pub fn now_minutes(&self, s: &Settings) -> f64 {
        self.now_seconds(s) / 60.0
    }
}
