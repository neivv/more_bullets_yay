use std::mem;
use std::ptr::null_mut;

use bincode;
use flate2;
use libc::c_void;

use bw;
use entity_serialize::{self, deserialize_entity, entity_serializable, EntitySerializable};
use save::{fread, fwrite, fread_num, fwrite_num, SaveError, LoadError, print_text};
use sprites::{
    sprite_to_id_current_mapping,
    sprite_from_id_current_mapping,
    lone_sprite_from_id_current_mapping,
    lone_sprite_to_id_current_mapping,
};

const UNIT_SAVE_MAGIC: u16 = 0xffed;
// 16 megabytes, should be more than enough, both compressed and without.
const UNIT_SAVE_MAX_SIZE: u32 = 0x1_000_000;

struct ConvertUnits;

const GHOST: u16 = 0x01;
const CARRIER: u16 = 0x48;
const WARBRINGER: u16 = 0x51;
const GANTRITHOR: u16 = 0x52;
const REAVER: u16 = 0x53;
const SCARAB: u16 = 0x55;
const INTERCEPTOR: u16 = 0x49;
const NUCLEAR_SILO: u16 = 0x6c;
const PYLON: u16 = 0x9c;
const MINERAL_FIELD1: u16 = 0xb0;
const MINERAL_FIELD2: u16 = 0xb1;
const MINERAL_FIELD3: u16 = 0xb2;
const VESPENE_GEYSER: u16 = 0xbc;

impl entity_serialize::SaveEntityPointer for ConvertUnits {
    type Pointer = bw::Unit;
    fn pointer_to_id(&self, val: *mut bw::Unit) -> Result<u32, SaveError> {
        Ok(unit_to_id(val) as u32)
    }
}

impl entity_serialize::LoadEntityPointer for ConvertUnits {
    type Pointer = bw::Unit;
    fn id_to_pointer(&self, val: u32) -> Result<*mut bw::Unit, LoadError> {
        unit_from_id(val as u16)
    }
}

#[derive(Serialize, Deserialize)]
struct SaveGlobals {
    first_active: u16,
    last_active: u16,
    first_hidden: u16,
    last_hidden: u16,
    first_dying: u16,
    last_dying: u16,
    first_revealer: u16,
    last_revealer: u16,
    first_free: u16,
    last_free: u16,
    first_invisible: u16,
    player_units: [u16; 0xc],
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum UnitAiSerializable {
    NoAi,
    Guard(u16),
    Worker(u16),
    Building(u16),
    Military(u16),
}

impl UnitAiSerializable {
    unsafe fn new(ai: *mut bw::UnitAi) -> Result<UnitAiSerializable, SaveError> {
        use self::UnitAiSerializable::*;
        if ai == null_mut() {
            Ok(NoAi)
        } else {
            match (*ai).ty {
                1 => {
                    let ptr: *mut bw::GuardAi = &mut bw::guard_ais[0];
                    let val = (ai as usize - ptr as usize) / mem::size_of::<bw::GuardAi>();
                    assert!(val < 1000);
                    Ok(Guard(val as u16))
                }
                2 => {
                    let ptr: *mut bw::WorkerAi = &mut bw::worker_ais[0];
                    let val = (ai as usize - ptr as usize) / mem::size_of::<bw::WorkerAi>();
                    assert!(val < 1000);
                    Ok(Worker(val as u16))
                }
                3 => {
                    let ptr: *mut bw::BuildingAi = &mut bw::building_ais[0];
                    let val = (ai as usize - ptr as usize) / mem::size_of::<bw::BuildingAi>();
                    assert!(val < 1000);
                    Ok(Building(val as u16))
                }
                4 => {
                    let ptr: *mut bw::MilitaryAi = &mut bw::military_ais[0];
                    let val = (ai as usize - ptr as usize) / mem::size_of::<bw::MilitaryAi>();
                    assert!(val < 1000);
                    Ok(Military(val as u16))
                }
                _ => Err(SaveError::InvalidUnitAi((*ai).ty)),
            }
        }
    }

