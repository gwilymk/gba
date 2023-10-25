use std::mem::size_of_val;

use sdl2::{
    audio::AudioFormat,
    sys::{
        SDL_AudioStream, SDL_AudioStreamAvailable, SDL_AudioStreamFlush, SDL_AudioStreamGet,
        SDL_AudioStreamPut, SDL_FreeAudioStream, SDL_NewAudioStream,
    },
};

pub struct SdlAudioStream {
    inner: *mut SDL_AudioStream,
}

impl SdlAudioStream {
    pub fn new(src_rate: i32, dest_rate: i32) -> Self {
        let inner = unsafe {
            SDL_NewAudioStream(
                AudioFormat::S16LSB as u16,
                2,
                src_rate,
                AudioFormat::S16LSB as u16,
                2,
                dest_rate,
            )
        };

        Self { inner }
    }

    pub fn put(&mut self, buffer: &[i16]) {
        unsafe {
            SDL_AudioStreamPut(
                self.inner,
                buffer.as_ptr().cast(),
                size_of_val(buffer) as i32,
            );
        }
    }

    pub fn get(&mut self, buffer: &mut Vec<i16>) -> Result<i32, ()> {
        let available = self.available();
        buffer.resize(available / 2, 0);

        let amount_read =
            unsafe { SDL_AudioStreamGet(self.inner, buffer.as_mut_ptr().cast(), available as i32) };

        if amount_read < 0 {
            Err(())
        } else {
            Ok(amount_read)
        }
    }

    fn available(&self) -> usize {
        (unsafe { SDL_AudioStreamAvailable(self.inner) }) as usize
    }

    pub fn flush(&mut self) {
        unsafe { SDL_AudioStreamFlush(self.inner) };
    }
}

impl Drop for SdlAudioStream {
    fn drop(&mut self) {
        unsafe {
            SDL_FreeAudioStream(self.inner);
        }
    }
}
