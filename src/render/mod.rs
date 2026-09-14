use std::sync::Arc;

use serde::Deserialize;
use ultelier::sync_guest::SsbuSyncConfig;

use crate::{render::profile::RenderProfileConfig, utils::is_emulator, CONFIG};

pub mod profile;

#[repr(C)]
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RenderConfig {
    pub render_profile_config: RenderProfileConfig,
    pub overclocker: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            render_profile_config: RenderProfileConfig::default(),
            overclocker: true,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ssbu_online_deluxe_set_render_config(
    render_config: *const RenderConfig,
) -> bool {
    println!("[ssbu-online-deluxe] Setting render config from external plugin...");
    if let Some(render_config) = render_config.as_ref().cloned() {
        let overclocker_enabled = render_config.overclocker;
        let config = CONFIG.load().clone();
        let mut config = (*config).clone();
        config.render_config = render_config;
        CONFIG.store(Arc::new(config));

        ultelier::sync_guest::set_env_flag(
            ultelier::sync_guest::EnvironmentFlags::OVERCLOCKER,
            overclocker_enabled,
        );

        return true;
    }
    false
}

pub(super) fn on_nro_load() {
    profile::on_nro_load();
}

pub(super) fn install() {
    let mut sync_config = SsbuSyncConfig::vanilla();
    sync_config.overclocker = !is_emulator() && CONFIG.load().render_config.overclocker;
    ultelier::sync_guest::install(sync_config);
    profile::install();
}