    unsafe fn to_pointer(self) -> Result<*mut bw::UnitAi, LoadError> {
        use self::UnitAiSerializable::*;
        match self {
            NoAi => Ok(null_mut()),
            Guard(val) => {
                let val = val as usize;
                if val >= 1000 {
                    Err(LoadError::Corrupted(format!("Invalid unit ai {:?}", self)))
                } else {
                    Ok(&mut bw::guard_ais[val] as *mut bw::GuardAi as *mut bw::UnitAi)
                }
            }
            Worker(val) => {
                let val = val as usize;
                if val >= 1000 {
                    Err(LoadError::Corrupted(format!("Invalid unit ai {:?}", self)))
                } else {
                    Ok(&mut bw::worker_ais[val] as *mut bw::WorkerAi as *mut bw::UnitAi)
                }
            }
            Building(val) => {
                let val = val as usize;
                if val >= 1000 {
                    Err(LoadError::Corrupted(format!("Invalid unit ai {:?}", self)))
                } else {
                    Ok(&mut bw::building_ais[val] as *mut bw::BuildingAi as *mut bw::UnitAi)
                }
            }
            Military(val) => {
                let val = val as usize;
                if val >= 1000 {
                    Err(LoadError::Corrupted(format!("Invalid unit ai {:?}", self)))
                } else {
                    Ok(&mut bw::military_ais[val] as *mut bw::MilitaryAi as *mut bw::UnitAi)
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct UnitSpecificSerializable([u8; 0x10]);

impl UnitSpecificSerializable {
    unsafe fn new(
        mut data: [u8; 0x10],
        unit_id: u16,
        is_building: bool,
    ) -> Result<UnitSpecificSerializable, SaveError> {
        let ptr = data.as_mut_ptr();
        if has_hangar(unit_id) {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(4) as *mut u32) =
                unit_to_id(*(ptr.offset(4) as *const *mut bw::Unit)) as u32;
        } else if unit_id == INTERCEPTOR || unit_id == SCARAB {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(4) as *mut u32) =
                unit_to_id(*(ptr.offset(4) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(8) as *mut u32) =
                unit_to_id(*(ptr.offset(8) as *const *mut bw::Unit)) as u32;
        } else if is_building {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
        } else if is_worker(unit_id) {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(8) as *mut u32) =
                unit_to_id(*(ptr.offset(8) as *const *mut bw::Unit)) as u32;
        }
        Ok(UnitSpecificSerializable(data))
    }

    unsafe fn deserialize(
        mut self,
        unit_id: u16,
        is_building: bool
    ) -> Result<[u8; 0x10], LoadError> {
        let ptr = self.0.as_mut_ptr();
        if has_hangar(unit_id) {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
            *(ptr.offset(4) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(4) as *const u16))?;
        } else if unit_id == INTERCEPTOR || unit_id == SCARAB {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
            *(ptr.offset(4) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(4) as *const u16))?;
            *(ptr.offset(8) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(8) as *const u16))?;
        } else if is_building {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
        } else if is_worker(unit_id) {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
            *(ptr.offset(8) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(8) as *const u16))?;
        }
        Ok(self.0)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct UnitSpecificSerializable2([u8; 0xc]);

impl UnitSpecificSerializable2 {
    unsafe fn new(
        mut data: [u8; 0xc],
        unit_id: u16,
    ) -> Result<UnitSpecificSerializable2, SaveError> {
        let ptr = data.as_mut_ptr();
        if is_resource(unit_id) {
            *(ptr.offset(4) as *mut u32) =
                unit_to_id(*(ptr.offset(4) as *const *mut bw::Unit)) as u32;
        } else if is_powerup(unit_id) {
            *(ptr.offset(4) as *mut u32) =
                unit_to_id(*(ptr.offset(4) as *const *mut bw::Unit)) as u32;
        } else if is_worker(unit_id) {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(4) as *mut u32) =
                unit_to_id(*(ptr.offset(4) as *const *mut bw::Unit)) as u32;
            *(ptr.offset(8) as *mut u32) =
                unit_to_id(*(ptr.offset(8) as *const *mut bw::Unit)) as u32;
        } else if unit_id == NUCLEAR_SILO {
            *(ptr.offset(0) as *mut u32) =
                unit_to_id(*(ptr.offset(0) as *const *mut bw::Unit)) as u32;
        } else if unit_id == GHOST {
            *(ptr.offset(0) as *mut u32) = lone_sprite_to_id_current_mapping(
                *(ptr.offset(0) as *const *mut bw::LoneSprite)
            )? as u32;
        } else if unit_id == PYLON {
            *(ptr.offset(0) as *mut u32) =
                sprite_to_id_current_mapping(*(ptr.offset(0) as *const *mut bw::Sprite))? as u32;
        }
        Ok(UnitSpecificSerializable2(data))
    }

