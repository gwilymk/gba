use std::{env, fs, time::Instant};

use anyhow::Context;
use audio_stream::SdlAudioStream;
use gba_audio::DynamicSampleRate;
use resampler::CubicResampler;
use sdl2::{
    audio::{AudioQueue, AudioSpecDesired},
    event::Event,
    keyboard::{Keycode, Scancode},
    pixels::PixelFormatEnum,
};

use crate::resampler::Resampler;

mod resampler;

const GBA_FRAMES_PER_SECOND: f64 = 59.727500569606;

mod audio_stream;
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

    let audio_sample_rate = audio_queue.spec().freq as f64;
    let audio_buffer_size = audio_queue.spec().samples;

    let mut resamplers = [
        CubicResampler::new(audio_sample_rate, audio_sample_rate),
        CubicResampler::new(audio_sample_rate, audio_sample_rate),
    ];

    let mut audio_buffer = vec![];

    mgba_core.set_audio_frequency(audio_sample_rate);
    let mut dynamic_sample_rate = DynamicSampleRate::new(audio_sample_rate);

    audio_queue.resume();

    let mut keys = 0;

    let mut prev_frame = None;

    'running: loop {
        let now = Instant::now();
        let frame_time = prev_frame.map(|prev_frame| now - prev_frame);
        prev_frame = Some(now);

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

        let max_buffer_size = audio_buffer_size as f64;

        let measured_buffer_size = audio_queue.size() as f64 / 8.;

        let ratio =
            max_buffer_size / ((1. + 0.005) * max_buffer_size - 2. * 0.005 * measured_buffer_size);

        let rate = audio_sample_rate * ratio;

        dbg!(max_buffer_size);
        dbg!(measured_buffer_size);
        dbg!(ratio);
        dbg!(rate);

        let rate = rate.clamp(2., 100000.);

        for resampler in resamplers.iter_mut() {
            resampler.set_input_frequency(rate);
        }

        for sample in audio_buffer.chunks_exact(2) {
            let sample_l = sample[0];
            let sample_r = sample[1];
            resamplers[0].write_sample(sample_l as f64);
            resamplers[1].write_sample(sample_r as f64);
        }

        audio_buffer.clear();
        while resamplers[0].len() != 0 {
            audio_buffer.push(resamplers[0].read_sample() as i16);
            audio_buffer.push(resamplers[1].read_sample() as i16);
        }

        audio_queue
            .queue_audio(&audio_buffer)
            .map_err(|e| anyhow::anyhow!("Failed to enqueue audio {e}"))?;

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

    let default = concat!(env!("CARGO_TARGET_DIR"), "/hyperspace-roll.gba").to_owned();
    let filename = args.get(1).unwrap_or(&default); //.ok_or("Expected 1 argument".to_owned())?;
    let content =
        fs::read(filename).with_context(|| format!("Failed to open ROM file {filename}"))?;

    Ok(content)
}
