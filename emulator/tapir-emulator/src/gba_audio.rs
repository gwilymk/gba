pub struct DynamicSampleRate {
    audio_samples_per_frame_avg: f64,
}

impl DynamicSampleRate {
    pub fn new(starting_sample_rate: f64) -> Self {
        Self {
            audio_samples_per_frame_avg: starting_sample_rate / 60.0,
        }
    }

    pub fn samples_per_frame_estimate(&self) -> usize {
        self.audio_samples_per_frame_avg as usize * 2
    }

    pub fn frequency_estimate(&self) -> f64 {
        self.audio_samples_per_frame_avg * 60.0
    }

    pub fn update_audio_samples_per_frame(&mut self, samples: usize) {
        const MOVING_AVG_ALPHA: f64 = 1.0 / 180.0;
        self.audio_samples_per_frame_avg = (MOVING_AVG_ALPHA * samples as f64)
            + (1.0 - MOVING_AVG_ALPHA) * self.audio_samples_per_frame_avg;
    }
}
