use agb::{
    display::object::{ObjectController, Tag},
    fixnum::Vector2D,
    InternalAllocator,
};
use alloc::boxed::Box;

use crate::{sfx::SfxPlayer, FixedNumberType, HatState, Level, BOSS_TAG_MAP};

use super::{BoxedEnemy, Enemy, EnemyInfo, EnemyUpdateState};

const SLIME_BOSS_IDLE: &Tag = BOSS_TAG_MAP.get("Slime Boss Idle");
const SLIME_BOSS_JUMP: &Tag = BOSS_TAG_MAP.get("Slime Boss Jump");

enum SlimeBossState {
    Idle(i32),
    Jumping(i32),
}

pub struct SlimeBoss<'a> {
    enemy_info: EnemyInfo<'a>,
    state: SlimeBossState,
}

impl<'a> Enemy<'a> for SlimeBoss<'a> {
    fn update(
        &mut self,
        controller: &'a ObjectController,
        level: &Level,
        player_pos: Vector2D<FixedNumberType>,
        hat_state: HatState,
        _timer: i32,
        sfx_player: &mut SfxPlayer,
    ) -> EnemyUpdateState {
        let player_has_collided =
            (self.enemy_info.entity.position - player_pos).magnitude_squared() < (20 * 20).into();

        match &mut self.state {
            SlimeBossState::Idle(counter) => {
                *counter += 1;

                let offset = ((*counter - 12) / 16) as usize;

                let frame = SLIME_BOSS_IDLE.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);

                if *counter > 8
                    && (self.enemy_info.entity.position - player_pos).magnitude_squared()
                        < (90 * 90).into()
                {
                    sfx_player.slime_jump();
                    self.state = SlimeBossState::Jumping(0);

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
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }
            }
            SlimeBossState::Jumping(i) => {
                if self.enemy_info.entity.is_on_ground(level) {
                    self.enemy_info.entity.velocity = (0, 0).into();
                    self.state = SlimeBossState::Idle(0);
                } else {
                    let sprite = SLIME_BOSS_JUMP.sprite((*i / 4).min(3) as usize);
                    let sprite = controller.sprite(sprite);
                    self.enemy_info.entity.sprite.set_sprite(sprite);
                    *i += 1;
                }

                if player_has_collided {
                    if hat_state == HatState::WizardTowards {
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }
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

impl<'a> SlimeBoss<'a> {
    pub fn new(object: &'a ObjectController, start_pos: Vector2D<FixedNumberType>) -> Self {
        let slime = SlimeBoss {
            enemy_info: EnemyInfo::new(object, start_pos, (65u16, 65u16).into()),
            state: SlimeBossState::Idle(0),
        };

        slime
    }

    pub fn new_boxed(
        object: &'a ObjectController,
        start_pos: Vector2D<FixedNumberType>,
    ) -> BoxedEnemy<'a> {
        Box::new_in(Self::new(object, start_pos), InternalAllocator)
    }
}
