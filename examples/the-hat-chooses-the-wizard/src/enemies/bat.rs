use agb::{
    display::object::{ObjectController, Tag},
    fixnum::Vector2D,
    InternalAllocator,
};
use alloc::boxed::Box;

use crate::{sfx::SfxPlayer, FixedNumberType, HatState, Level, TAG_MAP};

const BAT_FLY: &Tag = TAG_MAP.get("Bat");
const BAT_DEAD: &Tag = TAG_MAP.get("Bat dead");

use super::{BoxedEnemy, Enemy, EnemyInfo, EnemyUpdateState};

const CHASE_TIME: u32 = 5;

enum BatState {
    Idle,
    Chasing(u32), // remaining frames to chase for
    Falling(i32), // the frame the bat died
    Dead,
}

pub struct Bat<'a> {
    enemy_info: EnemyInfo<'a>,
    state: BatState,
}

impl<'a> Enemy<'a> for Bat<'a> {
    fn collides_with_hat(&self, _position: Vector2D<FixedNumberType>) -> bool {
        false
    }

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

        const SLOW_FLAP_REDUCTION: i32 = 8;
        const FAST_FLAP_REDUCTION: i32 = 1;
        const FLAP_FRAMES: i32 = 9;
        const SFX_FRAME: i32 = 6;

        match &mut self.state {
            BatState::Idle => {
                self.enemy_info.affected_by_gravity = false;
                let offset = (timer / SLOW_FLAP_REDUCTION) as usize;
                let frame = BAT_FLY.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.velocity = (0, 0).into();

                self.enemy_info.entity.sprite.set_sprite(sprite);

                if timer % (FLAP_FRAMES * SLOW_FLAP_REDUCTION) == SFX_FRAME * SLOW_FLAP_REDUCTION {
                    sfx_player.bat_flap();
                }

                if (self.enemy_info.entity.position - player_pos).magnitude_squared()
                    < (64 * 64).into()
                {
                    self.state = BatState::Chasing(CHASE_TIME * 60);
                }

                if player_has_collided {
                    if hat_state == HatState::WizardTowards {
                        self.state = BatState::Falling(timer);
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }
            }
            BatState::Chasing(0) => {
                self.state = BatState::Idle;
            }
            BatState::Chasing(remaining_frames) => {
                *remaining_frames -= 1;
                let offset = timer as usize;

                let frame = BAT_FLY.animation_sprite(offset);
                let sprite = controller.sprite(frame);

                self.enemy_info.entity.sprite.set_sprite(sprite);

                let direction = (player_pos - self.enemy_info.entity.position).fast_normalise() / 2;

                self.enemy_info.entity.velocity = direction;

                if player_has_collided {
                    if hat_state == HatState::WizardTowards {
                        self.state = BatState::Falling(timer);
                    } else {
                        return EnemyUpdateState::KillPlayer;
                    }
                }

                if timer % (FLAP_FRAMES * FAST_FLAP_REDUCTION) == SFX_FRAME * FAST_FLAP_REDUCTION {
                    sfx_player.bat_flap();
                }
            }
            BatState::Falling(falling_start_frame) => {
                self.enemy_info.affected_by_gravity = true;
                if timer == *falling_start_frame + 1 {
                    let frame = BAT_DEAD.animation_sprite(0);
                    let sprite = controller.sprite(frame);
                    self.enemy_info.entity.sprite.set_sprite(sprite);

                    sfx_player.bat_dead();
                }

                if self.enemy_info.entity.is_on_ground(level) {
                    self.enemy_info.entity.velocity = (0, 0).into();
                    self.state = BatState::Dead;
                }
            }
            BatState::Dead => {}
        }

        self.enemy_info.update(level);

        EnemyUpdateState::None
    }

    fn commit(&mut self, background_offset: Vector2D<FixedNumberType>) {
        self.enemy_info.commit(background_offset);
    }
}

impl<'a> Bat<'a> {
    pub fn new(object: &'a ObjectController, start_pos: Vector2D<FixedNumberType>) -> Self {
        Self {
            enemy_info: EnemyInfo::new(object, start_pos, (12u16, 12u16).into()),
            state: BatState::Idle,
        }
    }

    pub fn new_boxed(
        object: &'a ObjectController,
        start_pos: Vector2D<FixedNumberType>,
    ) -> BoxedEnemy {
        Box::new_in(Self::new(object, start_pos), InternalAllocator)
    }
}
