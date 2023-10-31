use std::{env, fs};

use anyhow::Context;
use resampler::{CubicResampler, SharedAudioQueue};
use sdl2::{
    audio::AudioSpecDesired,
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
};

use crate::resampler::{calculate_dynamic_rate_ratio, Resampler};

mod resampler;

const GBA_FRAMES_PER_SECOND: f64 = 59.727500569606;

fn main() -> anyhow::Result<()> {
    let sdl_context = sdl2::init().unwrap();

    let rom_data = load_rom()?;
    let mut mgba_core =
        mgba::MCore::new().ok_or_else(|| anyhow::anyhow!("Failed to initialise mgba core"))?;
    mgba_core.load_rom(mgba::MemoryBacked::new(rom_data));

    let (width, height) = (240, 160);

    let video_subsystem = sdl_context
        .video()
        .map_err(|e| anyhow::anyhow!("Failed to initialise video subsystem {e}"))?;
    let audio_subsystem = sdl_context
        .audio()
        .map_err(|e| anyhow::anyhow!("Failed to initialise audio subsystem {e}"))?;

    let window = video_subsystem
        .window("Tapir emulator", width * 3, height * 3)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().present_vsync().build()?;

    let texture_creator = canvas.texture_creator();
    let mut texture =
        texture_creator.create_texture_streaming(PixelFormatEnum::ABGR8888, width, height)?;

    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!("Failed to initialise event pump {e}"))?;

    let audio_queue = SharedAudioQueue::default();

    let audio_system = audio_subsystem
        .open_playback(
            None,
            &AudioSpecDesired {
                freq: None,
                channels: Some(2),
                samples: None,
            },
            |s| {
                let queue = audio_queue.clone();
                queue.set_sample_rate(s.freq as usize);

                queue
            },
        )
        .expect("should be able to initialise audio");

    let audio_sample_rate = audio_queue.sample_rate() as f64;

    let mut resamplers = [
        CubicResampler::new(audio_sample_rate, audio_sample_rate),
        CubicResampler::new(audio_sample_rate, audio_sample_rate),
    ];

    audio_system.resume();

    let mut audio_buffer = vec![];
    let audio_sample_rate = audio_system.spec().freq as f64;

    mgba_core.set_audio_frequency(audio_sample_rate);

    let mut keys = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(gba_keycode) = to_gba_keycode(scancode) {
                        keys |= 1 << gba_keycode as usize;
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(gba_keycode) = to_gba_keycode(scancode) {
                        keys &= !(1 << gba_keycode as usize);
                    }
                }
                _ => {}
            }
        }

        mgba_core.set_keys(keys);
        mgba_core.frame();

        mgba_core.read_audio(&mut audio_buffer);

        {
            let queue_length = audio_queue.samples();
            let ratio = calculate_dynamic_rate_ratio(800, queue_length, 0.005);

            let rate = audio_sample_rate * ratio;

            dbg!(ratio);
            dbg!(rate);
            dbg!(queue_length);

            for resampler in resamplers.iter_mut() {
                resampler.set_input_frequency(rate);
            }

            for sample in audio_buffer.chunks_exact(2) {
                let sample_l = sample[0];
                let sample_r = sample[1];
                resamplers[0].write_sample(sample_l as f64);
                resamplers[1].write_sample(sample_r as f64);
            }

            while let (Some(a), Some(b)) =
                (resamplers[0].read_sample(), resamplers[1].read_sample())
            {
                audio_queue.push([a as i16, b as i16]);
            }

            audio_buffer.clear();
        }

        texture
            .with_lock(None, |buffer, _pitch| {
                let mgba_buffer = mgba_core.video_buffer();
                for (i, data) in mgba_buffer.iter().enumerate() {
                    buffer[(i * 4)..((i + 1) * 4)].copy_from_slice(&data.to_ne_bytes());
                }
            })
            .map_err(|e| anyhow::anyhow!("Failed to copy mgba texture {e}"))?;

        canvas
            .copy(&texture, None, None)
            .map_err(|e| anyhow::anyhow!("Failed to copy texture {e}"))?;
        canvas.present();
    }

    Ok(())
}

fn to_gba_keycode(keycode: Scancode) -> Option<mgba::KeyMap> {
    Some(match keycode {
        Scancode::Left | Scancode::J => mgba::KeyMap::Left,
        Scancode::Right | Scancode::L => mgba::KeyMap::Right,
        Scancode::Up | Scancode::I => mgba::KeyMap::Up,
        Scancode::Down | Scancode::K => mgba::KeyMap::Down,
        Scancode::Z => mgba::KeyMap::B,
        Scancode::X => mgba::KeyMap::A,
        Scancode::Return => mgba::KeyMap::Start,
        Scancode::Backspace => mgba::KeyMap::Select,
        Scancode::A => mgba::KeyMap::L,
        Scancode::S => mgba::KeyMap::R,
        _ => return None,
    })
}

fn load_rom() -> anyhow::Result<Vec<u8>> {
    let args: Vec<String> = env::args().collect();

    let default = concat!(env!("CARGO_TARGET_DIR"), "/combo.gba").to_owned();
    let filename = args.get(1).unwrap_or(&default); //.ok_or("Expected 1 argument".to_owned())?;
    let content =
        fs::read(filename).with_context(|| format!("Failed to open ROM file {filename}"))?;

    Ok(content)
}
