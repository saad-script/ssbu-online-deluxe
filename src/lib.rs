#![allow(static_mut_refs)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LazyLock,
};

use arc_swap::ArcSwap;
use serde::Deserialize;
use skyline::nro::NroInfo;

use crate::{input::KeyBindingConfig, render::RenderConfig};

mod input;
mod net;
mod perf_scaler;
mod render;
mod ui;
mod utils;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub render_config: RenderConfig,
    pub key_binding_config: KeyBindingConfig,
}

static CONFIG_FILE_PATH: &str = "sd:/ultimate/ssbu_online_deluxe/config.toml";
static CONFIG: LazyLock<ArcSwap<Config>> =
    LazyLock::new(|| ArcSwap::from_pointee(Config::default()));

fn try_load_config_file() -> std::io::Result<Config> {
    let config_file = std::path::PathBuf::from(CONFIG_FILE_PATH);
    if !config_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "SSBU Online Deluxe config file not found",
        ));
    }

    let contents = std::fs::read_to_string(CONFIG_FILE_PATH)?;
    let config: Config = toml::from_str(contents.as_str())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(config)
}

fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let err_msg = format!("thread has panicked at '{}', {}\0", msg, location);
        skyline::error::show_error(
            69,
            "Skyline plugin has panicked! Please open the details and send a screenshot to the developer, then close the game.\0",
            err_msg.as_str()
        );
    }));
}

fn on_nro_load(_info: &NroInfo) {
    static NRO_LOADED: AtomicBool = AtomicBool::new(false);
    if NRO_LOADED.swap(true, Ordering::SeqCst) {
        return;
    }
    crate::render::on_nro_load();
}

#[skyline::main(name = "ssbu-online-deluxe")]
pub fn main() {
    setup_panic_hook();

    skyline::nro::add_hook(on_nro_load)
        .expect("Unable to add NRO load hook! Make sure nro hook plugin is present.");

    let load_config_thread = std::thread::Builder::new()
        .stack_size(0x20000)
        .spawn(|| unsafe {
            skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 5);
            println!("Config not specified. Trying to load from file...");
            match try_load_config_file() {
                Err(err) => {
                    println!("Unable to load config from file: {}", err);
                    println!("Using default config...");
                }
                Ok(rc) => {
                    println!("Parsed config file succesfully!");
                    CONFIG.store(Arc::new(rc));
                }
            };
            println!("Loaded Config:\n{:?}", CONFIG.load());
            skyline::nn::os::ChangeThreadPriority(skyline::nn::os::GetCurrentThread(), 24);
        })
        .expect("Unable to spawn config loader thread!");
    load_config_thread
        .join()
        .expect("Config loader thread crashed!");

    render::install();
    net::install();
    ui::install();
    perf_scaler::install();
}
