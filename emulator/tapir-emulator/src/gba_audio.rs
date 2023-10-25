pub struct DynamicSampleRate {
    sample_rate: f64,
}

impl DynamicSampleRate {
    pub fn new(starting_sample_rate: f64) -> Self {
        Self {
            sample_rate: starting_sample_rate,
        }
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn update_sample_rate(&mut self, guess: f64) {
        const MOVING_AVG_ALPHA: f64 = 1.0 / 180.0;
        self.sample_rate =
            (MOVING_AVG_ALPHA * guess as f64) + (1.0 - MOVING_AVG_ALPHA) * self.sample_rate;
    }
}