    unsafe fn deserialize(
        mut self,
        unit_id: u16,
    ) -> Result<[u8; 0xc], LoadError> {
        let ptr = self.0.as_mut_ptr();
        if is_resource(unit_id) {
            *(ptr.offset(4) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(4) as *const u16))?;
        } else if is_powerup(unit_id) {
            *(ptr.offset(4) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(4) as *const u16))?;
        } else if is_worker(unit_id) {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
            *(ptr.offset(4) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(4) as *const u16))?;
            *(ptr.offset(8) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(8) as *const u16))?;
        } else if unit_id == NUCLEAR_SILO {
            *(ptr.offset(0) as *mut *mut bw::Unit) =
                unit_from_id(*(ptr.offset(0) as *const u16))?;
        } else if unit_id == GHOST {
            *(ptr.offset(0) as *mut *mut bw::LoneSprite) =
                lone_sprite_from_id_current_mapping(*(ptr.offset(0) as *const u32))?;
        } else if unit_id == PYLON {
            *(ptr.offset(0) as *mut *mut bw::Sprite) =
                sprite_from_id_current_mapping(*(ptr.offset(0) as *const u32))?;
        }
        Ok(self.0)
    }
}

fn has_hangar(unit_id: u16) -> bool {
    unit_id == CARRIER ||
        unit_id == GANTRITHOR ||
        unit_id == REAVER ||
        unit_id == WARBRINGER
}

fn is_resource(unit_id: u16) -> bool {
    unit_id == MINERAL_FIELD1 ||
        unit_id == MINERAL_FIELD2 ||
        unit_id == MINERAL_FIELD3 ||
        unit_id == VESPENE_GEYSER
}

unsafe fn is_worker(unit_id: u16) -> bool {
    bw::units_dat_flags[unit_id as usize] & 0x8 != 0
}

unsafe fn is_powerup(unit_id: u16) -> bool {
    bw::units_dat_flags[unit_id as usize] & 0x800 != 0
}

#[derive(Serialize, Deserialize, Clone)]
struct RallyPylonSerializable {
    val1: u16,
    val2: u16,
    val3: u16,
}

impl RallyPylonSerializable {
    unsafe fn new(data: [u8; 0x8], unit_id: u16) -> RallyPylonSerializable {
        let data = data.as_ptr();
        if unit_id == PYLON {
            RallyPylonSerializable {
                val1: unit_to_id(*(data.offset(0) as *const *mut bw::Unit)),
                val2: unit_to_id(*(data.offset(4) as *const *mut bw::Unit)),
                val3: 0,
            }
        } else {
            // Whatever
            RallyPylonSerializable {
                val1: *(data.offset(0) as *const u16),
                val2: *(data.offset(2) as *const u16),
                val3: unit_to_id(*(data.offset(4) as *const *mut bw::Unit)),
            }
        }
    }

