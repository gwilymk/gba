#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]
#![feature(allocator_api)]

extern crate alloc;

use agb::{
    display::{
        object::{Graphics, ObjectController, Tag, TagMap},
        tiled::{
            InfiniteScrolledMap, PartialUpdateStatus, RegularBackgroundSize, TileFormat, TileSet,
            TileSetting, TiledMap, VRamManager,
        },
        Priority, HEIGHT, WIDTH,
    },
    fixnum::{FixedNum, Vector2D},
    input::{self, Button, ButtonController},
    sound::mixer::Frequency,
};
use alloc::{boxed::Box, vec::Vec};
use enemies::BoxedEnemy;

mod enemies;
mod level_display;
mod sfx;
mod splash_screen;

mod entity;

use entity::Entity;

const GRAVITY: Vector2D<FixedNumberType> = Vector2D::new(
    FixedNumberType::from_raw(0),
    FixedNumberType::from_raw((1 << 10) / 16),
);

pub struct Level {
    background: &'static [u16],
    foreground: &'static [u16],
    dimensions: Vector2D<u32>,
    collision: &'static [u32],

    enemies: &'static [((i32, i32), enemies::EnemyType)],
    enemy_stops: &'static [(i32, i32)],
    start_pos: (i32, i32),
}

mod map_tiles {
    use super::Level;
    pub const LEVELS: &[Level] = &[
        l1_1::get_level(),
        l1_2::get_level(),
        l1_3::get_level(),
        l1_4::get_level(),
        l1_5::get_level(),
        l1_7::get_level(), // these are intentionally this way round
        l1_6::get_level(),
        l1_8::get_level(),
        l2_3::get_level(), // goes 2-3, 2-1 then 2-2
        l2_1::get_level(),
        l2_2::get_level(),
        l2_4::get_level(),
    ];

    macro_rules! include_out_dir {
        ($filename:expr) => {
            include!(concat!(env!("OUT_DIR"), "/", $filename));
        };
    }

    macro_rules! include_level {
        ($mod_name:ident, $filename:tt) => {
            pub mod $mod_name {
                include_out_dir!(concat!($filename, ".tmx.rs"));
            }
        };
    }

    include_level!(l1_1, "1-1");
    include_level!(l1_2, "1-2");
    include_level!(l1_3, "1-3");
    include_level!(l1_4, "1-4");
    include_level!(l1_5, "1-5");
    include_level!(l1_6, "1-6");
    include_level!(l1_7, "1-7");
    include_level!(l1_8, "1-8");
    include_level!(l2_1, "2-1");
    include_level!(l2_2, "2-2");
    include_level!(l2_3, "2-3");
    include_level!(l2_4, "2-4");

    pub mod tilemap {
        include_out_dir!("tilemap.rs");
    }
}

agb::include_gfx!("gfx/tile_sheet.toml");

const GRAPHICS: &Graphics = agb::include_aseprite!("gfx/sprites.aseprite");
const TAG_MAP: &TagMap = GRAPHICS.tags();

const WALKING: &Tag = TAG_MAP.get("Walking");
const JUMPING: &Tag = TAG_MAP.get("Jumping");
const FALLING: &Tag = TAG_MAP.get("Falling");
const PLAYER_DEATH: &Tag = TAG_MAP.get("Player Death");
const HAT_SPIN_1: &Tag = TAG_MAP.get("HatSpin");
const HAT_SPIN_2: &Tag = TAG_MAP.get("HatSpin2");
const HAT_SPIN_3: &Tag = TAG_MAP.get("HatSpin3");

type FixedNumberType = FixedNum<10>;

struct Map<'a, 'b> {
    background: &'a mut InfiniteScrolledMap<'b>,
    foreground: &'a mut InfiniteScrolledMap<'b>,
    position: Vector2D<FixedNumberType>,
    level: &'a Level,
}

impl<'a, 'b> Map<'a, 'b> {
    pub fn commit_position(&mut self, vram: &mut VRamManager) {
        self.background.set_pos(vram, self.position.floor());
        self.foreground.set_pos(vram, self.position.floor());

        self.background.commit(vram);
        self.foreground.commit(vram);
    }

