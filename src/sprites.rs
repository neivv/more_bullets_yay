use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::mem;
use std::ptr::null_mut;

use bincode;
use flate2;
use libc::c_void;

use bw;
use save::{fread_num, fread, fwrite, fwrite_num, SaveError, LoadError, print_text};
use save::{SaveMapping, LoadMapping};
use send_pointer::SendPtr;
use units::{unit_to_id, unit_from_id};

const SPRITE_LIMIT: usize = 200000;
const IMAGE_LIMIT: usize = 400000;

ome2_thread_local! {
    SPRITE_ARRAY: RefCell<RawVec<bw::Sprite>> =
        sprite_array(RefCell::new(RawVec::with_capacity(SPRITE_LIMIT)));
    IMAGE_ARRAY: RefCell<RawVec<bw::Image>> =
        image_array(RefCell::new(RawVec::with_capacity(IMAGE_LIMIT)));
    SPRITES: RefCell<HashSet<SendPtr<bw::Sprite>>> = all_sprites(RefCell::new(HashSet::new()));
    // Both lone and fow
    LONE_SPRITES: RefCell<HashSet<SendPtr<bw::LoneSprite>>> =
        all_lone_sprites(RefCell::new(HashSet::new()));
    NEXT_SPRITE_ID: Cell<u64> = next_sprite_id(Cell::new(0));
    SPRITE_DRAW_BUFFER: RefCell<Vec<SendPtr<bw::Sprite>>> =
        sprite_draw_buffer(RefCell::new(Vec::with_capacity(0x800)));
    SPRITE_SAVE_MAPPING: RefCell<SaveMapping<bw::Sprite>> =
        sprite_save_mapping(RefCell::new(SaveMapping::new()));
    SPRITE_LOAD_MAPPING: RefCell<LoadMapping<bw::Sprite>> =
        sprite_load_mapping(RefCell::new(LoadMapping::new()));
    LONE_SPRITE_SAVE_MAPPING: RefCell<SaveMapping<bw::LoneSprite>> =
        lone_sprite_save_mapping(RefCell::new(SaveMapping::new()));
    LONE_SPRITE_LOAD_MAPPING: RefCell<LoadMapping<bw::LoneSprite>> =
        lone_sprite_load_mapping(RefCell::new(LoadMapping::new()));
}

struct RawVec<T> {
    ptr: *mut T,
    size: usize,
    capacity: usize,
}

unsafe impl<T> Send for RawVec<T> {}
unsafe impl<T> Sync for RawVec<T> {}

impl<T> RawVec<T> {
    fn with_capacity(cap: usize) -> RawVec<T> {
        let (ptr, size, capacity) = Vec::with_capacity(cap).into_raw_parts();
        RawVec {
            ptr,
            size,
            capacity,
        }
    }