    unsafe fn deserialize(self, unit_id: u16) -> Result<[u8; 8], LoadError> {
        let mut result = [0u8; 8];
        let ptr = result.as_mut_ptr();
        if unit_id == PYLON {
            *(ptr.offset(0) as *mut *mut bw::Unit) = unit_from_id(self.val1)?;
            *(ptr.offset(4) as *mut *mut bw::Unit) = unit_from_id(self.val2)?;
        } else {
            *(ptr.offset(0) as *mut u16) = self.val2;
            *(ptr.offset(2) as *mut u16) = self.val2;
            *(ptr.offset(4) as *mut *mut bw::Unit) = unit_from_id(self.val3)?;
        }
        Ok(result)
    }
}

#[derive(Serialize, Deserialize)]
struct UnitSerializable {
    entity: EntitySerializable,
    shields: i32,
    unit_id: u16,
    unused66: u16,
    next_player_unit: u16,
    prev_player_unit: u16,
    subunit: u16,
    order_queue_begin: u16,
    order_queue_end: u16,
    previous_attacker: u16,
    related: u16,
    highlight_order_count: u8,
    order_wait: u8,
    unk86: u8,
    attack_notify_timer: u8,
    previous_unit_id: u16,
    minimap_draw_counter: u8,
    minimap_draw_color: u8,
    unused8c: u16,
    rank: u8,
    kills: u8,
    last_attacking_player: u8,
    secondary_order_wait: u8,
    ai_spell_flags: u8,
    order_flags: u8,
    buttons: u16,
    invisibility_effects: u8,
    movement_state: u8,
    build_queue: [u16; 5],
    energy: u16,
    current_build_slot: u8,
    minor_unique_index: u8,
    secondary_order: u8,
    building_overlay_state: u8,
    build_hp_gain: u16,
    build_shield_gain: u16,
    remaining_build_time: u16,
    previous_hp: u16,
    loaded_units: [u16; 8],
    unit_specific: UnitSpecificSerializable,
    unit_specific2: UnitSpecificSerializable2,
    flags: u32,
    carried_powerup_flags: u8,
    wireframe_seed: u8,
    secondary_order_state: u8,
    move_target_update_timer: u8,
    detection_status: u32,
    unke8: u16,
    unkea: u16,
    currently_building: u16,
    next_invisible: u16,
    prev_invisible: u16,
    rally_pylon: RallyPylonSerializable,
    path: u16,
    path_frame: u8,
    pathing_flags: u8,
    _unk106: u8,
    _unk107: u8,
    collision_points: [u16; 0x4],
    spells: UnitSpellsSerializable,
    bullet_spread_seed: u16,
    _padding132: [u8; 2],
    ai: UnitAiSerializable,
    air_strength: u16,
    ground_strength: u16,
    pos_search_left: u32,
    pos_search_right: u32,
    pos_search_top: u32,
    pos_search_bottom: u32,
    repulse: bw::Repulse,
}

#[derive(Serialize, Deserialize)]
struct UnitSpellsSerializable {
    death_timer: u16,
    defensive_matrix_dmg: u16,
    matrix_timer: u8,
    stim_timer: u8,
    ensnare_timer: u8,
    lockdown_timer: u8,
    irradiate_timer: u8,
    stasis_timer: u8,
    plague_timer: u8,
    is_under_storm: u8,
    irradiated_by: u16,
    irradiate_player: u8,
    parasited_by_players: u8,
    master_spell_timer: u8,
    is_blind: u8,
    maelstrom_timer: u8,
    _unk125: u8,
    acid_spore_count: u8,
    acid_spore_timers: [u8; 0x9],
}

pub unsafe fn save_unit_chunk(file: *mut c_void) -> u32 {
    if let Err(e) = save_units(file) {
        error!("Couldn't save units: {}", e);
        print_text(&format!("Unable to save the game: {}", e));
        return 0;
    }
    1
}

unsafe fn save_units(file: *mut c_void) -> Result<(), SaveError> {
    let data = serialize_units()?;
    fwrite_num(file, UNIT_SAVE_MAGIC)?;
    fwrite_num(file, 1u32)?;
    fwrite_num(file, data.len() as u32)?;
    fwrite(file, &data)?;
    Ok(())
}

