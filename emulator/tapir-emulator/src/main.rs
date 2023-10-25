use std::{env, fs};

use anyhow::Context;
use sdl2::{
    audio::{AudioQueue, AudioSpecDesired},
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
};

use gba_audio::DynamicSampleRate;

mod gba_audio;

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

    let audio_queue: AudioQueue<i16> = audio_subsystem
        .open_queue(
            None,
            &AudioSpecDesired {
                freq: None,
                channels: Some(2),
                samples: None,
            },
        )
        .map_err(|e| anyhow::anyhow!("Failed to open audio queue {e}"))?;

    let mut dynamic_sample_rate = DynamicSampleRate::new(audio_queue.spec().freq as f64);
    let mut audio_buffer = vec![0; dynamic_sample_rate.samples_per_frame_estimate()];

    audio_queue.resume();

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

        mgba_core.set_audio_frequency(dynamic_sample_rate.frequency_estimate());
        audio_buffer.resize(dynamic_sample_rate.samples_per_frame_estimate(), 0);
        let audio_amount = mgba_core.read_audio(&mut audio_buffer);

        audio_queue
            .queue_audio(&audio_buffer)
            .map_err(|e| anyhow::anyhow!("Failed to enqueue audio {e}"))?;

        dynamic_sample_rate.update_audio_samples_per_frame(audio_amount);

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
        Scancode::Left => mgba::KeyMap::Left,
        Scancode::Right => mgba::KeyMap::Right,
        Scancode::Up => mgba::KeyMap::Up,
        Scancode::Down => mgba::KeyMap::Down,
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

    let default = "../../target/hyperspace-roll.gba".to_owned();
    let filename = args.get(1).unwrap_or(&default); //.ok_or("Expected 1 argument".to_owned())?;
    let content =
        fs::read(filename).with_context(|| format!("Failed to open ROM file {filename}"))?;

    Ok(content)
}
