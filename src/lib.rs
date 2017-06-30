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

#[macro_use] mod macros;

mod bullets;
mod bw;
mod entity_serialize;
pub mod mpqdraft;
mod send_pointer;

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

pub fn patch() {
    unsafe {
        let mut active_patcher = PATCHER.lock().unwrap();
        let mut exe = active_patcher.patch_exe(0x00400000);
        bw::init_funcs(&mut exe);
        bw::init_funcs_cdecl(&mut exe);
        bw::init_vars(&mut exe);

        exe.hook_opt(bw::CreateBullet, bullets::create_bullet);
        exe.hook_opt(bw::DeleteBullet, bullets::delete_bullet);
        exe.hook(bw::SaveBulletChunk, bullets::save_bullet_chunk);
        exe.hook(bw::LoadBulletChunk, bullets::load_bullet_chunk);
        exe.call_hook(bw::GameEnd, bullets::delete_all);
    }
}
