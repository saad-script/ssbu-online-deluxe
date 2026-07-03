use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};

use smashline::{
    skyline_smash::{
        app::{
            self,
            lua_bind::{SlowModule, WorkModule},
        },
        lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    },
    *,
};

use crate::perf_scaler::{
    pop_dynamic_res_report, push_dynamic_res_report, utils::is_valid_fighter_entry_id,
};

const CRITICAL_HIT_FINISH_COOLDOWN_FRAMES: i8 = 7;
static CRITICAL_HIT_ACTIVE: [AtomicBool; 8] = [const { AtomicBool::new(false) }; 8];
static CRITICAL_HIT_COOLDOWN: [AtomicI8; 8] =
    [const { AtomicI8::new(CRITICAL_HIT_FINISH_COOLDOWN_FRAMES) }; 8];

unsafe extern "C" fn global_critical_hit_init(fighter: &mut L2CFighterCommon) {
    let entry_id = WorkModule::get_int(
        fighter.module_accessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !is_valid_fighter_entry_id(entry_id) {
        return;
    }
    CRITICAL_HIT_ACTIVE[entry_id as usize].store(false, Ordering::SeqCst);
    CRITICAL_HIT_COOLDOWN[entry_id as usize]
        .store(CRITICAL_HIT_FINISH_COOLDOWN_FRAMES, Ordering::SeqCst);
}

unsafe extern "C" fn global_critical_hit_fighter_frame(fighter: &mut L2CFighterCommon) {
    let entry_id = WorkModule::get_int(
        fighter.module_accessor,
        *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    );
    if !is_valid_fighter_entry_id(entry_id) {
        return;
    }
    let critical_hit_active = CRITICAL_HIT_ACTIVE[entry_id as usize].load(Ordering::SeqCst);
    if !critical_hit_active {
        return;
    }

    let is_slow = SlowModule::is_slow(fighter.module_accessor);

    if is_slow {
        CRITICAL_HIT_COOLDOWN[entry_id as usize]
            .store(CRITICAL_HIT_FINISH_COOLDOWN_FRAMES, Ordering::SeqCst);
    } else {
        let prev = CRITICAL_HIT_COOLDOWN[entry_id as usize].fetch_sub(1, Ordering::SeqCst);
        let cooldown_frames_left = prev - 1;

        println!(
            "[CRITICAL_HIT_DRS] critical hit finish cooldown frames left: {}",
            cooldown_frames_left
        );
        if cooldown_frames_left <= 0 {
            pop_dynamic_res_report();
            CRITICAL_HIT_ACTIVE[entry_id as usize].store(false, Ordering::SeqCst);
            println!(
                "[CRITICAL_HIT_DRS] intensive_frame_end entry_id={}",
                entry_id
            );
        }
    }
}

#[skyline::hook(offset = 0x06a7100)]
unsafe fn fighter_cut_in_start(
    figher_cut_in_manager: u64,
    boma: *mut app::BattleObjectModuleAccessor,
    cut_in_transactor: u64,
    cut_in_type: u32,
    cut_in_data: u64,
    cut_in_priority: u32,
) {
    call_original!(
        figher_cut_in_manager,
        boma,
        cut_in_transactor,
        cut_in_type,
        cut_in_data,
        cut_in_priority
    );

    println!("[CRITICAL_HIT_DRS] FighterCutInStart");
    let entry_id = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if !is_valid_fighter_entry_id(entry_id) {
        println!(
            "[CRITICAL_HIT_DRS] ERROR: FigherCutInStart called on invalid entry_id={}",
            entry_id
        );
        return;
    }
    if !CRITICAL_HIT_ACTIVE[entry_id as usize].swap(true, Ordering::SeqCst) {
        println!(
            "[CRITICAL_HIT_DRS] intensive_frame_start, entry_id={}",
            entry_id
        );
        push_dynamic_res_report();
    }
}

pub fn install() {
    skyline::install_hooks!(fighter_cut_in_start);
    Agent::new("fighter")
        .on_start(global_critical_hit_init)
        .on_line(Main, global_critical_hit_fighter_frame)
        .install();
}
