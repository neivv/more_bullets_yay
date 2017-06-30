use libc::c_void;

pub struct GrpSprite;

#[repr(C, packed)]
pub struct RemapPalette {
    pub id: u32,
    pub data: *const u8,
    pub name: [u8; 0xc],
}

#[derive(Serialize, Deserialize, Clone)]
#[repr(C, packed)]
pub struct Iscript {
    pub header: u16,
    pub pos: u16,
    pub return_pos: u16,
    pub animation_id: u8,
    pub wait: u8,
}

#[repr(C, packed)]
pub struct Image {
    pub prev: *mut Image,
    pub next: *mut Image,
    pub image_id: u16,
    pub drawfunc: u8,
    pub direction: u8,
    pub flags: u16,
    pub x_offset: i8,
    pub y_offset: i8,
    pub iscript: Iscript,
    pub frameset: u16,
    pub frame: u16,
    pub map_position: Point,
    pub screen_position: [i16; 2],
    pub grp_bounds: [i16; 4],
    pub grp: *mut GrpSprite,
    pub drawfunc_param: *mut c_void,
    pub draw: unsafe extern "fastcall" fn(u32, u32, *const c_void, *const u16, *mut c_void),
    pub step_frame: unsafe extern "fastcall" fn(*mut Image),
    pub parent: *mut Sprite,
}

#[repr(C, packed)]
pub struct ImageDraw {
    pub id: u32,
    pub normal: unsafe extern "fastcall" fn(u32, u32, *const c_void, *const u16, *mut c_void),
    pub flipped: unsafe extern "fastcall" fn(u32, u32, *const c_void, *const u16, *mut c_void),
}

#[repr(C, packed)]
pub struct ImageStepFrame {
    pub id: u32,
    pub func: unsafe extern "fastcall" fn(*mut Image),
}