    pub fn init_background(&mut self, vram: &mut VRamManager) -> PartialUpdateStatus {
        self.background.init_partial(vram, self.position.floor())
    }

    pub fn init_foreground(&mut self, vram: &mut VRamManager) -> PartialUpdateStatus {
        self.foreground.init_partial(vram, self.position.floor())
    }
}

impl Level {
    fn collides(&self, x: i32, y: i32) -> bool {
        self.at_point(x, y, map_tiles::tilemap::COLLISION_TILE as u32)
    }

    fn kills(&self, x: i32, y: i32) -> bool {
        self.at_point(x, y, map_tiles::tilemap::KILL_TILE as u32)
    }

    fn at_point(&self, x: i32, y: i32, tile: u32) -> bool {
        if (x < 0 || x >= self.dimensions.x as i32) || (y < 0 || y >= self.dimensions.y as i32) {
            return true;
        }
        let pos = (self.dimensions.x as i32 * y + x) as usize;
        let tile_foreground = self.foreground[pos];
        let tile_background = self.background[pos];
        let foreground_tile_property = self.collision[tile_foreground as usize];
        let background_tile_property = self.collision[tile_background as usize];
        foreground_tile_property == tile || background_tile_property == tile
    }

    fn wins(&self, x: i32, y: i32) -> bool {
        self.at_point(x, y, map_tiles::tilemap::WIN_TILE as u32)
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum HatState {
    OnHead,
    Thrown,
    WizardTowards,
}

struct Player<'a> {
    wizard: Entity<'a>,
    hat: Entity<'a>,
    hat_state: HatState,
    hat_left_range: bool,
    hat_slow_counter: i32,
    wizard_frame: u8,
    num_recalls: i8,
    is_on_ground: bool,
    facing: input::Tri,
}

fn ping_pong(i: i32, n: i32) -> i32 {
    let cycle = 2 * (n - 1);
    let i = i % cycle;
    if i >= n {
        cycle - i
    } else {
        i
    }
}

impl<'a> Player<'a> {
    fn new(controller: &'a ObjectController, start_position: Vector2D<FixedNumberType>) -> Self {
        let mut wizard = Entity::new(controller, (6_u16, 14_u16).into());
        let mut hat = Entity::new(controller, (6_u16, 6_u16).into());

        wizard
            .sprite
            .set_sprite(controller.sprite(HAT_SPIN_1.sprite(0)));
        hat.sprite
            .set_sprite(controller.sprite(HAT_SPIN_1.sprite(0)));

        wizard.sprite.show();
        hat.sprite.show();

        hat.sprite.set_z(-1);

        wizard.position = start_position;
        hat.position = start_position - (0, 10).into();

        Player {
            wizard,
            hat,
            hat_slow_counter: 0,
            hat_state: HatState::OnHead,
            hat_left_range: false,
            wizard_frame: 0,
            num_recalls: 0,
            is_on_ground: true,
            facing: input::Tri::Zero,
        }
    }