unsafe fn serialize_units() -> Result<Vec<u8>, SaveError> {
    let buf = Vec::with_capacity(0x10000);
    let mut writer = flate2::write::DeflateEncoder::new(buf, flate2::Compression::Default);

    let size_limit = bincode::Bounded(UNIT_SAVE_MAX_SIZE as u64);
    let globals = SaveGlobals {
        first_active: unit_to_id(*bw::first_active_unit),
        last_active: unit_to_id(*bw::last_active_unit),
        first_hidden: unit_to_id(*bw::first_hidden_unit),
        last_hidden: unit_to_id(*bw::last_hidden_unit),
        first_dying: unit_to_id(*bw::first_dying_unit),
        last_dying: unit_to_id(*bw::last_dying_unit),
        first_revealer: unit_to_id(*bw::first_revealer),
        last_revealer: unit_to_id(*bw::last_revealer),
        first_invisible: unit_to_id(*bw::first_invisible_unit),
        first_free: unit_to_id(*bw::first_free_unit),
        last_free: unit_to_id(*bw::last_free_unit),
        player_units: {
            let mut ids = [0; 0xc];
            for (&unit, out) in bw::first_player_unit.iter().zip(ids.iter_mut()) {
                *out = unit_to_id(unit);
            }
            ids
        }
    };
    bincode::serialize_into(&mut writer, &globals, size_limit)?;
    for unit in bw::units.iter() {
        let serializable = unit_serializable(unit)?;
        bincode::serialize_into(&mut writer, &serializable, size_limit)?;
        if writer.total_in() > UNIT_SAVE_MAX_SIZE as u64{
            return Err(SaveError::SizeLimit(writer.total_in()));
        }
    }
    Ok(writer.finish()?)
}

