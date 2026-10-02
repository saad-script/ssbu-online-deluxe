use std::sync::atomic::{AtomicBool, AtomicI8, Ordering};

use smashline::skyline_smash::app;

use crate::perf_scaler::utils::is_cut_in_playing;

const CUT_IN_FINISH_COOLDOWN_FRAMES: i8 = 7;
static CUT_IN_ACTIVE: AtomicBool = AtomicBool::new(false);
static CUT_IN_COOLDOWN: AtomicI8 = AtomicI8::new(CUT_IN_FINISH_COOLDOWN_FRAMES);

pub fn init() {
    CUT_IN_ACTIVE.store(false, Ordering::SeqCst);
    CUT_IN_COOLDOWN.store(CUT_IN_FINISH_COOLDOWN_FRAMES, Ordering::SeqCst);
}

// FighterCutInManager per-frame update (vtable slot, called once per frame).
// Ticks the cut-in timers and calls stop when a cut-in finishes.
#[skyline::hook(offset = 0x06ac5f0)]
unsafe fn fighter_cut_in_manager_update(manager: u64) {
    call_original!(manager);

    if !CUT_IN_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    if is_cut_in_playing() {
        CUT_IN_COOLDOWN.store(CUT_IN_FINISH_COOLDOWN_FRAMES, Ordering::SeqCst);
        return;
    }

    let cooldown_frames_left = CUT_IN_COOLDOWN.fetch_sub(1, Ordering::SeqCst) - 1;
    println!(
        "[CUT_IN_DRS] finish cooldown frames left: {}",
        cooldown_frames_left
    );
    if cooldown_frames_left <= 0 {
        CUT_IN_ACTIVE.store(false, Ordering::SeqCst);
        ultelier::sync_guest::pop_dynamic_res_report();
        println!("[CUT_IN_DRS] intensive_frame_end");
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

    // start can reject the request (priority, lockout, etc.) without
    // starting a cut-in; only push if one is actually playing now.
    if !is_cut_in_playing() {
        println!("[CUT_IN_DRS] start rejected type={}", cut_in_type);
        return;
    }

    // Push on the same frame start is requested; deferring to the update
    // hook makes the dynamic res a frame late.
    CUT_IN_COOLDOWN.store(CUT_IN_FINISH_COOLDOWN_FRAMES, Ordering::SeqCst);
    if !CUT_IN_ACTIVE.swap(true, Ordering::SeqCst) {
        ultelier::sync_guest::push_dynamic_res_report();
        println!("[CUT_IN_DRS] intensive_frame_start type={}", cut_in_type);
    }
}

pub fn install() {
    skyline::install_hooks!(fighter_cut_in_start, fighter_cut_in_manager_update);
}