    fn update_frame(
        &mut self,
        input: &ButtonController,
        controller: &'a ObjectController,
        timer: i32,
        level: &Level,
        enemies: &[enemies::BoxedEnemy],
        sfx_player: &mut sfx::SfxPlayer,
    ) {
        // throw or recall
        if input.is_just_pressed(Button::A) {
            if self.hat_state == HatState::OnHead {
                let direction: Vector2D<FixedNumberType> = {
                    let up_down = input.y_tri() as i32;
                    let left_right = if up_down == 0 {
                        self.facing as i32
                    } else {
                        input.x_tri() as i32
                    };
                    (left_right, up_down).into()
                };

                if direction != (0, 0).into() {
                    let mut velocity = direction.normalise() * 5;
                    if velocity.y > 0.into() {
                        velocity.y *= FixedNumberType::new(4) / 3;
                    }
                    self.hat.velocity = velocity;
                    self.hat_state = HatState::Thrown;

                    sfx_player.throw();
                }
            } else if self.hat_state == HatState::Thrown {
                self.num_recalls += 1;
                if self.num_recalls < 3 {
                    self.hat.velocity = (0, 0).into();
                    self.wizard.velocity = (0, 0).into();
                    self.hat_state = HatState::WizardTowards;
                }
            } else if self.hat_state == HatState::WizardTowards {
                self.hat_state = HatState::Thrown;
                self.wizard.velocity /= 8;
            }
        }

        let was_on_ground = self.is_on_ground;
        let is_on_ground = self.wizard.is_on_ground(level);

        if is_on_ground && !was_on_ground && self.wizard.velocity.y > 1.into() {
            sfx_player.land();
        }
        self.is_on_ground = is_on_ground;

        if self.hat_state != HatState::WizardTowards {
            if is_on_ground {
                self.num_recalls = 0;
            }

            if is_on_ground {
                self.wizard.velocity.x += FixedNumberType::new(input.x_tri() as i32) / 16;
                self.wizard.velocity = self.wizard.velocity * 54 / 64;
                if input.is_just_pressed(Button::B) {
                    self.wizard.velocity.y = -FixedNumberType::new(3) / 2;
                    sfx_player.jump();
                }
            } else {
                self.wizard.velocity.x += FixedNumberType::new(input.x_tri() as i32) / 64;
                self.wizard.velocity = self.wizard.velocity * 63 / 64;

                self.wizard.velocity += GRAVITY;
            }

            self.wizard.velocity = self.wizard.update_position(level);

            if self.wizard.velocity.x.abs() > 0.into() {
                let offset = (ping_pong(timer / 16, 4)) as usize;
                self.wizard_frame = offset as u8;

                let frame = WALKING.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.wizard.sprite.set_sprite(sprite);
            }

            if self.wizard.velocity.y < -FixedNumberType::new(1) / 16 {
                // going up
                self.wizard_frame = 5;

                let frame = JUMPING.animation_sprite(0);
                let sprite = controller.sprite(frame);

                self.wizard.sprite.set_sprite(sprite);
            } else if self.wizard.velocity.y > FixedNumberType::new(1) / 16 {
                // going down
                let offset = if self.wizard.velocity.y * 2 > 3.into() {
                    (timer / 4) as usize
                } else {
                    // Don't flap beard unless going quickly
                    0
                };

                self.wizard_frame = 0;

                let frame = FALLING.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.wizard.sprite.set_sprite(sprite);
            }

            if input.x_tri() != agb::input::Tri::Zero {
                self.facing = input.x_tri();
            }
        }

        let hat_base_tile = match self.num_recalls {
            0 => HAT_SPIN_1,
            1 => HAT_SPIN_2,
            _ => HAT_SPIN_3,
        };

        let hat_resting_position = match self.wizard_frame {
            1 | 2 => (0, 9).into(),
            5 => (0, 10).into(),
            _ => (0, 8).into(),
        };

        match self.facing {
            agb::input::Tri::Negative => {
                self.wizard.sprite.set_hflip(true);
                self.hat
                    .sprite
                    .set_sprite(controller.sprite(hat_base_tile.sprite(5)));
            }
            agb::input::Tri::Positive => {
                self.wizard.sprite.set_hflip(false);
                self.hat
                    .sprite
                    .set_sprite(controller.sprite(hat_base_tile.sprite(0)));
            }
            _ => {}
        }

        match self.hat_state {
            HatState::Thrown => {
                // hat is thrown, make hat move towards wizard
                let distance_vector =
                    self.wizard.position - self.hat.position - hat_resting_position;
                let distance = distance_vector.magnitude();
                let direction = if distance == 0.into() {
                    (0, 0).into()
                } else {
                    distance_vector / distance
                };

                let hat_sprite_divider = match self.num_recalls {
                    0 => 1,
                    1 => 2,
                    _ => 4,
                };

                let hat_sprite_offset = (timer / hat_sprite_divider) as usize;

                self.hat.sprite.set_sprite(
                    controller.sprite(hat_base_tile.animation_sprite(hat_sprite_offset)),
                );

                if self.hat_slow_counter < 30 && self.hat.velocity.magnitude() < 2.into() {
                    self.hat.velocity = (0, 0).into();
                    self.hat_slow_counter += 1;
                } else {
                    self.hat.velocity += direction / 4;
                }
                let (new_velocity, enemy_collision) =
                    self.hat.update_position_with_enemy(level, enemies);
                self.hat.velocity = new_velocity;

                if enemy_collision {
                    sfx_player.snail_hat_bounce();
                }

                if distance > 16.into() {
                    self.hat_left_range = true;
                }
                if self.hat_left_range && distance < 16.into() {
                    sfx_player.catch();
                    self.hat_state = HatState::OnHead;
                }
            }
            HatState::OnHead => {
                // hat is on head, place hat on head
                self.hat_slow_counter = 0;
                self.hat_left_range = false;
                self.hat.position = self.wizard.position - hat_resting_position;
            }
            HatState::WizardTowards => {
                self.hat.sprite.set_sprite(
                    controller.sprite(hat_base_tile.animation_sprite(timer as usize / 2)),
                );
                let distance_vector =
                    self.hat.position - self.wizard.position + hat_resting_position;
                let distance = distance_vector.magnitude();
                if distance != 0.into() {
                    let v = self.wizard.velocity.magnitude() + 1;
                    self.wizard.velocity = distance_vector / distance * v;
                }
                self.wizard.velocity = self.wizard.update_position(level);
                if distance < 16.into() {
                    self.wizard.velocity /= 8;
                    self.hat_state = HatState::OnHead;
                    sfx_player.catch();
                }
            }
        }
    }
}

struct PlayingLevel<'a, 'b> {
    timer: i32,
    background: Map<'a, 'b>,
    input: ButtonController,
    player: Player<'a>,