unsafe fn unit_serializable(unit: *const bw::Unit) -> Result<UnitSerializable, SaveError> {
    let bw::Unit {
        ref entity,
        shields,
        unit_id,
        unused66,
        next_player_unit,
        prev_player_unit,
        subunit,
        order_queue_begin,
        order_queue_end,
        previous_attacker,
        related,
        highlight_order_count,
        order_wait,
        unk86,
        attack_notify_timer,
        previous_unit_id,
        minimap_draw_counter,
        minimap_draw_color,
        unused8c,
        rank,
        kills,
        last_attacking_player,
        secondary_order_wait,
        ai_spell_flags,
        order_flags,
        buttons,
        invisibility_effects,
        movement_state,
        build_queue,
        energy,
        current_build_slot,
        minor_unique_index,
        secondary_order,
        building_overlay_state,
        build_hp_gain,
        build_shield_gain,
        remaining_build_time,
        previous_hp,
        loaded_units,
        unit_specific,
        unit_specific2,
        flags,
        carried_powerup_flags,
        wireframe_seed,
        secondary_order_state,
        move_target_update_timer,
        detection_status,
        unke8,
        unkea,
        currently_building,
        next_invisible,
        prev_invisible,
        rally_pylon,
        path,
        path_frame,
        pathing_flags,
        _unk106,
        _unk107,
        collision_points,
        spells: bw::UnitSpells {
            death_timer,
            defensive_matrix_dmg,
            matrix_timer,
            stim_timer,
            ensnare_timer,
            lockdown_timer,
            irradiate_timer,
            stasis_timer,
            plague_timer,
            is_under_storm,
            irradiated_by,
            irradiate_player,
            parasited_by_players,
            master_spell_timer,
            is_blind,
            maelstrom_timer,
            _unk125,
            acid_spore_count,
            acid_spore_timers,
        },
        bullet_spread_seed,
        _padding132,
        ai,
        air_strength,
        ground_strength,
        pos_search_left,
        pos_search_right,
        pos_search_top,
        pos_search_bottom,
        ref repulse
    } = *unit;
    let is_building = flags & 0x2 != 0;
    Ok(UnitSerializable {
        entity: entity_serializable(entity, &ConvertUnits)?,
        shields,
        unit_id,
        unused66,
        next_player_unit: unit_to_id(next_player_unit),
        prev_player_unit: unit_to_id(prev_player_unit),
        subunit: unit_to_id(subunit),
        order_queue_begin: order_to_id(order_queue_begin),
        order_queue_end: order_to_id(order_queue_end),
        previous_attacker: unit_to_id(previous_attacker),
        related: unit_to_id(related),
        highlight_order_count,
        order_wait,
        unk86,
        attack_notify_timer,
        previous_unit_id,
        minimap_draw_counter,
        minimap_draw_color,
        unused8c,
        rank,
        kills,
        last_attacking_player,
        secondary_order_wait,
        ai_spell_flags,
        order_flags,
        buttons,
        invisibility_effects,
        movement_state,
        build_queue,
        energy,
        current_build_slot,
        minor_unique_index,
        secondary_order,
        building_overlay_state,
        build_hp_gain,
        build_shield_gain,
        remaining_build_time,
        previous_hp,
        loaded_units,
        unit_specific: UnitSpecificSerializable::new(unit_specific, unit_id, is_building)?,
        unit_specific2: UnitSpecificSerializable2::new(unit_specific2, unit_id)?,
        flags,
        carried_powerup_flags,
        wireframe_seed,
        secondary_order_state,
        move_target_update_timer,
        detection_status,
        unke8,
        unkea,
        currently_building: unit_to_id(currently_building),
        next_invisible: unit_to_id(next_invisible),
        prev_invisible: unit_to_id(prev_invisible),
        rally_pylon: RallyPylonSerializable::new(rally_pylon, unit_id),
        path: path_to_id(path),
        path_frame,
        pathing_flags,
        _unk106,
        _unk107,
        collision_points,
        spells: UnitSpellsSerializable {
            death_timer,
            defensive_matrix_dmg,
            matrix_timer,
            stim_timer,
            ensnare_timer,
            lockdown_timer,
            irradiate_timer,
            stasis_timer,
            plague_timer,
            is_under_storm,
            irradiated_by: unit_to_id(irradiated_by),
            irradiate_player,
            parasited_by_players,
            master_spell_timer,
            is_blind,
            maelstrom_timer,
            _unk125,
            acid_spore_count,
            acid_spore_timers,
        },
        bullet_spread_seed,
        _padding132,
        ai: UnitAiSerializable::new(ai)?,
        air_strength,
        ground_strength,
        pos_search_left,
        pos_search_right,
        pos_search_top,
        pos_search_bottom,
        repulse: repulse.clone(),
    })
}

pub unsafe fn load_unit_chunk(file: *mut c_void, save_version: u32) -> u32 {
    if save_version != 3 {
        error!("Unusupported save version: {}", save_version);
        return 0;
    }
    if let Err(e) = load_units(file) {
        info!("Couldn't load a save: {}", e);
        return 0;
    }
    1
}

unsafe fn load_units(file: *mut c_void) -> Result<(), LoadError> {
    let magic = fread_num::<u16>(file)?;
    if magic != UNIT_SAVE_MAGIC {
        return Err(LoadError::WrongMagic(magic));
    }
    let version = fread_num::<u32>(file)?;
    if version != 1 {
        return Err(LoadError::Version(version));
    }
    let size = fread_num::<u32>(file)?;
    if size > UNIT_SAVE_MAX_SIZE {
        return Err(LoadError::Corrupted(format!("Unit chunk size {} is too large", size)));
    }
    let data = fread(file, size)?;
    let mut reader = flate2::read::DeflateDecoder::new(&data[..]);


    let size_limit = bincode::Bounded(UNIT_SAVE_MAX_SIZE as u64);
    let globals: SaveGlobals = bincode::deserialize_from(&mut reader, size_limit)?;
    for unit in bw::units.iter_mut() {
        let serialized = bincode::deserialize_from(&mut reader, size_limit)?;
        *unit = deserialize_unit(&serialized)?;
        if reader.total_out() > UNIT_SAVE_MAX_SIZE as u64 {
            return Err(LoadError::SizeLimit)
        }
    }
    *bw::first_active_unit = unit_from_id(globals.first_active)?;
    *bw::first_hidden_unit = unit_from_id(globals.first_hidden)?;
    *bw::first_dying_unit = unit_from_id(globals.first_dying)?;
    *bw::first_revealer= unit_from_id(globals.first_revealer)?;
    *bw::first_free_unit = unit_from_id(globals.first_free)?;
    *bw::first_invisible_unit = unit_from_id(globals.first_invisible)?;
    *bw::last_active_unit = unit_from_id(globals.last_active)?;
    *bw::last_hidden_unit = unit_from_id(globals.last_hidden)?;
    *bw::last_dying_unit = unit_from_id(globals.last_dying)?;
    *bw::last_revealer= unit_from_id(globals.last_revealer)?;
    *bw::last_free_unit = unit_from_id(globals.last_free)?;
    for (unit, &saved) in bw::first_player_unit.iter_mut().zip(globals.player_units.iter()) {
        *unit = unit_from_id(saved)?;
    }

    let mut unit = *bw::first_active_unit;
    while unit != null_mut() {
        add_unit_to_game(unit);
        unit = (*unit).entity.next as *mut bw::Unit;
    }

    Ok(())
}

