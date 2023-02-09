use super::{sfx::SfxPlayer, Entity, FixedNumberType, HatState, Level};
use agb::{display::object::ObjectController, fixnum::Vector2D, InternalAllocator};
use alloc::boxed::Box;

pub mod bat;
pub mod slime;
pub mod snail;

pub type BoxedEnemy<'a> = Box<dyn Enemy<'a> + 'a, InternalAllocator>;

#[derive(Debug, Clone, Copy)]
pub enum EnemyType {
    Bat,
    Slime,
    Snail,
}

pub fn new_enemy<'a>(
    enemy_type: EnemyType,
    object: &'a ObjectController,
    start_pos: Vector2D<FixedNumberType>,
) -> BoxedEnemy {
    match enemy_type {
        EnemyType::Bat => bat::Bat::new_boxed(object, start_pos),
        EnemyType::Slime => slime::Slime::new_boxed(object, start_pos),
        EnemyType::Snail => snail::Snail::new_boxed(object, start_pos),
    }
}

pub trait Enemy<'a> {
    fn collides_with_hat(&self, position: Vector2D<FixedNumberType>) -> bool;
    fn update(
        &mut self,
        controller: &'a ObjectController,
        level: &Level,
        player_pos: Vector2D<FixedNumberType>,
        hat_state: HatState,
        timer: i32,
        sfx_player: &mut SfxPlayer,
    ) -> EnemyUpdateState;
    fn commit(&mut self, background_offset: Vector2D<FixedNumberType>);
}

pub enum EnemyUpdateState {
    None,
    Remove,
    KillPlayer,
}
struct EnemyInfo<'a> {
    entity: Entity<'a>,
    affected_by_gravity: bool,
}

impl<'a> EnemyInfo<'a> {
    fn new(
        object: &'a ObjectController,
        start_pos: Vector2D<FixedNumberType>,
        collision: Vector2D<u16>,
    ) -> Self {
        let mut enemy_info = EnemyInfo {
            entity: Entity::new(object, collision),
            affected_by_gravity: true,
        };
        enemy_info.entity.position = start_pos;
        enemy_info
    }

    fn update(&mut self, level: &Level) {
        for &enemy_stop in level.enemy_stops {
            if (self.entity.position + self.entity.velocity - enemy_stop.into())
                .manhattan_distance()
                < 8.into()
            {
                self.entity.velocity.x = 0.into();
            }
        }

        if self.affected_by_gravity {
            let is_on_ground = self.entity.is_on_ground(level);
            if !is_on_ground {
                self.entity.velocity += crate::GRAVITY;
            }
        }

        self.entity.velocity = self.entity.update_position(level);
    }

    fn commit(&mut self, background_offset: Vector2D<FixedNumberType>) {
        self.entity.commit_position(background_offset);
    }
}
