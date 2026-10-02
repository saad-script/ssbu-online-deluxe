use smashline::skyline_smash::app::{self, lua_bind::FighterCutInManager};

extern "C" {
    #[link_name = "\u{1}_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E"]
    static mut FIGHTER_MANAGER: *mut app::FighterManager;

    #[link_name = "\u{1}_ZN3lib9SingletonIN3app19FighterCutInManagerEE9instance_E"]
    static mut FIGHTER_CUT_IN_MANAGER: *mut app::FighterCutInManager;

    #[link_name = "\u{1}_ZN3app8lua_bind38FighterManager__get_fighter_entry_implEPNS_14FighterManagerENS_14FighterEntryIDE"]
    fn get_fighter_entry(arg1: *mut app::FighterManager, arg2: i32) -> u64;
}

pub fn is_cut_in_playing() -> bool {
    unsafe {
        if FIGHTER_CUT_IN_MANAGER.is_null() {
            return false;
        }
        FighterCutInManager::is_play_status(FIGHTER_CUT_IN_MANAGER)
    }
}

pub fn is_valid_fighter_entry_id(entry_id: i32) -> bool {
    unsafe {
        if FIGHTER_MANAGER.is_null() {
            return false;
        }
        if entry_id < 0 || entry_id > 7 {
            return false;
        }
        let entry = get_fighter_entry(FIGHTER_MANAGER, entry_id);
        return entry != 0;
    }
}