    fn push(&mut self) -> *mut T {
        unsafe {
            if self.size == self.capacity {
                null_mut()
            } else {
                self.size += 1;
                self.ptr.add(self.size - 1)
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = *mut T> {
        unsafe {
            let ptr = self.ptr;
            (0..self.size).map(move |x| ptr.add(x))
        }
    }
}

const SPRITE_SAVE_MAGIC: u16 = 0xffee;
// 16 megabytes, should be more than enough, both compressed and without.
const SPRITE_SAVE_MAX_SIZE: u32 = 0x1000000;

#[derive(Serialize, Deserialize)]
struct SaveGlobals {
    horizontal_lines: Vec<(u32, u32)>,
    sprite_count: u32,
    lone_count: u32,
    fow_count: u32,
    cursor_marker: u32,
}

#[derive(Serialize, Deserialize)]
struct SpriteSerializable {
    prev: u32,
    next: u32,
    sprite_id: u16,
    player: u8,
    selection_index: u8,
    visibility_mask: u8,
    elevation: u8,
    flags: u8,
    selection_flash_timer: u8,
    index: u16,
    width: u8,
    height: u8,
    position: bw::Point,
    main_image_id: u32,
    images: Vec<ImageSerializable>,
    extra: bw::SpriteExtension,
}

#[derive(Serialize, Deserialize)]
struct ImageSerializable {
    offset: usize,
    image_id: u16,
    drawfunc: u8,
    direction: u8,
    flags: u16,
    x_offset: i8,
    y_offset: i8,
    iscript: bw::Iscript,
    frameset: u16,
    frame: u16,
    map_position: bw::Point,
    screen_position: [i16; 2],
    grp_bounds: [i16; 4],
    grp: u16,
    drawfunc_param: u32,
}

#[derive(Serialize, Deserialize)]
struct LoneSpriteSerializable {
    sprite: u32,
    value: u32,
}

fn allocate_sprite() -> *mut bw::Sprite {
    let mut sprites = sprite_array().borrow_mut();
    sprites.push()
}

fn allocate_image() -> *mut bw::Image {
    let mut array = image_array().borrow_mut();
    array.push()
}

unsafe fn refill_sprite_image_list() {
    let mut sprites = all_sprites().borrow_mut();
    if sprites.len() == 0 {
        // Init
        *bw::first_free_sprite = null_mut();
        *bw::last_free_sprite = null_mut();
        *bw::first_free_image = null_mut();
        *bw::last_free_image = null_mut();
    }
    let sprite_count = BwLinkedListIter(*bw::first_free_sprite).count();
    if sprite_count < 500 {
        for _ in 0..(500 - sprite_count) {
            let sprite = allocate_sprite();
            if sprite.is_null() {
                break;
            }
            *sprite = bw::Sprite {
                next: *bw::first_free_sprite,
                ..mem::zeroed()
            };
            if *bw::first_free_sprite != null_mut() {
                (**bw::first_free_sprite).prev = sprite;
            } else {
                *bw::last_free_sprite = sprite;
            }
            *bw::first_free_sprite = sprite;
            sprites.insert(sprite.into());
        }
    }
    let image_count = BwLinkedListIter(*bw::first_free_image).count();
    if image_count < 1500 {
        for _ in 0..(1500 - image_count) {
            let image = allocate_image();
            if image.is_null() {
                break;
            }
            *image = bw::Image {
                next: *bw::first_free_image,
                ..mem::zeroed()
            };
            if *bw::first_free_image != null_mut() {
                (**bw::first_free_image).prev = image;
            } else {
                *bw::last_free_image = image;
            }
            *bw::first_free_image = image;
        }
    }
}

pub unsafe fn create_sprite(
    sprite_id: u32,
    x: u32,
    y: u32,
    player: u32,
    orig: unsafe extern fn(u32, u32, u32, u32) -> *mut bw::Sprite,
) -> *mut bw::Sprite {
    if *bw::scmain_state != 3 {
        // Lobby minimap preview, since this plugin doesn't reset sprites aren
        // on game init (only end), but BW does, calling the hook is problematic in lobby.
        return orig(sprite_id, x, y, player);
    }
    refill_sprite_image_list();
    let actual_sprite = orig(sprite_id, x, y, player);

    if actual_sprite != null_mut() {
        let cell = next_sprite_id();
        (*actual_sprite).extra.spawn_order = (cell.get() as u32, (cell.get() >> 32) as u32);
        cell.set(cell.get().checked_add(1).unwrap());
    }
    actual_sprite
}

pub unsafe fn create_lone(
    sprite_id: u32,
    x: u32,
    y: u32,
    player: u32,
    orig: unsafe extern fn(u32, u32, u32, u32) -> *mut bw::LoneSprite,
) -> *mut bw::LoneSprite {
    let sprite = Box::new(bw::LoneSprite {
        ..mem::zeroed()
    });
    let sprite = Box::into_raw(sprite);
    *bw::first_free_lone_sprite = sprite;
    *bw::last_free_lone_sprite = sprite;
    let actual_sprite = orig(sprite_id, x, y, player);
    *bw::first_free_lone_sprite = null_mut();
    *bw::last_free_lone_sprite = null_mut();
    if actual_sprite == null_mut() {
        info!( "Couldn't create lone sprite {:x} at {:x}.{:x}", sprite_id, x, y);
        let _ = Box::from_raw(sprite);
        return null_mut();
    } else if actual_sprite != sprite {
        error!(
            "Created a different lone sprite from what was expected: {:p} {:p}",
            sprite,
            actual_sprite,
        );
    }

    let mut sprites = all_lone_sprites().borrow_mut();
    sprites.insert(sprite.into());
    sprite
}

pub unsafe fn create_fow(
    unit_id: u32,
    base: *mut bw::Sprite,
    orig: unsafe extern fn(u32, *mut bw::Sprite) -> *mut bw::LoneSprite,
) -> *mut bw::LoneSprite {
    let sprite = Box::new(bw::LoneSprite {
        ..mem::zeroed()
    });
    let sprite = Box::into_raw(sprite);
    *bw::first_free_fow_sprite = sprite;
    *bw::last_free_fow_sprite = sprite;
    let actual_sprite = orig(unit_id, base);
    *bw::first_free_fow_sprite = null_mut();
    *bw::last_free_fow_sprite = null_mut();
    if actual_sprite == null_mut() {
        info!("Couldn't create fow sprite {:x}", unit_id);
        let _ = Box::from_raw(sprite);
        return null_mut();
    } else if actual_sprite != sprite {
        error!(
            "Created a different fow sprite from what was expected: {:p} {:p}",
            sprite,
            actual_sprite,
        );
    }

    let mut sprites = all_lone_sprites().borrow_mut();
    sprites.insert(sprite.into());
    sprite
}

pub unsafe fn delete_sprite(sprite: *mut bw::Sprite, orig: unsafe extern fn(*mut bw::Sprite)) {
    orig(sprite);
}

pub unsafe fn step_lone_frame(
    sprite: *mut bw::LoneSprite,
    orig: unsafe extern fn(*mut bw::LoneSprite),
) {
    *bw::first_free_lone_sprite = null_mut();
    *bw::last_free_lone_sprite = null_mut();
    orig(sprite);
    if *bw::first_free_lone_sprite == sprite {
        let _ = Box::from_raw(sprite);
        let mut sprites = all_lone_sprites().borrow_mut();
        sprites.remove(&sprite.into());
    }
    *bw::first_free_lone_sprite = null_mut();
    *bw::last_free_lone_sprite = null_mut();
}

pub unsafe fn step_fow_frame(
    sprite: *mut bw::LoneSprite,
    orig: unsafe extern fn(*mut bw::LoneSprite),
) {
    *bw::first_free_fow_sprite = null_mut();
    *bw::last_free_fow_sprite = null_mut();
    orig(sprite);
    if *bw::first_free_fow_sprite == sprite {
        let _ = Box::from_raw(sprite);
        let mut sprites = all_lone_sprites().borrow_mut();
        sprites.remove(&sprite.into());
    }
    *bw::first_free_fow_sprite = null_mut();
    *bw::last_free_fow_sprite = null_mut();
}

pub unsafe fn create_image(orig: unsafe extern fn() -> *mut bw::Image) -> *mut bw::Image {
    orig()
}

pub unsafe fn delete_image(image: *mut bw::Image, orig: unsafe extern fn(*mut bw::Image)) {
    orig(image);
}

pub unsafe fn delete_all() {
    let mut sprites = all_sprites().borrow_mut();
    sprites.clear();
    sprite_array().borrow_mut().size = 0;
    image_array().borrow_mut().size = 0;
}

unsafe fn is_selection_image(image: *mut bw::Image) -> bool {
    ((*image).image_id >= 0x231 && (*image).image_id <= 0x23a) || (*image).drawfunc == 0xb
}

pub unsafe fn add_to_drawn_sprites(sprite: *mut bw::Sprite) {
    let mut buf = sprite_draw_buffer().borrow_mut();
    buf.push(sprite.into());
    sprite_vision_sync(sprite);
}

unsafe fn sprite_vision_sync(sprite: *mut bw::Sprite) {
    use std::cmp::{max, min};
    let sync = *(*bw::sprite_include_in_vision_sync).offset((*sprite).sprite_id as isize);
    if sync != 0 {
        let y_tile = min(max((*sprite).position.y / 32, 0), *bw::map_height_tiles as i16);
        bw::sync_horizontal_lines[y_tile as usize] ^= *bw::player_visions as u8;
    }
}

pub unsafe fn draw_sprites() {
    let mut buf = sprite_draw_buffer().borrow_mut();
    buf.sort_by(|&SendPtr(a), &SendPtr(b)| {
        use std::cmp::Ordering;
        match (*a).elevation.cmp(&(*b).elevation) {
            Ordering::Equal => (),
            x => return x,
        }
        // Ground units are sorted by y position
        if (*a).elevation <= 4 {
            match (*a).position.y.cmp(&(*b).position.y) {
                Ordering::Equal => (),
                x => return x,
            }
        }
        match ((*a).flags & 0x10).cmp(&((*b).flags & 0x10)) {
            Ordering::Equal => (),
            x => return x,
        }
        (*a).extra.spawn_order.cmp(&(*b).extra.spawn_order)
    });
    for &SendPtr(sprite) in buf.iter() {
        bw::draw_sprite(sprite);
    }
}

pub unsafe fn redraw_screen_hook(orig: unsafe extern fn()) {
    orig();
    // The buffer may be filled but not used if the screen doesn't need to be redrawn.
    let mut buf = sprite_draw_buffer().borrow_mut();
    buf.clear();
}

pub unsafe fn save_sprite_chunk(file: *mut c_void) -> u32 {
    if let Err(e) = save_sprites(file) {
        error!("Couldn't save sprites: {}", e);
        print_text(&format!("Unable to save the game: {}", e));
        return 0;
    }
    1
}

unsafe fn save_sprites(file: *mut c_void) -> Result<(), SaveError> {
    let data = serialize_sprites()?;
    fwrite_num(file, SPRITE_SAVE_MAGIC)?;
    fwrite_num(file, 1u32)?;
    fwrite_num(file, data.len() as u32)?;
    fwrite(file, &data)?;
    Ok(())
}

unsafe fn serialize_sprites() -> Result<Vec<u8>, SaveError> {
    let ptr_to_id_map = sprite_pointer_to_id_map();
    let lone_ptr_to_id_map = lone_sprite_pointer_to_id_map();

    let buf = Vec::with_capacity(0x10000);
    let mut writer = flate2::write::DeflateEncoder::new(buf, flate2::Compression::Default);

    let size_limit = bincode::Bounded(SPRITE_SAVE_MAX_SIZE as u64);
    let horizontal_lines = (0..*bw::map_height_tiles as usize).map(|i| {
        Ok((ptr_to_id_map.id(bw::horizontal_sprite_lines_begin[i])?,
            ptr_to_id_map.id(bw::horizontal_sprite_lines_end[i])?))
    }).collect::<Result<Vec<_>, SaveError>>()?;
    let globals = SaveGlobals {
        horizontal_lines,
        sprite_count: ptr_to_id_map.len() as u32,
        lone_count: lone_ptr_to_id_map.len() as u32,
        fow_count: lone_sprites(*bw::first_active_fow_sprite).count() as u32,
        cursor_marker: lone_ptr_to_id_map.id(*bw::cursor_marker)?,
    };
    bincode::serialize_into(&mut writer, &globals, size_limit)?;
    {
        let sprites = sprite_array().borrow_mut();
        for sprite in sprites.iter() {
            let serializable = sprite_serializable(sprite, &ptr_to_id_map)?;
            bincode::serialize_into(&mut writer, &serializable, size_limit)?;
            if writer.total_in() > SPRITE_SAVE_MAX_SIZE as u64 {
                return Err(SaveError::SizeLimit(writer.total_in()));
            }
        }
    }
    for sprite in lone_sprites(*bw::first_active_lone_sprite) {
        let serializable = lone_sprite_serializable(sprite, &ptr_to_id_map)?;
        bincode::serialize_into(&mut writer, &serializable, size_limit)?;
        if writer.total_in() > SPRITE_SAVE_MAX_SIZE as u64 {
            return Err(SaveError::SizeLimit(writer.total_in()));
        }
    }
    for sprite in lone_sprites(*bw::first_active_fow_sprite) {
        let serializable = lone_sprite_serializable(sprite, &ptr_to_id_map)?;
        bincode::serialize_into(&mut writer, &serializable, size_limit)?;
        if writer.total_in() > SPRITE_SAVE_MAX_SIZE as u64 {
            return Err(SaveError::SizeLimit(writer.total_in()));
        }
    }

    let mut global_mapping = sprite_save_mapping().borrow_mut();
    *global_mapping = ptr_to_id_map;
    let mut lone_global = lone_sprite_save_mapping().borrow_mut();
    *lone_global = lone_ptr_to_id_map;
    Ok(writer.finish()?)
}

unsafe fn sprite_pointer_to_id_map() -> SaveMapping<bw::Sprite> {
    let sprites = sprite_array().borrow_mut();
    sprites.iter().enumerate().map(|(x, y)| (y.into(), x as u32 + 1)).collect()
}

unsafe fn lone_sprites(ptr: *mut bw::LoneSprite) -> BwLinkedListIter<bw::LoneSprite> {
    BwLinkedListIter(ptr)
}

struct BwLinkedListIter<T: BwLinkedListObject>(*mut T);

trait BwLinkedListObject {
    unsafe fn next(val: *mut Self) -> *mut Self;
}

impl BwLinkedListObject for bw::Sprite {
    unsafe fn next(val: *mut bw::Sprite) -> *mut bw::Sprite {
        (*val).next
    }
}

impl BwLinkedListObject for bw::LoneSprite {
    unsafe fn next(val: *mut bw::LoneSprite) -> *mut bw::LoneSprite {
        (*val).next
    }
}

impl BwLinkedListObject for bw::Image {
    unsafe fn next(val: *mut bw::Image) -> *mut bw::Image {
        (*val).next
    }
}

impl<T: BwLinkedListObject> Iterator for BwLinkedListIter<T> {
    type Item = *mut T;
    fn next(&mut self) -> Option<*mut T> {
        unsafe {
            let val = self.0;
            if val != null_mut() {
                self.0 = T::next(val);
                Some(val)
            } else {
                None
            }
        }
    }
}

unsafe fn lone_sprite_pointer_to_id_map() -> SaveMapping<bw::LoneSprite> {
    lone_sprites(*bw::first_active_lone_sprite)
        .enumerate()
        .map(|(x, y)| (y.into(), x as u32 + 1))
        .collect()
}
pub fn sprite_to_id_current_mapping(sprite: *mut bw::Sprite) -> Result<u32, SaveError> {
    let mapping = sprite_save_mapping().borrow();
    mapping.id(sprite)
}

pub fn sprite_from_id_current_mapping(id: u32) -> Result<*mut bw::Sprite, LoadError> {
    let mapping = sprite_load_mapping().borrow();
    mapping.pointer(id)
}

pub fn lone_sprite_to_id_current_mapping(sprite: *mut bw::LoneSprite) -> Result<u32, SaveError> {
    let mapping = lone_sprite_save_mapping().borrow();
    mapping.id(sprite)
}

pub fn lone_sprite_from_id_current_mapping(id: u32) -> Result<*mut bw::LoneSprite, LoadError> {
    let mapping = lone_sprite_load_mapping().borrow();
    mapping.pointer(id)
}

unsafe fn sprite_serializable(
    sprite: *const bw::Sprite,
    mapping: &SaveMapping<bw::Sprite>,
) -> Result<SpriteSerializable, SaveError> {
    let bw::Sprite {
        prev,
        next,
        sprite_id,
        player,
        selection_index,
        visibility_mask,
        elevation,
        flags,
        selection_flash_timer,
        index,
        width,
        height,
        position,
        main_image,
        first_overlay,
        last_overlay: _,
        extra,
    } = *sprite;
    let (images, main_image_id) = images_serializable(first_overlay, main_image)?;
    Ok(SpriteSerializable {
        prev: mapping.id(prev)?,
        next: mapping.id(next)?,
        sprite_id,
        player,
        selection_index,
        visibility_mask,
        elevation,
        // Selection overlays are not saved
        flags: flags & !(0x1 | 0x8),
        selection_flash_timer,
        index,
        width,
        height,
        position,
        images,
        main_image_id,
        extra,
    })
}

unsafe fn images_serializable(
    first: *mut bw::Image,
    main_image: *mut bw::Image
) -> Result<(Vec<ImageSerializable>, u32), SaveError> {
    let mut out = Vec::new();
    let mut main_index = 0;
    let mut image = first;
    // It's possible for the main image to be unreachable and dead value.
    let mut index = 0;
    while image != null_mut() {
        if !is_selection_image(image) {
            index += 1;
            let bw::Image {
                prev: _,
                next: _,
                image_id,
                drawfunc,
                direction,
                flags,
                x_offset,
                y_offset,
                iscript,
                frameset,
                frame,
                map_position,
                screen_position,
                grp_bounds,
                grp,
                drawfunc_param,
                draw: _,
                step_frame: _,
                parent: _,
            } = *image;
            if image == main_image {
                main_index = index;
            }
            let offset = image as usize - image_array().borrow().ptr as usize;
            out.push(ImageSerializable {
                offset,
                image_id,
                drawfunc,
                direction,
                flags,
                x_offset,
                y_offset,
                iscript,
                frameset,
                frame,
                map_position,
                screen_position,
                grp_bounds,
                grp: grp_to_id(grp)?,
                drawfunc_param: drawfunc_param_serializable(drawfunc, drawfunc_param)?,
            });
        }
        image = (*image).next;
    }
    Ok((out, main_index))
}

unsafe fn lone_sprite_serializable(
    sprite: *const bw::LoneSprite,
    mapping: &SaveMapping<bw::Sprite>
) -> Result<LoneSpriteSerializable, SaveError> {
    Ok(LoneSpriteSerializable {
        sprite: mapping.id((*sprite).sprite)?,
        value: (*sprite).value,
    })
}

unsafe fn grp_to_id(grp: *mut bw::GrpSprite) -> Result<u16, SaveError> {
    if grp == null_mut() {
        Ok(0)
    } else {
        (0..image_count()).position(|i| *(*bw::image_grps).offset(i as isize) == grp)
            .map(|x| x as u16 + 1)
            .ok_or(SaveError::InvalidGrpPointer)
    }
}

unsafe fn image_count() -> u32 {
    (*bw::image_count_part1).checked_add(*bw::image_count_part2).unwrap()
}

unsafe fn grp_from_id(grp: u16) -> Result<*mut bw::GrpSprite, LoadError> {
    if grp == 0 {
        Ok(null_mut())
    } else if grp as u32 - 1 < image_count() {
        Ok(*(*bw::image_grps).offset(grp as isize - 1))
    } else {
        Err(LoadError::Corrupted(format!("Invalid grp {}", grp)))
    }
}

unsafe fn drawfunc_param_serializable(func: u8, param: *mut c_void) -> Result<u32, SaveError> {
    match func {
        0x9 => {
            let param = param as *const u8;
            bw::remap_palettes.iter()
                .position(|palette| palette.data == param)
                .map(|x| x as u32 + 1)
                .ok_or(SaveError::InvalidRemapPalette)
        }
        0xb => Ok(unit_to_id(param as *mut bw::Unit) as u32),
        _ => Ok(param as u32),
    }
}

unsafe fn deserialize_drawfunc_param(func: u8, param: u32) -> Result<*mut c_void, LoadError> {
    match func {
        0x9 => {
            if param == 0 {
                Ok(null_mut())
            } else if param as usize - 1 < bw::remap_palettes.len() {
                let pointer = bw::remap_palettes[param as usize - 1].data;
                Ok(pointer as *mut c_void)
            } else {
                Err(LoadError::Corrupted(format!("Invalid remap palette {}", param)))
            }
        }
        0xb => Ok(unit_from_id(param as u16)? as *mut c_void),
        _ => Ok(param as *mut c_void),
    }
}

pub unsafe fn load_sprite_chunk(file: *mut c_void) -> u32 {
    if let Err(e) = load_sprites(file) {
        info!("Couldn't load a save: {}", e);
        return 0;
    }
    1
}

unsafe fn load_sprites(file: *mut c_void) -> Result<(), LoadError> {
    let magic = fread_num::<u16>(file)?;
    if magic != SPRITE_SAVE_MAGIC {
        return Err(LoadError::WrongMagic(magic));
    }
    let version = fread_num::<u32>(file)?;
    if version != 1 {
        return Err(LoadError::Version(version));
    }
    let size = fread_num::<u32>(file)?;
    if size > SPRITE_SAVE_MAX_SIZE {
        return Err(LoadError::Corrupted(format!("Sprite chunk size {} is too large", size)));
    }
    let data = fread(file, size)?;
    let mut reader = flate2::read::DeflateDecoder::new(&data[..]);

    let size_limit = bincode::Bounded(SPRITE_SAVE_MAX_SIZE as u64);
    let globals: SaveGlobals = bincode::deserialize_from(&mut reader, size_limit)?;
    let mapping;
    let lone_mapping;
    let mut lone_sprites;
    {
        let mut sprites = sprite_array().borrow_mut();
        let mut images = image_array().borrow_mut();
        mapping = allocate_sprites(&mut sprites, globals.sprite_count);
        let (lone_sprites_, lone_mapping_) =
            allocate_lone_sprites(globals.lone_count + globals.fow_count);
        lone_sprites = lone_sprites_;
        lone_mapping = lone_mapping_;
        for sprite_result in sprites.iter() {
            let serialized = bincode::deserialize_from(&mut reader, size_limit)?;
            let sprite = deserialize_sprite(&serialized, &mapping, sprite_result, &mut images)?;
            *sprite_result = sprite;
            if reader.total_out() > SPRITE_SAVE_MAX_SIZE as u64 {
                return Err(LoadError::SizeLimit)
            }
        }

        for lone_sprite_result in &mut lone_sprites {
            let serialized = bincode::deserialize_from(&mut reader, size_limit)?;
            let sprite = deserialize_lone_sprite(&serialized, &mapping)?;
            **lone_sprite_result = sprite;
            if reader.total_out() > SPRITE_SAVE_MAX_SIZE as u64 {
                return Err(LoadError::SizeLimit)
            }
        }
        for i in 0..lone_sprites.len() {
            if i != 0 && i != globals.lone_count as usize {
                lone_sprites[i - 1].next = &mut *lone_sprites[i];
            }
            if i != globals.lone_count as usize - 1 && i != lone_sprites.len() - 1 {
                lone_sprites[i + 1].prev = &mut *lone_sprites[i];
            }
        }

        let mut sprite_set = all_sprites().borrow_mut();
        for sprite in sprites.iter() {
            sprite_set.insert(SendPtr(sprite));
        }
    }

    *bw::first_free_sprite = null_mut();
    *bw::last_free_sprite = null_mut();
    *bw::first_free_image = null_mut();
    *bw::last_free_image = null_mut();

    *bw::first_active_lone_sprite = match globals.lone_count {
        0 => null_mut(),
        _ => &mut *lone_sprites[0],
    };
    *bw::last_active_lone_sprite = match globals.lone_count {
        0 => null_mut(),
        _ => &mut *lone_sprites[globals.lone_count as usize - 1],
    };
    *bw::first_active_fow_sprite = match globals.fow_count {
        0 => null_mut(),
        _ => &mut *lone_sprites[globals.lone_count as usize],
    };
    *bw::last_active_fow_sprite = match globals.fow_count {
        0 => null_mut(),
        _ => &mut **lone_sprites.last_mut().unwrap(),
    };

    {
        let mut lone_sprite_set = all_lone_sprites().borrow_mut();
        for sprite in lone_sprites {
            lone_sprite_set.insert(Box::into_raw(sprite).into());
        }
    }

    for (i, (begin, end)) in globals.horizontal_lines.into_iter().enumerate() {
        bw::horizontal_sprite_lines_begin[i] = mapping.pointer(begin)?;
        bw::horizontal_sprite_lines_end[i] = mapping.pointer(end)?;
    }
    *bw::cursor_marker = lone_mapping.pointer(globals.cursor_marker)?;

    {
        let mut global_mapping = sprite_load_mapping().borrow_mut();
        *global_mapping = mapping;
        let mut lone_global_mapping = lone_sprite_load_mapping().borrow_mut();
        *lone_global_mapping = lone_mapping;
    }
    // Refill sprite / image list for GPTP which allocates images from reading through
    // first_free_image during hooks.
    // Most of the time the refill_sprite_image_list at create_sprite_hook is good enough,
    // but loading a save may cause something else that allocates images to run before
    // any sprites are created.
    refill_sprite_image_list();
    Ok(())
}

unsafe fn deserialize_sprite(
    sprite: &SpriteSerializable,
    mapping: &LoadMapping<bw::Sprite>,
    pointer: *mut bw::Sprite,
    image_array: &mut RawVec<bw::Image>,
) -> Result<bw::Sprite, LoadError> {
    let SpriteSerializable {
        prev,
        next,
        sprite_id,
        player,
        selection_index,
        visibility_mask,
        elevation,
        flags,
        selection_flash_timer,
        index,
        width,
        height,
        position,
        ref images,
        main_image_id,
        ref extra,
    } = *sprite;
    let mut image_ptrs = deserialize_images(images, pointer, image_array)?;
    Ok(bw::Sprite {
        prev: mapping.pointer(prev)?,
        next: mapping.pointer(next)?,
        sprite_id,
        player,
        selection_index,
        visibility_mask,
        elevation,
        flags,
        selection_flash_timer,
        index,
        width,
        height,
        position,
        first_overlay: image_ptrs.first_mut()
            .map(|x| &mut **x as *mut bw::Image).unwrap_or(null_mut()),
        last_overlay: image_ptrs.last_mut()
            .map(|x| &mut **x as *mut bw::Image).unwrap_or(null_mut()),
        main_image: if main_image_id == 0 {
            null_mut()
        } else {
            image_ptrs.get_mut(main_image_id as usize - 1).map(|x| &mut **x).ok_or_else(|| {
                LoadError::Corrupted(format!("Invalid main image 0x{:x}", main_image_id))
            })?
        },
        extra: extra.clone(),
    })
}

unsafe fn deserialize_images(
    images: &[ImageSerializable],
    parent: *mut bw::Sprite,
    image_array: &mut RawVec<bw::Image>,
) -> Result<Vec<*mut bw::Image>, LoadError> {
    let mut result: Vec<*mut bw::Image> = Vec::with_capacity(images.len());
    for img in images {
        let ImageSerializable {
            offset,
            image_id,
            drawfunc,
            direction,
            flags,
            x_offset,
            y_offset,
            ref iscript,
            frameset,
            frame,
            map_position,
            screen_position,
            grp_bounds,
            grp,
            drawfunc_param,
        } = *img;
        let ptr = (image_array.ptr as usize + offset) as *mut bw::Image;
        let index = offset / mem::size_of::<bw::Image>();
        if index >= image_array.size {
            image_array.size = index + 1;
        }
        *ptr = bw::Image {
            prev: result.last().copied().unwrap_or(null_mut()),
            next: null_mut(),
            image_id,
            drawfunc,
            direction,
            flags,
            x_offset,
            y_offset,
            iscript: iscript.clone(),
            frameset,
            frame,
            map_position,
            screen_position,
            grp_bounds,
            grp: grp_from_id(grp)?,
            drawfunc_param: deserialize_drawfunc_param(drawfunc, drawfunc_param)?,
            parent,
            draw: {
                let drawfunc = bw::image_drawfuncs.get(drawfunc as usize)
                    .unwrap_or_else(|| &bw::image_drawfuncs[0]);
                match flags & 0x2 == 0 {
                    true => drawfunc.normal,
                    false => drawfunc.flipped,
                }
            },
            step_frame: {
                let func = bw::image_updatefuncs.get(drawfunc as usize)
                    .unwrap_or_else(|| &bw::image_updatefuncs[0]);
                func.func
            }
        };
        if let Some(prev) = result.last() {
            (**prev).next = ptr;
        }
        result.push(ptr);
    }
    Ok(result)
}

unsafe fn deserialize_lone_sprite(
    sprite: &LoneSpriteSerializable,
    mapping: &LoadMapping<bw::Sprite>,
) -> Result<bw::LoneSprite, LoadError> {
    Ok(bw::LoneSprite {
        prev: null_mut(),
        next: null_mut(),
        value: sprite.value,
        sprite: mapping.pointer(sprite.sprite)?,
    })
}

// Returning the pointer vector isn't really necessary, just simpler. Could also create a
// vector abstraction that allows reading addresses of any Bullet while holding a &mut reference
// to one of them.
fn allocate_sprites(sprites: &mut RawVec<bw::Sprite>, count: u32) -> LoadMapping<bw::Sprite> {
    sprites.size = count as usize;
    LoadMapping((0..count).map(|i| {
        unsafe { SendPtr(sprites.ptr.add(i as usize)) }
    }).collect())
}

fn allocate_lone_sprites(count: u32) -> (Vec<Box<bw::LoneSprite>>, LoadMapping<bw::LoneSprite>) {
    (0..count).map(|_| {
        let mut sprite = Box::new(unsafe { mem::zeroed() });
        let pointer: *mut bw::LoneSprite = &mut *sprite;
        (sprite, pointer)
    }).unzip()
}
