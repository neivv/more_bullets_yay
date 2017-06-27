use std::collections::HashMap;
use std::mem;
use std::ptr::null_mut;

use bw;
use bullets::{bullet_to_id, bullet_from_id, LoadError, SaveError};

#[derive(Serialize, Deserialize)]
pub struct EntitySerializable {
    prev: u32,
    next: u32,
    hitpoints: i32,
    sprite: u16,
    move_target: bw::Point,
    move_target_unit: u16,
    next_move_waypoint: bw::Point,
    unk_move_waypoint: bw::Point,
    flingy_flags: u8,
    facing_direction: u8,
    flingy_turn_speed: u8,
    movement_direction: u8,
    flingy_id: u16,
    unk_26: u8,
    flingy_movement_type: u8,
    position: bw::Point,
    exact_position: bw::Point32,
    flingy_top_speed: u32,
    current_speed: i32,
    next_speed: i32,
    speed: i32,
    speed2: i32,
    acceleration: u16,
    new_direction: u8,
    target_direction: u8,
    player: u8,
    order: u8,
    order_state: u8,
    order_signal: u8,
    order_fow_unit: u16,
    unused52: u16,
    order_timer: u8,
    ground_cooldown: u8,
    air_cooldown: u8,
    spell_cooldown: u8,
    order_target_pos: bw::Point,
    target: u16,
}

pub unsafe fn entity_serializable(
    entity: *const bw::Entity,
    mapping: &HashMap<*mut bw::Bullet, u32>
) -> Result<EntitySerializable, SaveError> {
    let bw::Entity {
        prev,
        next,
        hitpoints,
        sprite,
        move_target,
        move_target_unit,
        next_move_waypoint,
        unk_move_waypoint,
        flingy_flags,
        facing_direction,
        flingy_turn_speed,
        movement_direction,
        flingy_id,
        unk_26,
        flingy_movement_type,
        position,
        exact_position,
        flingy_top_speed,
        current_speed,
        next_speed,
        speed,
        speed2,
        acceleration,
        new_direction,
        target_direction,
        player,
        order,
        order_state,
        order_signal,
        order_fow_unit,
        unused52,
        order_timer,
        ground_cooldown,
        air_cooldown,
        spell_cooldown,
        order_target_pos,
        target,
    } = *entity;
    Ok(EntitySerializable {
        prev: bullet_to_id(prev as *mut bw::Bullet, mapping)?,
        next: bullet_to_id(next as *mut bw::Bullet, mapping)?,
        hitpoints,
        sprite: sprite_to_id(sprite),
        move_target,
        move_target_unit: unit_to_id(move_target_unit),
        next_move_waypoint,
        unk_move_waypoint,
        flingy_flags,
        facing_direction,
        flingy_turn_speed,
        movement_direction,
        flingy_id,
        unk_26,
        flingy_movement_type,
        position,
        exact_position,
        flingy_top_speed,
        current_speed,
        next_speed,
        speed,
        speed2,
        acceleration,
        new_direction,
        target_direction,
        player,
        order,
        order_state,
        order_signal,
        order_fow_unit,
        unused52,
        order_timer,
        ground_cooldown,
        air_cooldown,
        spell_cooldown,
        order_target_pos,
        target: unit_to_id(target),
    })
}

pub fn deserialize_entity(
    entity: &EntitySerializable,
    mapping: &[*mut bw::Bullet],
) -> Result<bw::Entity, LoadError> {
    let EntitySerializable {
        prev,
        next,
        hitpoints,
        sprite,
        move_target,
        move_target_unit,
        next_move_waypoint,
        unk_move_waypoint,
        flingy_flags,
        facing_direction,
        flingy_turn_speed,
        movement_direction,
        flingy_id,
        unk_26,
        flingy_movement_type,
        position,
        exact_position,
        flingy_top_speed,
        current_speed,
        next_speed,
        speed,
        speed2,
        acceleration,
        new_direction,
        target_direction,
        player,
        order,
        order_state,
        order_signal,
        order_fow_unit,
        unused52,
        order_timer,
        ground_cooldown,
        air_cooldown,
        spell_cooldown,
        order_target_pos,
        target,
    } = *entity;
    Ok(bw::Entity {
        prev: bullet_from_id(prev, mapping)? as *mut bw::Entity,
        next: bullet_from_id(next, mapping)? as *mut bw::Entity,
        hitpoints,
        sprite: sprite_from_id(sprite)?,
        move_target,
        move_target_unit: unit_from_id(move_target_unit)?,
        next_move_waypoint,
        unk_move_waypoint,
        flingy_flags,
        facing_direction,
        flingy_turn_speed,
        movement_direction,
        flingy_id,
        unk_26,
        flingy_movement_type,
        position,
        exact_position,
        flingy_top_speed,
        current_speed,
        next_speed,
        speed,
        speed2,
        acceleration,
        new_direction,
        target_direction,
        player,
        order,
        order_state,
        order_signal,
        order_fow_unit,
        unused52,
        order_timer,
        ground_cooldown,
        air_cooldown,
        spell_cooldown,
        order_target_pos,
        target: unit_from_id(target)?,
    })
}

pub fn unit_to_id(val: *mut bw::Unit) -> u16 {
    unsafe {
        if val == null_mut() {
            0
        } else {
            let ptr: *mut bw::Unit = &mut bw::units[0];
            let val = (val as usize - ptr as usize) / mem::size_of::<bw::Unit>();
            assert!(val < 1700);
            val as u16 + 1
        }
    }
}

pub fn unit_from_id(val: u16) -> Result<*mut bw::Unit, LoadError> {
    if val == 0 {
        Ok(null_mut())
    } else if val <= 1700 {
        unsafe { Ok(&mut bw::units[val as usize - 1]) }
    } else {
        Err(LoadError::Corrupted(format!("Invalid unit id 0x{:x}", val)))
    }
}

pub fn sprite_to_id(val: *mut bw::Sprite) -> u16 {
    unsafe {
        if val == null_mut() {
            0
        } else {
            let ptr: *mut bw::Sprite = &mut bw::sprites[0];
            let val = (val as usize - ptr as usize) / mem::size_of::<bw::Sprite>();
            assert!(val < 2500);
            val as u16 + 1
        }
    }
}

pub fn sprite_from_id(val: u16) -> Result<*mut bw::Sprite, LoadError> {
    if val == 0 {
        Ok(null_mut())
    } else if val <= 2500 {
        unsafe { Ok(&mut bw::sprites[val as usize - 1]) }
    } else {
        Err(LoadError::Corrupted(format!("Invalid sprite id 0x{:x}", val)))
    }
}
