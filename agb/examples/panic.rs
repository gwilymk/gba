#![no_std]
#![no_main]

use agb::display;

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut bitmap = gba.display.video.bitmap3();
    let mut input = agb::input::ButtonController::new();

    loop {
        input.update();
        // if A is pressed, draw out of range
        if input.is_just_pressed(agb::input::Button::A) {
            bitmap.draw_point(display::WIDTH, 0, 0x05);
        }
        if input.is_just_pressed(agb::input::Button::B) {
            #[allow(arithmetic_overflow)]
            let _p = i32::MAX + 1;
        }
    }
}