    enemies: Vec<BoxedEnemy<'a>>,
}

enum UpdateState {
    Normal,
    Dead,
    Complete,
}

impl<'a, 'b> PlayingLevel<'a, 'b> {
    fn open_level(
        level: &'a Level,
        object_control: &'a ObjectController,
        background: &'a mut InfiniteScrolledMap<'b>,
        foreground: &'a mut InfiniteScrolledMap<'b>,
        input: ButtonController,
    ) -> Self {
        let mut e = Vec::with_capacity(level.enemies.len());
        for (enemy_start, enemy_type) in level.enemies {
            e.push(enemies::new_enemy(
                *enemy_type,
                object_control,
                (*enemy_start).into(),
            ));
        }

        let start_pos: Vector2D<FixedNumberType> = level.start_pos.into();

        let background_position = (
            (start_pos.x - WIDTH / 2)
                .clamp(0.into(), ((level.dimensions.x * 8) as i32 - WIDTH).into()),
            (start_pos.y - HEIGHT / 2)
                .clamp(0.into(), ((level.dimensions.y * 8) as i32 - HEIGHT).into()),
        )
            .into();

        PlayingLevel {
            timer: 0,
            background: Map {
                background,
                foreground,
                level,
                position: background_position,
            },
            player: Player::new(object_control, start_pos),
            input,
            enemies: e,
        }
    }

    fn show_backgrounds(&mut self) {
        self.background.background.show();
        self.background.foreground.show();
    }

    fn hide_backgrounds(&mut self) {
        self.background.background.hide();
        self.background.foreground.hide();
    }

    fn clear_backgrounds(&mut self, vram: &mut VRamManager) {
        self.background.background.clear(vram);
        self.background.foreground.clear(vram);
    }

    fn dead_start(&mut self) {
        self.player.wizard.velocity = (0, -1).into();
        self.player.wizard.sprite.set_priority(Priority::P0);
    }

    fn dead_update(&mut self, controller: &'a ObjectController) -> bool {
        self.timer += 1;

        let frame = PLAYER_DEATH.animation_sprite(self.timer as usize / 8);
        let sprite = controller.sprite(frame);

        self.player.wizard.velocity += (0.into(), FixedNumberType::new(1) / 32).into();
        self.player.wizard.position += self.player.wizard.velocity;
        self.player.wizard.sprite.set_sprite(sprite);

        self.player.wizard.commit_position(self.background.position);

        self.player.wizard.position.y - self.background.position.y < (HEIGHT + 8).into()
    }

