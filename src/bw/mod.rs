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

    0x004990F0 => CreateSprite(u32, u32, @edi u32, u32) -> *mut Sprite;
    0x00497B40 => DeleteSprite(@edi *mut Sprite);
    0x0042D4C0 => AddToDrawnSpriteHeap(@esi *mut Sprite);
    0x00498D40 => DrawSprites();
    0x0041CA00 => RedrawScreen();
    0x004D4E30 => GetEmptyImage() -> *mut Image;
    0x004D4CE0 => DeleteImage(@esi *mut Image);
    0x00498740 => SaveSpriteChunk(*mut c_void) -> u32;
    0x004D64C0 => SaveImageChunk(*mut c_void) -> u32;
    0x00498570 => LoadSpriteChunk(*mut c_void) -> u32;
    0x004D6220 => LoadImageChunk(*mut c_void) -> u32;

    0x004EAAF0 => SaveUnitChunk(*mut c_void) -> u32;
    0x0049E910 => LoadUnitChunk(@ebx *mut c_void, u32) -> u32;
    0x00487EC0 => SaveLoneSpriteChunk(*mut c_void, *const LoneSprite, u32) -> u32;
    0x00488100 => LoadNonFlingySpriteChunk(@esi *mut c_void) -> u32;

    0x00488210 => CreateLoneSprite(u32, u32, @edi u32, u32) -> *mut LoneSprite;
    0x00488410 => CreateFowSprite(u32, *mut Sprite) -> *mut LoneSprite;
    0x00488020 => StepLoneSpriteFrame(@edi *mut LoneSprite);
    0x00488350 => StepFowSpriteFrame(@ebx *mut LoneSprite);

    0x0049B130 => LoadMapPlayerColors(*const u8, @eax u32);
);

whack_funcs!(stdcall, init_funcs, 0x00400000,
    0x0048D1C0 => print_text(*const u8, u32, @eax u32);
    0x00498C50 => draw_sprite(@eax *mut Sprite);

    0x00453300 => add_to_repulse_chunk(@esi *mut Unit);
    0x0042D9A0 => check_unstack(@eax *mut Unit);
    0x00469F60 => set_building_tile_flag(@eax *mut Unit, u32, u32);
    0x0046A3A0 => add_to_pos_search(@esi *mut Unit);
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

    0x0063FE30 => first_free_sprite: *mut Sprite;
    0x0063FE34 => last_free_sprite: *mut Sprite;
    0x0057EB68 => first_free_image: *mut Image;
    0x0057EB70 => last_free_image: *mut Image;

    0x0059CCA8 => units: [Unit; 0x6a4];

    0x0042D517 => sprite_include_in_vision_sync: *mut u8;
    0x0057F1D6 => map_height_tiles: u16;
    0x00629C90 => sync_horizontal_lines: [u8; 0x100];
    0x0057F0B0 => player_visions: u32;

    0x00629288 => horizontal_sprite_lines_end: [*mut Sprite; 0x100];
    0x00629688 => horizontal_sprite_lines_begin: [*mut Sprite; 0x100];
    0x004D7268 => image_grps: *mut *mut GrpSprite;
    0x004D6352 => image_count_part1: u32;
    0x004D6357 => image_count_part2: u32;
    0x0051290C => remap_palettes: [RemapPalette; 0x7];

    0x00512510 => image_updatefuncs: [ImageStepFrame; 0x11];
    0x005125A0 => image_drawfuncs: [ImageDraw; 0x11];

    0x006283EC => first_hidden_unit: *mut Unit;
    0x00628428 => last_hidden_unit: *mut Unit;
    0x00628430 => first_active_unit: *mut Unit;
    0x0059CC9C => last_active_unit: *mut Unit;
    0x0062842C => first_dying_unit: *mut Unit;
    0x0059CC98 => last_dying_unit: *mut Unit;
    0x0063FF5C => first_invisible_unit: *mut Unit;
    0x006283F4 => first_revealer: *mut Unit;
    0x00628434 => last_revealer: *mut Unit;
    0x00628438 => first_free_unit: *mut Unit;
    0x0062843C => last_free_unit: *mut Unit;
    0x006283F8 => first_player_unit: [*mut Unit; 0xc];

    0x0067D400 => guard_ais: [GuardAi; 0x3e8];
    0x006B5448 => worker_ais: [WorkerAi; 0x3e8];
    0x0069F468 => building_ais: [BuildingAi; 0x3e8];
    0x006957E0 => military_ais: [MilitaryAi; 0x3e8];
    0x006416A0 => orders: [Order; 0x7d0];
    0x006BEE8C => path_array_start: *mut Path;

    0x00654874 => first_active_lone_sprite: *mut LoneSprite;
    0x00654868 => first_active_fow_sprite: *mut LoneSprite;
    0x0065291C => last_active_lone_sprite: *mut LoneSprite;
    0x0065486C => last_active_fow_sprite: *mut LoneSprite;
    0x006509D0 => first_free_fow_sprite: *mut LoneSprite;
    0x00654870 => last_free_fow_sprite: *mut LoneSprite;
    0x00654878 => first_free_lone_sprite: *mut LoneSprite;
    0x0065487C => last_free_lone_sprite: *mut LoneSprite;
    0x00652918 => cursor_marker: *mut LoneSprite;

    0x00581DD6 => player_minimap_color: [u8; 0xc];
    0x00581D76 => player_color_palette: [[u8; 0x8]; 0xc];
);

pub const TooltipSurfaceHeight: usize = 0x00481359;
pub const TooltipSurfaceBytes: usize = 0x0048133F;
pub const TooltipTextSurfaceHeight: usize = 0x0048137C;
pub const TooltipTextSurfaceBytes: usize = 0x0048136C;

pub mod storm {
    whack_funcs!(stdcall, init_funcs, 0x15000000,
        0x15027760 =>
            SBmpLoadImage(*const u8, *mut u8, *mut u8, u32, *mut u32, *mut u32, u32) -> u32;
    );
}
