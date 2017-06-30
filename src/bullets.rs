use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::ptr::null_mut;

use bincode;
use flate2;
use libc::c_void;

use bw;
use entity_serialize::{self, deserialize_entity, entity_serializable, EntitySerializable};
use units::{unit_to_id, unit_from_id};
use save::{fread, fwrite, fread_num, fwrite_num, SaveError, LoadError, print_text};
use save::{SaveMapping, LoadMapping};
use send_pointer::SendPtr;

ome2_thread_local! {
    BULLETS: RefCell<HashSet<SendPtr<bw::Bullet>>> =
        all_bullets(RefCell::new(HashSet::new()));
}

const BULLET_SAVE_MAGIC: u16 = 0xffed;
// 8 megabytes, should be more than enough, both compressed and without.
const BULLET_SAVE_MAX_SIZE: u32 = 0x800000;

impl entity_serialize::SaveEntityPointer for SaveMapping<bw::Bullet> {
    type Pointer = bw::Bullet;
    fn pointer_to_id(&self, val: *mut bw::Bullet) -> Result<u32, SaveError> {
        self.id(val)
    }
}

impl entity_serialize::LoadEntityPointer for LoadMapping<bw::Bullet> {
    type Pointer = bw::Bullet;
    fn id_to_pointer(&self, val: u32) -> Result<*mut bw::Bullet, LoadError> {
        self.pointer(val)
    }
}

pub unsafe fn create_bullet(
    parent: *mut bw::Unit,
    bullet_id: u32,
    x: u32,
    y: u32,
    player: u32,
    direction: u32,
    orig: &Fn(*mut bw::Unit, u32, u32, u32, u32, u32) -> *mut bw::Bullet,
) -> *mut bw::Bullet {
    // Bullet count is only used to limit valkyries, so faking it to be 0 is fine
    *bw::bullet_count = 0;
    // Could set spread seed so it's not always 0
    let bullet = Box::new(bw::Bullet {
        ..mem::zeroed()
    });
    let bullet = Box::into_raw(bullet);
    *bw::first_free_bullet = bullet;
    *bw::last_free_bullet = bullet;
    let actual_bullet = orig(parent, bullet_id, x, y, player, direction);
    *bw::first_free_bullet = null_mut();
    *bw::last_free_bullet = null_mut();
    if actual_bullet == null_mut() {
        info!(
            "Couldn't create bullet {:x} at {:x}.{:x} facing {:x}", bullet_id, x, y, direction
        );
        Box::from_raw(bullet);
        return null_mut();
    } else if actual_bullet != bullet {
        error!(
            "Created a different bullet from what was expected: {:p} {:p}",
            bullet,
            actual_bullet
        );
    }
    let mut bullets = all_bullets().borrow_mut();
    bullets.insert(bullet.into());
    bullet
}

pub unsafe fn delete_bullet(bullet: *mut bw::Bullet, orig: &Fn(*mut bw::Bullet)) {
    if (*bullet).entity.sprite == null_mut() {
        // Have to call orig to remove the bullet from active bullet list
        *bw::first_free_bullet = null_mut();
        *bw::last_free_bullet = null_mut();
        orig(bullet);
        Box::from_raw(bullet);
        let mut bullets = all_bullets().borrow_mut();
        bullets.remove(&bullet.into());
    }
}

pub unsafe fn delete_all() {
    let mut bullets = all_bullets().borrow_mut();
    for bullet in bullets.iter() {
        Box::from_raw(**bullet);
    }
    bullets.clear();
    // Not sure if these are necessary, but doing this won't hurt either
    *bw::first_active_bullet = null_mut();
    *bw::last_active_bullet = null_mut();
    *bw::first_free_bullet = null_mut();
    *bw::last_free_bullet = null_mut();
}

#[derive(Serialize, Deserialize)]
struct SaveGlobals {
    first_bullet: u32,
    last_bullet: u32,
    bullet_count: u32,
}

#[derive(Serialize, Deserialize)]
struct BulletSerializable {
    entity: EntitySerializable,
    weapon_id: u8,
    death_timer: u8,
    flags: u8,
    bounces_remaining: u8,
    parent: u16,
    previous_bounce_target: u16,
    spread_seed: u8,
}

pub unsafe fn save_bullet_chunk(file: *mut c_void) -> u32 {
    if let Err(e) = save_bullets(file) {
        error!("Couldn't save bullets: {}", e);
        print_text(&format!("Unable to save the game: {}", e));
        return 0;
    }
    1
}

unsafe fn save_bullets(file: *mut c_void) -> Result<(), SaveError> {
    let data = serialize_bullets()?;
    fwrite_num(file, BULLET_SAVE_MAGIC)?;
    fwrite_num(file, 1u32)?;
    fwrite_num(file, data.len() as u32)?;
    fwrite(file, &data)?;
    Ok(())
}

