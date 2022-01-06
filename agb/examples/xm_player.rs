#![no_std]
#![no_main]

use agb::include_xm;
use agb::Gba;

use agb::sound::mixer::tracker::{TrackerMusic, TrackerState};

// Music - "Ninja on speed" by algar
const NINJA_ON_SPEED: TrackerMusic = include_xm!("examples/algar_-_ninja_on_speed.xm");

#[agb::entry]
fn main() -> ! {
    let mut gba = Gba::new();
    let vblank_provider = agb::interrupt::VBlank::get();

    let mut timer_controller = gba.timers.timers();
    let mut timer = timer_controller.timer1;
    timer.set_enabled(true);

    let mut mixer = gba.mixer.mixer(&mut timer_controller.timer0);
    mixer.enable();

    let mut xm_state = TrackerState::new(&NINJA_ON_SPEED);

    let mut frame_counter = 0i32;
    loop {
        vblank_provider.wait_for_vblank();
        let before_mixing_cycles = timer.get_value();
        mixer.after_vblank();
        xm_state.update(&mut mixer);
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
