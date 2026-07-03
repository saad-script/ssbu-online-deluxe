use std::sync::atomic::{AtomicU8, Ordering};
use ultelier::sync_guest::{self, ResolutionLevel};

mod common;
mod sephiroth;
mod utils;

use crate::render::profile::RenderProfileManager;

static ENTRY_BASE_RESOLUTION: AtomicU8 = AtomicU8::new(ResolutionLevel::Res1920x1080 as u8);

pub(in crate::perf_scaler) fn push_dynamic_res_report() {
    let base_res_level =
        ResolutionLevel::from_u32(ENTRY_BASE_RESOLUTION.load(Ordering::SeqCst) as u32).unwrap();
    let target_res = if base_res_level >= ResolutionLevel::Res1024x576 {
        ResolutionLevel::Res854x480
    } else if base_res_level >= ResolutionLevel::Res1280x720 {
        ResolutionLevel::Res1024x576
    } else {
        ResolutionLevel::Res1280x720
    };

    sync_guest::push_dynamic_res_report(target_res);
}

pub(in crate::perf_scaler) fn pop_dynamic_res_report() {
    let base_res_level =
        ResolutionLevel::from_u32(ENTRY_BASE_RESOLUTION.load(Ordering::SeqCst) as u32).unwrap();
    let target_res = if base_res_level >= ResolutionLevel::Res1024x576 {
        ResolutionLevel::Res854x480
    } else if base_res_level >= ResolutionLevel::Res1280x720 {
        ResolutionLevel::Res1024x576
    } else {
        ResolutionLevel::Res1280x720
    };

    sync_guest::pop_dynamic_res_report(target_res);
}

pub(crate) fn match_init() {
    sync_guest::clear_all_dynamic_res_report();
    let rps = RenderProfileManager::active_render_profile_settings();
    let base_res_level = rps.default_resolution_level();
    ENTRY_BASE_RESOLUTION.store(base_res_level as u8, Ordering::SeqCst);
}

pub(crate) fn match_reinit() {
    sync_guest::clear_all_dynamic_res_report();
}

pub(crate) fn match_cleanup() {
    sync_guest::clear_all_dynamic_res_report();
}

pub(super) fn install() {
    common::install();
    sephiroth::install();
}
