#![allow(non_upper_case_globals)]

use libc::c_void;

pub mod structs;

pub use self::structs::*;

whack_hooks!(stdcall, 0x00400000,
    0x0048C260 => CreateBullet(@eax *mut Unit, @ecx u32, u32, u32, u32, u32) -> *mut Bullet;
    0x0048A560 => DeleteBullet(@eax *mut Bullet);
    0x0048AEB0 => SaveBulletChunk(*mut c_void) -> u32;
    0x0048AE40 => LoadBulletChunk(@edx *mut c_void, u32) -> u32;
    0x004EE8C0 => GameEnd();
);

whack_funcs!(stdcall, init_funcs, 0x00400000,
    0x0048D1C0 => print_text(*const u8, u32, @eax u32);
);

whack_funcs!(init_funcs_cdecl, 0x00400000,
    0x004117DE => fread(*mut c_void, u32, u32, *mut c_void) -> i32;
    0x00411931 => fwrite(*const c_void, u32, u32, *mut c_void) -> i32;
);

whack_vars!(init_vars, 0x00400000,
    0x0064EED8 => first_free_bullet: *mut Bullet;
    0x0064EEDC => last_free_bullet: *mut Bullet;
    0x0064DEBC => bullet_count: u32;
    0x0064DEC4 => first_active_bullet: *mut Bullet;
    0x0064DEAC => last_active_bullet: *mut Bullet;

    0x0059CCA8 => units: [Unit; 0x6a4];
    0x00629D98 => sprites: [Sprite; 0x9c4];
);
