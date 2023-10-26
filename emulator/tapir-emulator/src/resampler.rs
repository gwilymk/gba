use std::collections::VecDeque;

pub trait Resampler {
    fn set_input_frequency(&mut self, frequency: f64);

    fn len(&self) -> usize;
    fn write_sample(&mut self, sample: f64);
    fn read_sample(&mut self) -> f64;
}

pub struct CubicResampler {
    buffered_samples: VecDeque<f64>,
    history: [f64; 4],
    fraction: f64,
    input_frequency: f64,
    output_frequency: f64,
    frequency_ratio: f64,
}

impl CubicResampler {
    pub fn new(output_frequency: f64, input_frequency: f64) -> Self {
        Self {
            buffered_samples: Default::default(),
            history: Default::default(),
            fraction: 0.,
            input_frequency,
            output_frequency,
            frequency_ratio: input_frequency / output_frequency,
        }
    }
}

impl Resampler for CubicResampler {
    fn set_input_frequency(&mut self, frequency: f64) {
        self.input_frequency = frequency;
        self.frequency_ratio = self.input_frequency / self.output_frequency;
    }

    fn len(&self) -> usize {
        self.buffered_samples.len()
    }

    fn write_sample(&mut self, sample: f64) {
        self.history[0] = self.history[1];
        self.history[1] = self.history[2];
        self.history[2] = self.history[3];
        self.history[3] = sample;

        let [s0, s1, s2, s3] = self.history;

        while self.fraction <= 1. {
            let t = self.fraction;

            let a = s3 - s2 - s0 + s1;
            let b = s0 - s1 - a;
            let c = s2 - s0;
            let d = s1;

            self.buffered_samples
                .push_back(a * t * t * t + b * t * t + c * t + d);

            self.fraction += self.frequency_ratio;
        }

        self.fraction -= 1.;
    }

    fn read_sample(&mut self) -> f64 {
        self.buffered_samples
            .pop_front()
            .expect("should have a sample to pop if that's what you requested")
    }
}
