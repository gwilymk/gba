// basically a copy of sdl2_gfx framerate but since linking that is annoying, just reimplementing it here

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

pub struct FpsManager {
    frame_count: usize,
    rate_s: f64,
    base_instant: Instant,
    last_instant: Instant,
}

impl FpsManager {
    pub fn new(target_fps: f64) -> Self {
        let now = Instant::now();

        Self {
            frame_count: 0,
            rate_s: 1. / target_fps,
            base_instant: now,
            last_instant: now,
        }
    }

    pub fn delay(&mut self) {
        self.frame_count += 1;

        let current_instant = Instant::now();
        self.last_instant = current_instant;

        let target_instant =
            self.base_instant + Duration::from_secs_f64(self.frame_count as f64 * self.rate_s);

        if current_instant <= target_instant {
            sleep((target_instant - current_instant) / 2);
        } else {
            self.frame_count = 0;
            self.base_instant = current_instant;
        }
    }
}
