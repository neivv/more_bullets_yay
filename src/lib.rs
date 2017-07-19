#![cfg_attr(target_env = "gnu", feature(link_args))]
#[cfg_attr(target_env = "gnu", link_args = "-static-libgcc")]
extern {}
#[macro_use]
extern crate whack;

extern crate libc;
extern crate byteorder;
#[macro_use] extern crate log;
extern crate fern;
extern crate chrono;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
extern crate bincode;
extern crate flate2;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate winapi;
extern crate thread_local;

extern crate bw_dat as dat;

#[macro_use] mod macros;
pub mod mpqdraft;

mod bullets;
mod bw;
mod entity_serialize;
mod save;
mod send_pointer;
mod sprites;
mod units;

use std::ptr::null_mut;

fn init() {
    let _ = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("{}[{}:{}][{}] {}",
                chrono::Local::now()
                    .format("[%Y-%m-%d][%H:%M:%S]"),
                record.location().file(),
                record.location().line(),
                record.level(),
                message))
        })
        .level(log::LogLevelFilter::Trace)
        .chain(fern::log_file("more_bullets_yay.log").unwrap())
        .apply();
    std::panic::set_hook(Box::new(|info| {
        match info.location() {
            Some(s) => error!("Panic at {}:{}", s.file(), s.line()),
            None => error!("Panic at unknown location")
        }
        match info.payload().downcast_ref::<&str>() {
            Some(s) => error!("Panic payload:\n{}", s),
            None => error!("Unknown panic payload"),
        }
    }));

    patch();
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern fn Initialize() {
    init();
}

lazy_static! {
    static ref PATCHER: whack::Patcher = whack::Patcher::new();
}

fn patch() {
    unsafe {
        let mut active_patcher = PATCHER.lock().unwrap();

        {
            let mut exe = active_patcher.patch_exe(0x00400000);
            dat::init(&mut exe);

            bw::init_funcs(&mut exe);
            bw::init_funcs_cdecl(&mut exe);
            bw::init_vars(&mut exe);

            exe.hook_opt(bw::CreateBullet, bullets::create_bullet);
            exe.hook_opt(bw::DeleteBullet, bullets::delete_bullet);
            exe.hook(bw::SaveBulletChunk, bullets::save_bullet_chunk);
            exe.hook(bw::LoadBulletChunk, bullets::load_bullet_chunk);

            exe.hook_opt(bw::CreateSprite, sprites::create_sprite);
            exe.hook_opt(bw::DeleteSprite, sprites::delete_sprite);
            exe.hook(bw::AddToDrawnSpriteHeap, sprites::add_to_drawn_sprites);
            exe.hook(bw::DrawSprites, sprites::draw_sprites);
            exe.hook_opt(bw::RedrawScreen, sprites::redraw_screen_hook);
            exe.hook_opt(bw::GetEmptyImage, sprites::create_image);
            exe.hook_opt(bw::DeleteImage, sprites::delete_image);
            exe.hook(bw::SaveSpriteChunk, sprites::save_sprite_chunk);
            exe.hook(bw::LoadSpriteChunk, sprites::load_sprite_chunk);
            // Images are saved with their sprites now.
            exe.hook_closure(bw::SaveImageChunk, |_, _: &Fn(_) -> _| 1);
            exe.hook_closure(bw::LoadImageChunk, |_, _: &Fn(_) -> _| 1);
            exe.hook_closure(bw::SaveLoneSpriteChunk, |_, _, _, _: &Fn(_, _, _) -> _| 1);
            exe.hook_closure(bw::LoadNonFlingySpriteChunk, |_, _: &Fn(_) -> _| 1);
            exe.hook_opt(bw::CreateLoneSprite, sprites::create_lone);
            exe.hook_opt(bw::CreateFowSprite, sprites::create_fow);
            exe.hook_opt(bw::StepLoneSpriteFrame, sprites::step_lone_frame);
            exe.hook_opt(bw::StepFowSpriteFrame, sprites::step_fow_frame);

            exe.hook(bw::SaveUnitChunk, units::save_unit_chunk);
            exe.hook(bw::LoadUnitChunk, units::load_unit_chunk);

            exe.call_hook(bw::GameEnd, bullets::delete_all);
            exe.call_hook(bw::GameEnd, sprites::delete_all);

            exe.replace_val(bw::TooltipSurfaceBytes, 0xa0u32 * 480);
            exe.replace_val(bw::TooltipSurfaceHeight, 480u16);
            exe.replace_val(bw::TooltipTextSurfaceBytes, 0xa0u32 * 480);
            exe.replace_val(bw::TooltipTextSurfaceHeight, 480u16);

            exe.hook(bw::LoadMapPlayerColors, load_map_player_colors);
        }

        {
            let mut storm = active_patcher.patch_library("storm.dll", 0x15000000);
            bw::storm::init_funcs(&mut storm);
        }
    }
}

unsafe fn load_map_player_colors(buf: *const u8, length: u32) {
    let tminimap_pcx = match storm_load_pcx("game\\tminimap.pcx") {
        Some(s) => s.0,
        None => return,
    };
    let tunit_pcx = match storm_load_pcx("game\\tunit.pcx") {
        Some(s) => s.0,
        None => return,
    };

    let player_colors = std::slice::from_raw_parts(buf, length as usize);
    for (player, &scx_color) in player_colors.iter().enumerate() {
        let minimap_color = tminimap_pcx.get(scx_color as usize).cloned().unwrap_or(0);
        bw::player_minimap_color[player as usize] = minimap_color;
        bw::player_color_palette[player as usize] = [0; 8];
        let unit_colors = tunit_pcx.get(8 * scx_color as usize..).into_iter()
            .flat_map(|slice| slice.iter().take(8));
        for (i, &color) in unit_colors.enumerate() {
            bw::player_color_palette[player as usize][i] = color;
        }
    }
}

unsafe fn storm_load_pcx(filename: &str) -> Option<(Vec<u8>, u32, u32)> {
    std::ffi::CString::new(filename).ok().and_then(|filename| {
        let mut width = 0u32;
        let mut height = 0u32;
        let ok = bw::storm::SBmpLoadImage(
            filename.as_ptr() as *const u8,
            null_mut(),
            null_mut(),
            0,
            &mut width,
            &mut height,
            0
        );
        if ok == 0 {
            return None;
        }
        let capacity = (width * height) as usize;
        let mut buf = Vec::with_capacity(capacity);
        let ok = bw::storm::SBmpLoadImage(
            filename.as_ptr() as *const u8,
            null_mut(),
            buf.as_mut_ptr(),
            capacity as u32,
            null_mut(),
            null_mut(),
            0,
        );
        buf.set_len(capacity);
        if ok == 0 {
            None
        } else {
            Some((buf, width, height))
        }
    })
}
