use super::Mixer;
use crate::number::Num;

pub struct TrackerMusic {
    samples: &'static [Sample],
    patterns: &'static [&'static [Note]],
    num_channels: u8,

    initial_speed: usize,
}

pub struct TrackerState {
    current_pattern: u8,
    current_pattern_pos: u8,

    current_speed: usize,

    tracker_music: &'static TrackerMusic,
}

impl TrackerState {
    pub fn new(tracker_music: &'static TrackerMusic) -> Self {
        Self {
            current_pattern: 0,
            current_pattern_pos: 0,

            current_speed: tracker_music.initial_speed,

            tracker_music,
        }
    }

    pub fn update(&mut self, mixer: &mut Mixer) -> bool {
        false
    }
}

#[doc(hidden)]
impl TrackerMusic {
    pub const fn new(
        samples: &'static [Sample],
        patterns: &'static [&'static [Note]],
        num_channels: u8,
        initial_speed: usize,
    ) -> Self {
        Self {
            samples,
            patterns,
            num_channels,
            initial_speed,
        }
    }
}

#[doc(hidden)]
pub struct Note {
    sample: u8,
    playback_speed: u8,
}

impl Note {
    pub const fn new(sample: u8, playback_speed: u8) -> Self {
        Self {
            sample,
            playback_speed,
        }
    }
}

#[doc(hidden)]
pub struct Sample {
    data: &'static [u8],
    should_loop: bool,
}

impl Sample {
    pub const fn new(data: &'static [u8], should_loop: bool) -> Self {
        Self { data, should_loop }
    }
}