    fn update_frame(
        &mut self,
        sfx_player: &mut sfx::SfxPlayer,
        vram: &mut VRamManager,
        controller: &'a ObjectController,
    ) -> UpdateState {
        self.timer += 1;
        self.input.update();

        let previous_hat_state = self.player.hat_state;
        self.player.update_frame(
            &self.input,
            controller,
            self.timer,
            self.background.level,
            &self.enemies,
            sfx_player,
        );

        let hat_state_for_enemy = if self.player.hat_state == HatState::WizardTowards {
            HatState::WizardTowards
        } else {
            previous_hat_state
        };

        let mut player_dead = false;
        self.enemies.retain_mut(|enemy| {
            match enemy.update(
                controller,
                self.background.level,
                self.player.wizard.position,
                hat_state_for_enemy,
                self.timer,
                sfx_player,
            ) {
                enemies::EnemyUpdateState::KillPlayer => player_dead = true,
                enemies::EnemyUpdateState::None => {}
                enemies::EnemyUpdateState::Remove => return false,
            };
            true
        });

        self.background.position = self.get_next_map_position();
        self.background.commit_position(vram);

        self.player.wizard.commit_position(self.background.position);
        self.player.hat.commit_position(self.background.position);

        for enemy in self.enemies.iter_mut() {
            enemy.commit(self.background.position);
        }

        player_dead |= self
            .player
            .wizard
            .killision_at_point(self.background.level, self.player.wizard.position);
        if player_dead {
            UpdateState::Dead
        } else if self
            .player
            .wizard
            .completion_at_point(self.background.level, self.player.wizard.position)
        {
            UpdateState::Complete
        } else {
            UpdateState::Normal
        }
    }

