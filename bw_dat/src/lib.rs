extern crate libc;
#[macro_use] extern crate whack;

mod bw;

pub unsafe fn init(patcher: &mut whack::ModulePatcher) {
    bw::init_vars(patcher);
}

unsafe fn get(dat: &bw::DatTable, id: u32) -> u32 {
    assert!(dat.entries > id);
    match dat.entry_size {
        1 => *(dat.data as *const u8).offset(id as isize) as u32,
        2 => *(dat.data as *const u16).offset(id as isize) as u32,
        4 => *(dat.data as *const u32).offset(id as isize),
        x => panic!("Invalid dat entry size: {}", x),
    }
}

pub mod units {
    use bw;
    pub fn get(index: usize, id: u16) -> u32 {
        unsafe {
            super::get(&bw::units_dat[index], id as u32)
        }
    }

    pub fn amount() -> u32 {
        unsafe { bw::units_dat[0].entries }
    }

    pub fn hitpoints(id: u16) -> i32 {
        get(8, id) as i32
    }

    pub fn shields(id: u16) -> i32 {
        // Yeah, it is stored as displayed
        get(7, id) as i32 * 256
    }

    pub fn has_shields(id: u16) -> u32 {
        get(6, id)
    }

    pub fn ground_weapon(id: u16) -> u32 {
        get(17, id)
    }

    pub fn air_weapon(id: u16) -> u32 {
        get(19, id)
    }

    pub fn flags(id: u16) -> u32 {
        get(22, id)
    }

    pub fn group_flags(id: u16) -> u32 {
        get(44, id)
    }

    pub fn armor(id: u16) -> u32 {
        get(27, id)
    }

    pub fn armor_upgrade(id: u16) -> u32 {
        get(25, id)
    }

    pub fn mineral_cost(id: u16) -> u32 {
        get(40, id)
    }

    pub fn gas_cost(id: u16) -> u32 {
        get(41, id)
    }

    pub fn build_time(id: u16) -> u32 {
        get(42, id)
    }

    pub fn supply_cost(id: u16) -> u32 {
        get(46, id)
    }
}

pub mod weapons {
    use bw;
    pub fn get(index: usize, id: u32) -> u32 {
        unsafe {
            super::get(&bw::weapons_dat[index], id)
        }
    }

    pub fn amount() -> u32 {
        unsafe { bw::weapons_dat[0].entries }
    }

    pub fn damage(id: u32) -> u32 {
        get(14, id)
    }

    pub fn upgrade(id: u32) -> u32{
        get(6, id)
    }

    pub fn bonus(id: u32) -> u32 {
        get(15, id)
    }

    pub fn factor(id: u32) -> u32{
        get(17, id)
    }

    pub fn label(id: u32) -> u32 {
        get(0, id)
    }

}

pub mod upgrades {
    use bw;
    pub fn get(index: usize, id: u32) -> u32 {
        unsafe {
            super::get(&bw::upgrades_dat[index], id)
        }
    }

    pub fn amount() -> u32 {
        unsafe { bw::upgrades_dat[0].entries }
    }

    pub fn label(id: u32) -> u32 {
        get(8, id)
    }

    pub fn mineral_cost(id: u32) -> u32 {
        get(0, id)
    }

    pub fn gas_cost(id: u32) -> u32 {
        get(2, id)
    }

    pub fn time(id: u32) -> u32 {
        get(4, id)
    }

    pub fn mineral_factor(id: u32) -> u32 {
        get(1, id)
    }

    pub fn gas_factor(id: u32) -> u32 {
        get(3, id)
    }

    pub fn time_factor(id: u32) -> u32 {
        get(5, id)
    }

    pub fn icon(id: u32) -> u32 {
        get(7, id)
    }

    pub fn repeat_count(id: u32) -> u32 {
        get(10, id)
    }
}

pub mod techdata {
    use bw;
    pub fn get(index: usize, id: u32) -> u32 {
        unsafe {
            super::get(&bw::techdata_dat[index], id)
        }
    }

    pub fn amount() -> u32 {
        unsafe { bw::techdata_dat[0].entries }
    }

    pub fn mineral_cost(id: u32) -> u32 {
        get(0, id)
    }

    pub fn gas_cost(id: u32) -> u32 {
        get(1, id)
    }

    pub fn time(id: u32) -> u32 {
        get(2, id)
    }

    pub fn energy_cost(id: u32) -> u32 {
        get(3, id)
    }

    pub fn icon(id: u32) -> u32 {
        get(6, id)
    }
}
