use super::{ChannelId, Mixer, SoundChannel};
use crate::number::Num;

pub struct TrackerMusic {
    samples: &'static [Sample],
    patterns: &'static [&'static [Note]],
    num_channels: u8,

    initial_speed: u8,
}

pub struct TrackerState {
    current_pattern: u8,
    current_pattern_pos: u16,

    current_speed: u8,
    frame: u8,

    current_playing: [Option<ChannelId>; 8],

    tracker_music: &'static TrackerMusic,
}

impl TrackerState {
    pub fn new(tracker_music: &'static TrackerMusic) -> Self {
        Self {
            current_pattern: 0,
            current_pattern_pos: 0,

            current_speed: tracker_music.initial_speed,
            frame: 0,

            current_playing: [None; 8],

            tracker_music,
        }
    }

    pub fn update(&mut self, mixer: &mut Mixer) -> bool {
        self.frame += 1;
        if self.frame != self.current_speed {
            return false; // leave everything playing as it was
        }

        self.frame = 0;

        let notes = self.get_to_play();
        self.current_pattern_pos += self.tracker_music.num_channels as u16;

        for (i, note) in notes.iter().enumerate() {
            if note.sample == 0 && note.playback_speed == 0 {
                continue;
            }

            let current_channel = self.current_playing[i]
                .map(|channel_id| mixer.get_channel(&channel_id))
                .flatten();
            if let Some(current_channel) = current_channel {
                if note.playback_speed == 0 || note.sample != 0 {
                    current_channel.stop();
                } else if note.sample == 0 {
                    current_channel.playback(Num::from_raw(note.playback_speed as usize));
                }
            }

            if note.sample != 0 && note.playback_speed != 0 {
                let tracker_sample = &self.tracker_music.samples[(note.sample - 1) as usize];
                let mut channel = SoundChannel::new_high_priority(tracker_sample.data);

                channel.playback(Num::from_raw(note.playback_speed as usize));

                if tracker_sample.should_loop {
                    channel.should_loop();
                }

                self.current_playing[i] = mixer.play_sound(channel);
            }
        }

        false
    }

    fn get_to_play(&mut self) -> &'static [Note] {
        loop {
            let pattern = self.tracker_music.patterns[self.current_pattern as usize];
            if self.current_pattern_pos as usize >= pattern.len() {
                self.current_pattern += 1;
                self.current_pattern_pos = 0;
            } else {
                return &pattern[(self.current_pattern_pos as usize)
                    ..((self.current_pattern_pos + self.tracker_music.num_channels as u16)
                        as usize)];
            }
        }
    }
}

#[doc(hidden)]
impl TrackerMusic {
    pub const fn new(
        samples: &'static [Sample],
        patterns: &'static [&'static [Note]],
        num_channels: u8,
        initial_speed: u8,
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
    playback_speed: u16,
}

impl Note {
    pub const fn new(sample: u8, playback_speed: u16) -> Self {
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
