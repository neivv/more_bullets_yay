use bw;
use save::{LoadError, SaveError};
use sprites;
use units;

pub trait SaveEntityPointer {
    type Pointer;
    fn pointer_to_id(&self, pointer: *mut Self::Pointer) -> Result<u32, SaveError>;
}

pub trait LoadEntityPointer {
    type Pointer;
    fn id_to_pointer(&self, id: u32) -> Result<*mut Self::Pointer, LoadError>;
}

#[derive(Serialize, Deserialize)]
pub struct EntitySerializable {
    prev: u32,
    next: u32,
    hitpoints: i32,
    sprite: u32,
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

pub unsafe fn entity_serializable<C: SaveEntityPointer>(
    entity: *const bw::Entity,
    save_pointer: &C,
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
        prev: save_pointer.pointer_to_id(prev as *mut C::Pointer)?,
        next: save_pointer.pointer_to_id(next as *mut C::Pointer)?,
        hitpoints,
        sprite: sprites::sprite_to_id_current_mapping(sprite)?,
        move_target,
        move_target_unit: units::unit_to_id(move_target_unit),
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
        target: units::unit_to_id(target),
    })
}

pub fn deserialize_entity<C: LoadEntityPointer>(
    entity: &EntitySerializable,
    load_pointer: &C,
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
        prev: load_pointer.id_to_pointer(prev)? as *mut bw::Entity,
        next: load_pointer.id_to_pointer(next)? as *mut bw::Entity,
        hitpoints,
        sprite: sprites::sprite_from_id_current_mapping(sprite)?,
        move_target,
        move_target_unit: units::unit_from_id(move_target_unit)?,
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
        target: units::unit_from_id(target)?,
    })
}
