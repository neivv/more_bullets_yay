#![allow(non_upper_case_globals)]

use libc::c_void;

#[repr(C, packed)]
pub struct DatTable {
    pub data: *mut c_void,
    pub entry_size: u32,
    pub entries: u32,
}

whack_vars!(init_vars, 0x00400000,
    0x00513C30 => units_dat: [DatTable; 0x35];
    0x005136E0 => upgrades_dat: [DatTable; 0xb];
    0x005137D8 => techdata_dat: [DatTable; 0x8];
    0x00513868 => weapons_dat: [DatTable; 0x17];
);