unsafe fn serialize_bullets() -> Result<Vec<u8>, SaveError> {
    let ptr_to_id_map = bullet_pointer_to_id_map();
    let buf = Vec::with_capacity(0x10000);
    let mut writer = flate2::write::DeflateEncoder::new(buf, flate2::Compression::Default);

    let size_limit = bincode::Bounded(BULLET_SAVE_MAX_SIZE as u64);
    let globals = SaveGlobals {
        first_bullet: ptr_to_id_map.id(*bw::first_active_bullet)?,
        last_bullet: ptr_to_id_map.id(*bw::last_active_bullet)?,
        bullet_count: ptr_to_id_map.len() as u32,
    };
    bincode::serialize_into(&mut writer, &globals, size_limit)?;
    let mut bullet = *bw::first_active_bullet;
    while bullet != null_mut() {
        let serializable = bullet_serializable(bullet, &ptr_to_id_map)?;
        bincode::serialize_into(&mut writer, &serializable, size_limit)?;
        bullet = (*bullet).entity.next as *mut bw::Bullet;
        if writer.total_in() > BULLET_SAVE_MAX_SIZE as u64{
            return Err(SaveError::SizeLimit(writer.total_in()));
        }
        // Could also check total out but it should be lower..
    }
    Ok(writer.finish()?)
}

unsafe fn bullet_serializable(
    bullet: *const bw::Bullet,
    mapping: &SaveMapping<bw::Bullet>,
) -> Result<BulletSerializable, SaveError> {
    let bw::Bullet {
        ref entity,
        weapon_id,
        death_timer,
        flags,
        bounces_remaining,
        parent,
        previous_bounce_target,
        spread_seed,
        padding6d: _,
    } = *bullet;
    Ok(BulletSerializable {
        entity: entity_serializable(entity, mapping)?,
        weapon_id,
        death_timer,
        flags,
        bounces_remaining,
        parent: unit_to_id(parent),
        previous_bounce_target: unit_to_id(previous_bounce_target),
        spread_seed,
    })
}

fn deserialize_bullet(
    bullet: &BulletSerializable,
    mapping: &LoadMapping<bw::Bullet>,
) -> Result<bw::Bullet, LoadError> {
    let BulletSerializable {
        ref entity,
        weapon_id,
        death_timer,
        flags,
        bounces_remaining,
        parent,
        previous_bounce_target,
        spread_seed,
    } = *bullet;
    Ok(bw::Bullet {
        entity: deserialize_entity(entity, mapping)?,
        weapon_id,
        death_timer,
        flags,
        bounces_remaining,
        parent: unit_from_id(parent)?,
        previous_bounce_target: unit_from_id(previous_bounce_target)?,
        spread_seed,
        padding6d: [0; 3],
    })
}

unsafe fn bullet_pointer_to_id_map() -> SaveMapping<bw::Bullet> {
    let mut id = 1;
    let mut bullet = *bw::first_active_bullet;
    let mut ret = HashMap::new();
    while bullet != null_mut() {
        let old = ret.insert(bullet.into(), id);
        assert!(old.is_none());
        bullet = (*bullet).entity.next as *mut bw::Bullet;
        id += 1;
    }
    SaveMapping(ret)
}

pub unsafe fn load_bullet_chunk(file: *mut c_void, save_version: u32) -> u32 {
    if save_version != 3 {
        error!("Unusupported save version: {}", save_version);
        return 0;
    }
    if let Err(e) = load_bullets(file) {
        info!("Couldn't load a save: {}", e);
        return 0;
    }
    1
}

unsafe fn load_bullets(file: *mut c_void) -> Result<(), LoadError> {
    let magic = fread_num::<u16>(file)?;
    if magic != BULLET_SAVE_MAGIC {
        return Err(LoadError::WrongMagic(magic));
    }
    let version = fread_num::<u32>(file)?;
    if version != 1 {
        return Err(LoadError::Version(version));
    }
    let size = fread_num::<u32>(file)?;
    if size > BULLET_SAVE_MAX_SIZE {
        return Err(LoadError::Corrupted(format!("Bullet chunk size {} is too large", size)));
    }
    let data = fread(file, size)?;
    let mut reader = flate2::read::DeflateDecoder::new(&data[..]);


    let size_limit = bincode::Bounded(BULLET_SAVE_MAX_SIZE as u64);
    let globals: SaveGlobals = bincode::deserialize_from(&mut reader, size_limit)?;
    let (mut bullets, mapping) = allocate_bullets(globals.bullet_count);
    for bullet in &mut bullets {
        let serialized = bincode::deserialize_from(&mut reader, size_limit)?;
        **bullet = deserialize_bullet(&serialized, &mapping)?;
        if reader.total_out() > BULLET_SAVE_MAX_SIZE as u64 {
            return Err(LoadError::SizeLimit)
        }
    }
    let mut bullet_set = all_bullets().borrow_mut();
    for bullet in bullets {
        bullet_set.insert(Box::into_raw(bullet).into());
    }
    *bw::first_active_bullet = mapping.pointer(globals.first_bullet)?;
    *bw::last_active_bullet = mapping.pointer(globals.last_bullet)?;
    Ok(())
}

// Returning the pointer vector isn't really necessary, just simpler. Could also create a
// vector abstraction that allows reading addresses of any Bullet while holding a &mut reference
// to one of them.
fn allocate_bullets(count: u32) -> (Vec<Box<bw::Bullet>>, LoadMapping<bw::Bullet>) {
    (0..count).map(|_| {
        let mut bullet = Box::new(unsafe { mem::zeroed() });
        let pointer: *mut bw::Bullet = &mut *bullet;
        (bullet, pointer)
    }).unzip()
}
