#![no_std]
#![no_main]

use agb::input::{Button, ButtonController, Tri};
use agb::number::Num;
use agb::sound::mixer::SoundChannel;
use agb::{include_wav, Gba};

// Music - "Dead Code" by Josh Woodward, free download at http://joshwoodward.com
const DEAD_CODE: &[u8] = include_wav!("examples/JoshWoodward-DeadCode.wav");

#[agb::entry]
fn main() -> ! {
    let mut gba = Gba::new();
    let mut input = ButtonController::new();
    let vblank_provider = agb::interrupt::VBlank::get();

    let mut timers = gba.timers.timers();
    let mut mixer = gba.mixer.mixer(&mut timers.timer0);
    mixer.enable();

    let mut timer = timers.timer1;
    timer.set_enabled(true);

    let channel = SoundChannel::new(DEAD_CODE);
    let channel_id = mixer.play_sound(channel).unwrap();

    let mut frame_counter = 0i32;
    loop {
        input.update();

        {
            if let Some(channel) = mixer.get_channel(&channel_id) {
                let half: Num<i16, 4> = Num::new(1) / 2;
                let half_usize: Num<usize, 8> = Num::new(1) / 2;
                match input.x_tri() {
                    Tri::Negative => channel.panning(-half),
                    Tri::Zero => channel.panning(0.into()),
                    Tri::Positive => channel.panning(half),
                };

                match input.y_tri() {
                    Tri::Negative => channel.playback(half_usize.change_base() + 1),
                    Tri::Zero => channel.playback(1.into()),
                    Tri::Positive => channel.playback(half_usize),
                };

                if input.is_pressed(Button::L) {
                    channel.volume(half);
                } else {
                    channel.volume(1.into());
                }
            }
        }

        vblank_provider.wait_for_vblank();
        let before_mixing_cycles = timer.get_value();
        mixer.after_vblank();
        mixer.frame();
        let after_mixing_cycles = timer.get_value();

        frame_counter = frame_counter.wrapping_add(1);

        if frame_counter % 128 == 0 {
            let total_cycles = after_mixing_cycles.wrapping_sub(before_mixing_cycles) as u32;

            let percent = (total_cycles * 100) / 280896;
            agb::println!(
                "Took {} cycles to calculate mixer ~= {}% of total frame",
                total_cycles,
                percent
            );
        }
    }
}