unsafe fn add_unit_to_game(unit: *mut bw::Unit) {
    if (*unit).pos_search_left != !0 {
        (*unit).pos_search_left = !0;
        (*unit).pos_search_top = !0;
        (*unit).pos_search_right = !0;
        (*unit).pos_search_bottom = !0;
        bw::add_to_pos_search(unit);
        if (*unit).flags & 0x2 != 0 {
            let pos = (*(*unit).entity.sprite).position;
            bw::set_building_tile_flag(unit, pos.x as u32, pos.y as u32);
        }
        bw::check_unstack(unit);
        if (*unit).flags & 0x4 != 0 {
            bw::add_to_repulse_chunk(unit);
        }
    }
}

unsafe fn deserialize_unit(unit: &UnitSerializable) -> Result<bw::Unit, LoadError> {
    let UnitSerializable {
        ref entity,
        shields,
        unit_id,
        unused66,
        next_player_unit,
        prev_player_unit,
        subunit,
        order_queue_begin,
        order_queue_end,
        previous_attacker,
        related,
        highlight_order_count,
        order_wait,
        unk86,
        attack_notify_timer,
        previous_unit_id,
        minimap_draw_counter,
        minimap_draw_color,
        unused8c,
        rank,
        kills,
        last_attacking_player,
        secondary_order_wait,
        ai_spell_flags,
        order_flags,
        buttons,
        invisibility_effects,
        movement_state,
        build_queue,
        energy,
        current_build_slot,
        minor_unique_index,
        secondary_order,
        building_overlay_state,
        build_hp_gain,
        build_shield_gain,
        remaining_build_time,
        previous_hp,
        loaded_units,
        ref unit_specific,
        ref unit_specific2,
        flags,
        carried_powerup_flags,
        wireframe_seed,
        secondary_order_state,
        move_target_update_timer,
        detection_status,
        unke8,
        unkea,
        currently_building,
        next_invisible,
        prev_invisible,
        ref rally_pylon,
        path,
        path_frame,
        pathing_flags,
        _unk106,
        _unk107,
        collision_points,
        spells: UnitSpellsSerializable {
            death_timer,
            defensive_matrix_dmg,
            matrix_timer,
            stim_timer,
            ensnare_timer,
            lockdown_timer,
            irradiate_timer,
            stasis_timer,
            plague_timer,
            is_under_storm,
            irradiated_by,
            irradiate_player,
            parasited_by_players,
            master_spell_timer,
            is_blind,
            maelstrom_timer,
            _unk125,
            acid_spore_count,
            acid_spore_timers,
        },
        bullet_spread_seed,
        _padding132,
        ref ai,
        air_strength,
        ground_strength,
        pos_search_left,
        pos_search_right,
        pos_search_top,
        pos_search_bottom,
        ref repulse,
    } = *unit;
    let is_building = flags & 0x2 != 0;
    Ok(bw::Unit {
        entity: deserialize_entity(entity, &ConvertUnits)?,
        shields,
        unit_id,
        unused66,
        next_player_unit: unit_from_id(next_player_unit)?,
        prev_player_unit: unit_from_id(prev_player_unit)?,
        subunit: unit_from_id(subunit)?,
        order_queue_begin: order_from_id(order_queue_begin)?,
        order_queue_end: order_from_id(order_queue_end)?,
        previous_attacker: unit_from_id(previous_attacker)?,
        related: unit_from_id(related)?,
        highlight_order_count,
        order_wait,
        unk86,
        attack_notify_timer,
        previous_unit_id,
        minimap_draw_counter,
        minimap_draw_color,
        unused8c,
        rank,
        kills,
        last_attacking_player,
        secondary_order_wait,
        ai_spell_flags,
        order_flags,
        buttons,
        invisibility_effects,
        movement_state,
        build_queue,
        energy,
        current_build_slot,
        minor_unique_index,
        secondary_order,
        building_overlay_state,
        build_hp_gain,
        build_shield_gain,
        remaining_build_time,
        previous_hp,
        loaded_units,
        unit_specific: unit_specific.clone().deserialize(unit_id, is_building)?,
        unit_specific2: unit_specific2.clone().deserialize(unit_id)?,
        flags,
        carried_powerup_flags,
        wireframe_seed,
        secondary_order_state,
        move_target_update_timer,
        detection_status,
        unke8,
        unkea,
        currently_building: unit_from_id(currently_building)?,
        next_invisible: unit_from_id(next_invisible)?,
        prev_invisible: unit_from_id(prev_invisible)?,
        rally_pylon: rally_pylon.clone().deserialize(unit_id)?,
        path: path_from_id(path)?,
        path_frame,
        pathing_flags,
        _unk106,
        _unk107,
        collision_points,
        spells: bw::UnitSpells {
            death_timer,
            defensive_matrix_dmg,
            matrix_timer,
            stim_timer,
            ensnare_timer,
            lockdown_timer,
            irradiate_timer,
            stasis_timer,
            plague_timer,
            is_under_storm,
            irradiated_by: unit_from_id(irradiated_by)?,
            irradiate_player,
            parasited_by_players,
            master_spell_timer,
            is_blind,
            maelstrom_timer,
            _unk125,
            acid_spore_count,
            acid_spore_timers,
        },
        bullet_spread_seed,
        _padding132,
        ai: ai.to_pointer()?,
        air_strength,
        ground_strength,
        pos_search_left,
        pos_search_right,
        pos_search_top,
        pos_search_bottom,
        repulse: repulse.clone(),
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

pub fn order_to_id(val: *mut bw::Order) -> u16 {
    unsafe {
        if val == null_mut() {
            0
        } else {
            let ptr: *mut bw::Order = &mut bw::orders[0];
            let val = (val as usize - ptr as usize) / mem::size_of::<bw::Order>();
            assert!(val < 2000);
            val as u16 + 1
        }
    }
}

pub fn order_from_id(val: u16) -> Result<*mut bw::Order, LoadError> {
    if val == 0 {
        Ok(null_mut())
    } else if val <= 2000 {
        unsafe { Ok(&mut bw::orders[val as usize - 1]) }
    } else {
        Err(LoadError::Corrupted(format!("Invalid order id 0x{:x}", val)))
    }
}

pub fn path_to_id(val: *mut bw::Path) -> u16 {
    unsafe {
        if val == null_mut() {
            0
        } else {
            let ptr: *mut bw::Path = *bw::path_array_start;
            let val = (val as usize - ptr as usize) / mem::size_of::<bw::Path>();
            val as u16 + 1
        }
    }
}

pub fn path_from_id(val: u16) -> Result<*mut bw::Path, LoadError> {
    if val == 0 {
        Ok(null_mut())
    } else {
        unsafe { Ok(bw::path_array_start.offset(val as isize - 1)) }
    }
}