    fn get_next_map_position(&self) -> Vector2D<FixedNumberType> {
        // want to ensure the player and the hat are visible if possible, so try to position the map
        // so the centre is at the average position. But give the player some extra priority
        let hat_pos = self.player.hat.position.floor();
        let player_pos = self.player.wizard.position.floor();

        let new_target_position = (hat_pos + player_pos * 3) / 4;

        let screen: Vector2D<i32> = (WIDTH, HEIGHT).into();
        let half_screen = screen / 2;
        let current_centre = self.background.position.floor() + half_screen;

        let mut target_position = ((current_centre * 3 + new_target_position) / 4) - half_screen;

        target_position.x = target_position.x.clamp(
            0,
            (self.background.level.dimensions.x * 8 - (WIDTH as u32)) as i32,
        );
        target_position.y = target_position.y.clamp(
            0,
            (self.background.level.dimensions.y * 8 - (HEIGHT as u32)) as i32,
        );

        target_position.into()
    }
}

pub fn main(mut agb: agb::Gba) -> ! {
    let (tiled, mut vram) = agb.display.video.tiled0();
    vram.set_background_palettes(tile_sheet::PALETTES);
    let mut splash_screen = tiled.background(Priority::P0, RegularBackgroundSize::Background32x32);
    let mut world_display = tiled.background(Priority::P0, RegularBackgroundSize::Background32x32);

    let tileset = TileSet::new(tile_sheet::background.tiles, TileFormat::FourBpp);

    for y in 0..32u16 {
        for x in 0..32u16 {
            world_display.set_tile(
                &mut vram,
                (x, y).into(),
                &tileset,
                TileSetting::from_raw(level_display::BLANK),
            );
        }
    }

    world_display.commit(&mut vram);
    world_display.show();

    splash_screen::show_splash_screen(
        splash_screen::SplashScreen::Start,
        None,
        None,
        &mut splash_screen,
        &mut vram,
    );

    loop {
        world_display.commit(&mut vram);
        world_display.show();

        vram.set_background_palettes(tile_sheet::PALETTES);

        let object = agb.display.object.get();
        let mut mixer = agb.mixer.mixer(Frequency::Hz32768);

        mixer.enable();
        let mut music_box = sfx::MusicBox::new();

        let vblank = agb::interrupt::VBlank::get();
        let mut current_level = 0;

        loop {
            if current_level == map_tiles::LEVELS.len() as u32 {
                break;
            }

            music_box.before_frame(&mut mixer);
            mixer.frame();
            vblank.wait_for_vblank();

            level_display::write_level(
                &mut world_display,
                current_level / 8 + 1,
                current_level % 8 + 1,
                &tileset,
                &mut vram,
            );

            world_display.commit(&mut vram);
            world_display.show();

            music_box.before_frame(&mut mixer);
            mixer.frame();
            vblank.wait_for_vblank();

            let map_current_level = current_level;
            let mut background = InfiniteScrolledMap::new(
                tiled.background(Priority::P2, RegularBackgroundSize::Background32x64),
                Box::new(|pos: Vector2D<i32>| {
                    let level = &map_tiles::LEVELS[map_current_level as usize];
                    (
                        &tileset,
                        TileSetting::from_raw(
                            *level
                                .background
                                .get((pos.y * level.dimensions.x as i32 + pos.x) as usize)
                                .unwrap_or(&0),
                        ),
                    )
                }),
            );
            let mut foreground = InfiniteScrolledMap::new(
                tiled.background(Priority::P0, RegularBackgroundSize::Background64x32),
                Box::new(|pos: Vector2D<i32>| {
                    let level = &map_tiles::LEVELS[map_current_level as usize];
                    (
                        &tileset,
                        TileSetting::from_raw(
                            *level
                                .foreground
                                .get((pos.y * level.dimensions.x as i32 + pos.x) as usize)
                                .unwrap_or(&0),
                        ),
                    )
                }),
            );

            let mut level = PlayingLevel::open_level(
                &map_tiles::LEVELS[current_level as usize],
                &object,
                &mut background,
                &mut foreground,
                agb::input::ButtonController::new(),
            );

            while level.background.init_background(&mut vram) != PartialUpdateStatus::Done {
                music_box.before_frame(&mut mixer);
                mixer.frame();
                vblank.wait_for_vblank();
            }

            while level.background.init_foreground(&mut vram) != PartialUpdateStatus::Done {
                music_box.before_frame(&mut mixer);
                mixer.frame();
                vblank.wait_for_vblank();
            }

            for _ in 0..20 {
                music_box.before_frame(&mut mixer);
                mixer.frame();
                vblank.wait_for_vblank();
            }

            object.commit();

            level.show_backgrounds();

            world_display.hide();

            loop {
                match level.update_frame(
                    &mut sfx::SfxPlayer::new(&mut mixer, &music_box),
                    &mut vram,
                    &object,
                ) {
                    UpdateState::Normal => {}
                    UpdateState::Dead => {
                        level.dead_start();
                        while level.dead_update(&object) {
                            music_box.before_frame(&mut mixer);
                            mixer.frame();
                            vblank.wait_for_vblank();
                            object.commit();
                        }
                        break;
                    }
                    UpdateState::Complete => {
                        current_level += 1;
                        break;
                    }
                }

                music_box.before_frame(&mut mixer);
                mixer.frame();
                vblank.wait_for_vblank();
                object.commit();
            }

            level.hide_backgrounds();
            level.clear_backgrounds(&mut vram);
        }

        object.commit();

        splash_screen::show_splash_screen(
            splash_screen::SplashScreen::End,
            Some(&mut mixer),
            Some(&mut music_box),
            &mut splash_screen,
            &mut vram,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agb::Gba;

    #[test_case]
    fn test_ping_pong(_gba: &mut Gba) {
        let test_cases = [
            [0, 2, 0],
            [0, 7, 0],
            [1, 2, 1],
            [2, 2, 0],
            [3, 2, 1],
            [4, 2, 0],
        ];

        for test_case in test_cases {
            assert_eq!(
                ping_pong(test_case[0], test_case[1]),
                test_case[2],
                "Expected ping_pong({}, {}) to equal {}",
                test_case[0],
                test_case[1],
                test_case[2],
            );
        }
    }
}
