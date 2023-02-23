use agb::{
    display::object::{ObjectController, Tag},
    fixnum::Vector2D,
    InternalAllocator,
};
use alloc::boxed::Box;

use crate::{sfx::SfxPlayer, FixedNumberType, HatState, Level, TAG_MAP};

use super::{BoxedEnemy, Enemy, EnemyInfo, EnemyUpdateState};

const SNAIL_EMERGE: &Tag = TAG_MAP.get("Snail Emerge");
const SNAIL_MOVE: &Tag = TAG_MAP.get("Snail Move");
const SNAIL_DEATH: &Tag = TAG_MAP.get("Snail Death");
const SNAIL_IDLE: &Tag = TAG_MAP.get("Snail Idle");

enum SnailState {
    Idle(i32),       // start frame (or 0 if newly created)
    Emerging(i32),   // start frame
    Retreating(i32), // start frame
    Moving(i32),     // start frame
    Death(i32),      // start frame
}

pub struct Snail<'a> {
    enemy_info: EnemyInfo<'a>,
    state: SnailState,
}

impl<'a> Enemy<'a> for Snail<'a> {
    fn update(
        &mut self,
        controller: &'a ObjectController,
        level: &Level,
        player_pos: Vector2D<FixedNumberType>,
        hat_state: HatState,
        timer: i32,
        sfx_player: &mut SfxPlayer,
    ) -> EnemyUpdateState {
        let player_has_collided =
            (self.enemy_info.entity.position - player_pos).magnitude_squared() < (10 * 10).into();

        match self.state {
            SnailState::Idle(wait_time) => {
                self.enemy_info.entity.velocity = (0, 0).into();

                if wait_time == 0 || timer - wait_time > 120 {
                    // wait at least 2 seconds after switching to this state
                    if (self.enemy_info.entity.position - player_pos).magnitude_squared()
                        < (48 * 48).into()
                    {
                        // player is close
                        self.state = SnailState::Emerging(timer);
                        sfx_player.snail_emerge();
                    }
                }

                let frame = SNAIL_IDLE.animation_sprite(0);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);
                if player_has_collided {
                    if hat_state != HatState::WizardTowards {
                        return EnemyUpdateState::KillPlayer;
                    } else {
                        self.state = SnailState::Death(timer);
                    }
                }
            }
            SnailState::Emerging(time) => {
                let offset = (timer - time) as usize / 4;

                if offset >= 5 {
                    self.state = SnailState::Moving(timer);
                }
                self.enemy_info.entity.velocity = (0, 0).into();

                let frame = SNAIL_EMERGE.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);

                if player_has_collided {
                    if hat_state != HatState::WizardTowards {
                        return EnemyUpdateState::KillPlayer;
                    } else if hat_state == HatState::WizardTowards {
                        self.state = SnailState::Death(timer);
                    }
                }
            }
            SnailState::Moving(time) => {
                if timer - time > 240 {
                    // only move for 4 seconds
                    self.state = SnailState::Retreating(timer);
                    sfx_player.snail_retreat();
                }

                let offset = (timer - time) as usize / 8;

                let frame = SNAIL_MOVE.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);

                if timer % 32 == 0 {
                    let x_vel: FixedNumberType =
                        if self.enemy_info.entity.position.x < player_pos.x {
                            self.enemy_info.entity.sprite.set_hflip(false);
                            1
                        } else {
                            self.enemy_info.entity.sprite.set_hflip(true);
                            -1
                        }
                        .into();

                    self.enemy_info.entity.velocity = (x_vel / 8, 0.into()).into();
                }

                if player_has_collided {
                    if hat_state != HatState::WizardTowards {
                        return EnemyUpdateState::KillPlayer;
                    } else if hat_state == HatState::WizardTowards {
                        self.state = SnailState::Death(timer);
                    }
                }
            }
            SnailState::Retreating(time) => {
                let offset = 5 - (timer - time) as usize / 4;

                if offset == 0 {
                    self.state = SnailState::Idle(timer);
                }

                let frame = SNAIL_EMERGE.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);
                self.enemy_info.entity.velocity = (0, 0).into();

                if player_has_collided {
                    if hat_state != HatState::WizardTowards {
                        return EnemyUpdateState::KillPlayer;
                    } else if hat_state == HatState::WizardTowards {
                        self.state = SnailState::Death(timer);
                    }
                }
            }
            SnailState::Death(time) => {
                if timer == time + 1 {
                    sfx_player.snail_death();
                }

                let offset = (timer - time) as usize / 4;
                let frame = if offset < 5 {
                    SNAIL_EMERGE.animation_sprite(5 - offset)
                } else if offset == 5 {
                    SNAIL_IDLE.animation_sprite(0)
                } else if offset < 5 + 7 {
                    SNAIL_DEATH.animation_sprite(offset - 5)
                } else {
                    return EnemyUpdateState::Remove;
                };

                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);
                self.enemy_info.entity.velocity = (0, 0).into();
            }
        }

        self.enemy_info.update(level);

        EnemyUpdateState::None
    }

    fn commit(&mut self, background_offset: Vector2D<FixedNumberType>) {
        self.enemy_info.commit(background_offset);
    }

    fn collides_with_hat(&self, position: agb::fixnum::Vector2D<crate::FixedNumberType>) -> bool {
        self.collides_with(position)
    }
}

impl<'a> Snail<'a> {
    fn new(object: &'a ObjectController, start_pos: Vector2D<FixedNumberType>) -> Self {
        let snail = Snail {
            enemy_info: EnemyInfo::new(object, start_pos, (16u16, 16u16).into()),
            state: SnailState::Idle(0),
        };

        snail
    }

    pub fn new_boxed(
        object: &'a ObjectController,
        start_pos: Vector2D<FixedNumberType>,
    ) -> BoxedEnemy<'a> {
        Box::new_in(Self::new(object, start_pos), InternalAllocator)
    }

    fn collides_with(&self, position: Vector2D<FixedNumberType>) -> bool {
        (self.enemy_info.entity.position - position).magnitude_squared() < (15 * 15).into()
    }
}
