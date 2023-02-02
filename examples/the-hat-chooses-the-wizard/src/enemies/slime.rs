use agb::{
    display::object::{ObjectController, Tag},
    fixnum::Vector2D,
    InternalAllocator,
};
use alloc::boxed::Box;

use crate::{sfx::SfxPlayer, FixedNumberType, HatState, Level, TAG_MAP};

use super::{BoxedEnemy, Enemy, EnemyInfo, EnemyUpdateState};

const SLIME_IDLE: &Tag = TAG_MAP.get("Slime Idle");
const SLIME_JUMP: &Tag = TAG_MAP.get("Slime Jump");
const SLIME_SPLAT: &Tag = TAG_MAP.get("Slime splat");

enum SlimeState {
    Idle(i32),
    Jumping,
    Dying(i32), // the start frame of the dying animation
}

pub struct Slime<'a> {
    enemy_info: EnemyInfo<'a>,
    state: SlimeState,
}

impl<'a> Enemy<'a> for Slime<'a> {
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

        match &mut self.state {
            SlimeState::Idle(counter) => {
                let offset = (timer / 16) as usize;

                *counter += 1;

                let frame = SLIME_IDLE.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);

                if *counter > 8
                    && (self.enemy_info.entity.position - player_pos).magnitude_squared()
                        < (64 * 64).into()
                {
                    sfx_player.slime_jump();
                    self.state = SlimeState::Jumping;

                    let x_vel: FixedNumberType =
                        if self.enemy_info.entity.position.x > player_pos.x {
                            -1
                        } else {
                            1
                        }
                        .into();

                    self.enemy_info.entity.velocity =
                        (x_vel / 4, FixedNumberType::new(-3) / 2).into();
                }

                if player_has_collided {
                    if hat_state == HatState::WizardTowards {
                        self.state = SlimeState::Dying(timer);
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }
            }
            SlimeState::Jumping => {
                if self.enemy_info.entity.is_on_ground(level) {
                    self.enemy_info.entity.velocity = (0, 0).into();
                    self.state = SlimeState::Idle(0);
                } else {
                    let sprite =
                        if self.enemy_info.entity.velocity.y.abs() > FixedNumberType::new(1) / 4 {
                            SLIME_JUMP.sprite(1)
                        } else {
                            SLIME_JUMP.sprite(0)
                        };
                    let sprite = controller.sprite(sprite);
                    self.enemy_info.entity.sprite.set_sprite(sprite);
                }

                if player_has_collided {
                    if hat_state == HatState::WizardTowards {
                        self.state = SlimeState::Dying(timer);
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }
            }
            SlimeState::Dying(dying_start_frame) => {
                if timer == *dying_start_frame + 1 {
                    sfx_player.slime_death();
                }

                let offset = (timer - *dying_start_frame) as usize / 4;
                self.enemy_info.entity.velocity = (0, 0).into();

                if offset >= 4 {
                    return EnemyUpdateState::Remove;
                }

                let frame = SLIME_SPLAT.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);
            }
        }

        self.enemy_info.update(level);

        EnemyUpdateState::None
    }

    fn commit(&mut self, background_offset: Vector2D<FixedNumberType>) {
        self.enemy_info.commit(background_offset);
    }

    fn collides_with_hat(&self, _position: Vector2D<FixedNumberType>) -> bool {
        false
    }
}

impl<'a> Slime<'a> {
    pub fn new(object: &'a ObjectController, start_pos: Vector2D<FixedNumberType>) -> Self {
        let slime = Slime {
            enemy_info: EnemyInfo::new(object, start_pos, (12u16, 9u16).into()),
            state: SlimeState::Idle(0),
        };

        slime
    }

    pub fn new_boxed(
        object: &'a ObjectController,
        start_pos: Vector2D<FixedNumberType>,
    ) -> BoxedEnemy {
        Box::new_in(Self::new(object, start_pos), InternalAllocator)
    }
}
