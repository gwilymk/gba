use crate::display::Font;

use super::{DynamicSprite, PaletteVram, Size, SpriteVram};

struct LetterGroup {
    sprite: SpriteVram,
    // the width of the letter group
    width: u16,
    left: i16,
}

struct WorkingLetter {
    dynamic: DynamicSprite,
    // the x offset of the current letter with respect to the start of the current letter group
    x_position: i32,
    // where to render the letter from x_min to x_max
    x_offset: i32,
}

impl WorkingLetter {
    fn new(size: Size) -> Self {
        Self {
            dynamic: DynamicSprite::new(size),
            x_position: 0,
            x_offset: 0,
        }
    }

    fn reset(&mut self) {
        self.x_position = 0;
        self.x_offset = 0;
    }
}

pub struct Configuration {
    sprite_size: Size,
    palette: PaletteVram,
}

impl Configuration {
    #[must_use]
    pub fn new(sprite_size: Size, palette: PaletteVram) -> Self {
        Self {
            sprite_size,
            palette,
        }
    }
}

struct WordRender<'font> {
    font: &'font Font,
    working: WorkingLetter,
    config: Configuration,
}

impl<'font> WordRender<'font> {
    #[must_use]
    fn new(font: &'font Font, config: Configuration) -> Self {
        WordRender {
            font,
            working: WorkingLetter::new(config.sprite_size),

            config,
        }
    }
}

impl WordRender<'_> {
    #[must_use]
    fn finalise_letter(&mut self) -> Option<LetterGroup> {
        if self.working.x_offset == 0 {
            return None;
        }

        let mut new_sprite = DynamicSprite::new(self.config.sprite_size);
        core::mem::swap(&mut self.working.dynamic, &mut new_sprite);
        let sprite = new_sprite.to_vram(self.config.palette.clone());

        let group = LetterGroup {
            sprite,
            width: self.working.x_offset as u16,
            left: self.working.x_position as i16,
        };
        self.working.reset();

        Some(group)
    }

    #[must_use]
    fn render_char(&mut self, c: char) -> Option<LetterGroup> {
        let font_letter = self.font.letter(c);

        // uses more than the sprite can hold
        let group = if self.working.x_offset + font_letter.width as i32
            > self.config.sprite_size.to_width_height().0 as i32
        {
            self.finalise_letter()
        } else {
            None
        };

        if self.working.x_offset == 0 {
            self.working.x_position = font_letter.xmin as i32;
        } else {
            self.working.x_offset += font_letter.xmin as i32;
        }

        let y_position = self.font.ascent() - font_letter.height as i32 - font_letter.ymin as i32;

        for y in 0..font_letter.height as usize {
            for x in 0..font_letter.width as usize {
                let rendered = font_letter.bit_absolute(x, y);
                if rendered {
                    self.working.dynamic.set_pixel(
                        x + self.working.x_offset as usize,
                        (y_position + y as i32) as usize,
                        1,
                    );
                }
            }
        }

        self.working.x_offset += font_letter.advance_width as i32;

        group
    }
}
