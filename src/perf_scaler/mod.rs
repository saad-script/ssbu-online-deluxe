mod common;
mod fighter;
mod utils;

use crate::render::profile::RenderProfileManager;

pub(crate) fn match_init() {
    let rps = RenderProfileManager::active_render_profile_settings();
    ultelier::sync_guest::clear_all_dynamic_res_report();
    ultelier::sync_guest::set_default_resolution_level(rps.default_resolution_level());
    ultelier::sync_guest::set_dynamic_resolution_level(rps.dynamic_resolution_level());
    common::init();
    fighter::init();
}

pub(crate) fn match_cleanup() {
    ultelier::sync_guest::clear_all_dynamic_res_report();
}

pub(super) fn install() {
    common::install();
    fighter::install();
}