#[repr(C, packed)]
pub struct Sprite {
    pub prev: *mut Sprite,
    pub next: *mut Sprite,
    pub sprite_id: u16,
    pub player: u8,
    pub selection_index: u8,
    pub visibility_mask: u8,
    pub elevation: u8,
    pub flags: u8,
    pub selection_flash_timer: u8,
    pub index: u16,
    pub width: u8,
    pub height: u8,
    pub position: Point,
    pub main_image: *mut Image,
    pub first_overlay: *mut Image,
    pub last_overlay: *mut Image,
    pub extra: SpriteExtension,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SpriteExtension {
    pub spawn_order: u64,
}

pub struct Order {
    pub data: [u8; 0x14],
}

pub struct Path {
    pub data: [u8; 0x80],
}

#[derive(Serialize, Deserialize, Copy, Clone)]
#[repr(C, packed)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
#[repr(C, packed)]
pub struct Point32 {
    pub x: i32,
    pub y: i32,
}

#[repr(C, packed)]
pub struct LoneSprite {
    pub prev: *mut LoneSprite,
    pub next: *mut LoneSprite,
    pub value: u32,
    pub sprite: *mut Sprite,
}

#[repr(C, packed)]
pub struct Entity {
    pub prev: *mut Entity,
    pub next: *mut Entity,
    pub hitpoints: i32,
    pub sprite: *mut Sprite,
    pub move_target: Point,
    pub move_target_unit: *mut Unit,
    pub next_move_waypoint: Point,
    pub unk_move_waypoint: Point,
    pub flingy_flags: u8,
    pub facing_direction: u8,
    pub flingy_turn_speed: u8,
    pub movement_direction: u8,
    pub flingy_id: u16,
    pub unk_26: u8,
    pub flingy_movement_type: u8,
    pub position: Point,
    pub exact_position: Point32,
    pub flingy_top_speed: u32,
    pub current_speed: i32,
    pub next_speed: i32,
    pub speed: i32,
    pub speed2: i32,
    pub acceleration: u16,
    pub new_direction: u8,
    pub target_direction: u8,
    // Flingy end
    pub player: u8,
    pub order: u8,
    pub order_state: u8,
    pub order_signal: u8,
    pub order_fow_unit: u16,
    pub unused52: u16,
    pub order_timer: u8,
    pub ground_cooldown: u8,
    pub air_cooldown: u8,
    pub spell_cooldown: u8,
    pub order_target_pos: Point,
    pub target: *mut Unit,
}

#[repr(C, packed)]
pub struct Bullet {
    pub entity: Entity,
    pub weapon_id: u8,
    pub death_timer: u8,
    pub flags: u8,
    pub bounces_remaining: u8,
    pub parent: *mut Unit,
    pub previous_bounce_target: *mut Unit,
    pub spread_seed: u8,
    pub padding6d: [u8; 3],
}

#[repr(C, packed)]
pub struct Unit {
    pub entity: Entity,
    pub shields: i32,
    pub unit_id: u16,
    pub unused66: u16,
    pub next_player_unit: *mut Unit,
    pub prev_player_unit: *mut Unit,
    pub subunit: *mut Unit,
    pub order_queue_begin: *mut Order,
    pub order_queue_end: *mut Order,
    pub previous_attacker: *mut Unit,
    pub related: *mut Unit,
    pub highlight_order_count: u8,
    pub order_wait: u8,
    pub unk86: u8,
    pub attack_notify_timer: u8,
    pub previous_unit_id: u16,
    pub minimap_draw_counter: u8,
    pub minimap_draw_color: u8,
    pub unused8c: u16,
    pub rank: u8,
    pub kills: u8,
    pub last_attacking_player: u8,
    pub secondary_order_wait: u8,
    pub ai_spell_flags: u8,
    pub order_flags: u8,
    pub buttons: u16,
    pub invisibility_effects: u8,
    pub movement_state: u8,
    pub build_queue: [u16; 5],
    pub energy: u16,
    pub current_build_slot: u8,
    pub minor_unique_index: u8,
    pub secondary_order: u8,
    pub building_overlay_state: u8,
    pub build_hp_gain: u16,
    pub build_shield_gain: u16,
    pub remaining_build_time: u16,
    pub previous_hp: u16,
    pub loaded_units: [u16; 8],
    pub unit_specific: [u8; 16],
    pub unit_specific2: [u8; 12],
    pub flags: u32,
    pub carried_powerup_flags: u8,
    pub wireframe_seed: u8,
    pub secondary_order_state: u8,
    pub move_target_update_timer: u8,
    pub detection_status: u32,
    pub unke8: u16,
    pub unkea: u16,
    pub currently_building: *mut Unit,
    pub next_invisible: *mut Unit,
    pub prev_invisible: *mut Unit,
    pub rally_pylon: [u8; 8],
    pub path: *mut Path,
    pub path_frame: u8,
    pub pathing_flags: u8,
    pub _unk106: u8,
    pub _unk107: u8,
    pub collision_points: [u16; 0x4],
    pub spells: UnitSpells,
    pub bullet_spread_seed: u16,
    pub _padding132: [u8; 2],
    pub ai: *mut UnitAi,
    pub air_strength: u16,
    pub ground_strength: u16,
    pub pos_search_left: u32,
    pub pos_search_right: u32,
    pub pos_search_top: u32,
    pub pos_search_bottom: u32,
    pub repulse: Repulse,
}

#[repr(C, packed)]
pub struct UnitAi {
    pub next: *mut UnitAi,
    pub prev: *mut UnitAi,
    pub ty: u8,
}

#[repr(C, packed)]
pub struct GuardAi {
    pub data: [u8; 0x20],
}

#[repr(C, packed)]
pub struct WorkerAi {
    pub data: [u8; 0x18],
}

#[repr(C, packed)]
pub struct BuildingAi {
    pub data: [u8; 0x2c],
}

#[repr(C, packed)]
pub struct MilitaryAi {
    pub data: [u8; 0x14],
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Repulse {
    pub repulse_misc: u8,
    pub repulse_direction: u8,
    pub repulse_chunk_x: u8,
    pub repulse_chunk_y: u8,
}

pub struct UnitSpells {
    pub death_timer: u16,
    pub defensive_matrix_dmg: u16,
    pub matrix_timer: u8,
    pub stim_timer: u8,
    pub ensnare_timer: u8,
    pub lockdown_timer: u8,
    pub irradiate_timer: u8,
    pub stasis_timer: u8,
    pub plague_timer: u8,
    pub is_under_storm: u8,
    pub irradiated_by: *mut Unit,
    pub irradiate_player: u8,
    pub parasited_by_players: u8,
    pub master_spell_timer: u8,
    pub is_blind: u8,
    pub maelstrom_timer: u8,
    pub _unk125: u8,
    pub acid_spore_count: u8,
    pub acid_spore_timers: [u8; 0x9],
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_sizes() {
        use std::mem;
        assert_eq!(mem::size_of::<Unit>(), 0x150);
        assert_eq!(mem::size_of::<Bullet>(), 0x70);
        assert_eq!(mem::size_of::<Sprite>() - mem::size_of::<SpriteExtension>(), 0x24);
        assert_eq!(mem::size_of::<Image>(), 0x40);
        assert_eq!(mem::size_of::<RemapPalette>(), 0x14);
        assert_eq!(mem::size_of::<Order>(), 0x14);
        assert_eq!(mem::size_of::<Path>(), 0x80);
        assert_eq!(mem::size_of::<LoneSprite>(), 0x10);
    }
}
