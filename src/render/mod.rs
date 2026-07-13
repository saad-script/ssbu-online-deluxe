use std::sync::{Arc, LazyLock};

use arc_swap::ArcSwap;
use serde::Deserialize;
use ultelier::sync_guest::SsbuSyncConfig;

use crate::{
    render::profile::RenderProfileConfig,
    utils::{is_emulator, lookup_symbol_addr},
};

pub mod profile;

static RENDER_PROFILE_CONFIG_FILE_PATH: &str = "sd:/ultimate/ssbu_online_deluxe/config.toml";
static RENDER_CONFIG: LazyLock<ArcSwap<RenderConfig>> =
    LazyLock::new(|| ArcSwap::from_pointee(RenderConfig::default()));

#[repr(C)]
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RenderConfig {
    render_profile_config: RenderProfileConfig,
    overclocker: bool,
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
    println!("[ssbu-online-deluxe] Setting config from external plugin...");
    if let Some(render_config) = render_config.as_ref().cloned() {
        RENDER_CONFIG.store(Arc::new(render_config));
        try_load_overclock_service();
        return true;
    }
    false
}

fn try_load_overclock_service() -> bool {
    let rc = RENDER_CONFIG.load();
    let is_emulator = is_emulator();
    let overclocker_enabled = rc.overclocker && !is_emulator;
    if overclocker_enabled {
        if !nx_over_configure_nstuff_oc() {
            skyline::error::show_error(
                70,
                "Unable to load overclocker service!\0",
                "SSBU Online Deluxe requires the overclocker service to be installed correctly! Ensure 'libnx_over.nro' is present and 'sd:/atmosphere/contents/00FF0000A11CE0FF' contains the overclock service files. If you want to intentionally disable the overclocker, set 'overclocker_enabled=false' in the render config file (sd:/ultimate/ssbu_online_deluxe/config.toml). You may continue, but note that the game will drop frames and stutter heavily when using non-vanilla render profiles (LessLag, LessLagUltra, etc).\0",
            );
            return false;
        }
        return true;
    } else {
        println!(
            "Overclocker load skipped! (render_config.overclocker_enabled={}, is_emulator={})",
            rc.overclocker, is_emulator
        );
        return false;
    }
}

fn nx_over_configure_nstuff_oc() -> bool {
    static NX_OVER_CONFIGURE_NSTUFF_OC_SYMBOL: &[u8] = b"nx_over_configure_nstuff_oc\0";
    if let Some(func_addr) = lookup_symbol_addr(NX_OVER_CONFIGURE_NSTUFF_OC_SYMBOL) {
        unsafe {
            let func: fn() -> u32 = std::mem::transmute(func_addr);
            func();
        }
        return true;
    }
    false
}

fn try_load_config_file() -> std::io::Result<RenderConfig> {
    let config_file = std::path::PathBuf::from(RENDER_PROFILE_CONFIG_FILE_PATH);
    if !config_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "SSBU Online Deluxe config file not found",
        ));
    }

    let contents = std::fs::read_to_string(RENDER_PROFILE_CONFIG_FILE_PATH)?;
    let config: RenderConfig = toml::from_str(contents.as_str())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(config)
}

pub(super) fn on_nro_load() {
    profile::on_nro_load();
}

pub(super) fn install() {
    let load_config_thread = std::thread::Builder::new()
        .stack_size(0x20000)
        .spawn(|| unsafe {
            skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 5);
            println!("Render profile config not specified. Trying to load from file...");
            match try_load_config_file() {
                Err(err) => {
                    println!("Unable to load config from file: {}", err);
                    println!("Using default config...");
                }
                Ok(rc) => {
                    println!("Parsed render profile config file succesfully!");
                    RENDER_CONFIG.store(Arc::new(rc));
                }
            };
            println!("Loaded Render Profile Config:\n{:?}", RENDER_CONFIG.load());
            skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 24);
        })
        .expect("Unable to spawn config loader thread!");
    load_config_thread
        .join()
        .expect("Config loader thread crashed!");

    let overclocker_loaded = try_load_overclock_service();
    let mut sync_config = SsbuSyncConfig::vanilla();
    sync_config.overclocker = overclocker_loaded;
    ultelier::sync_guest::install(sync_config);
}
