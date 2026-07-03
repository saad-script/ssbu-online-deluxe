use std::sync::{
    atomic::{AtomicBool, AtomicU16, Ordering},
    RwLock,
};

use serde::{de::IntoDeserializer, Deserialize, Deserializer};
use ssbu_pia_interface::StationConnectionManager;
use ultelier::sync_guest::{self, profile::DockedProfile, BufferMode, IndexMode, ResolutionLevel};

use crate::{
    input_poll::InputSnapshot,
    net::{get_match_status, is_connected, is_in_game, is_valid_online_mode, MatchStatus},
    utils::is_emulator,
};

static RENDER_PROFILE_CONFIG_FILE_PATH: &str = "sd:/ultimate/ssbu_online_deluxe/config.toml";
static RENDER_PROFILE_CONFIG: RwLock<Option<RenderProfileConfig>> = RwLock::new(None);

#[repr(C)]
#[derive(Debug, Default)]
pub struct FFIMatchRenderProfiles {
    singles: *const RenderProfile,
    doubles: *const RenderProfile,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FFIRenderProfileConfig {
    menu: *const RenderProfile,
    offline_match: *const FFIMatchRenderProfiles,
    online_match: *const FFIMatchRenderProfiles,
}

#[derive(Debug, Default, Deserialize)]
pub struct MatchRenderProfiles {
    singles: Option<RenderProfile>,
    doubles: Option<RenderProfile>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RenderProfileConfig {
    menu: Option<RenderProfile>,
    offline_match: Option<MatchRenderProfiles>,
    online_match: Option<MatchRenderProfiles>,
}

#[derive(Debug, Clone)]
pub struct RenderProfileSettings {
    buffer_mode: BufferMode,
    index_mode: IndexMode,
    default_resolution_level: ResolutionLevel,
    dynamic_res_enabled: bool,
    vsync_enabled: bool,
    render_opts_enabled: bool,
    boost_enabled: bool,
}

impl RenderProfileSettings {
    pub const fn vanilla() -> Self {
        RenderProfileSettings {
            buffer_mode: BufferMode::Triple,
            index_mode: IndexMode::TwoBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_res_enabled: false,
            vsync_enabled: true,
            render_opts_enabled: false,
            boost_enabled: false,
        }
    }
    pub fn less_lag() -> Self {
        let is_emulator = is_emulator();
        RenderProfileSettings {
            buffer_mode: BufferMode::Double,
            index_mode: IndexMode::OneBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_res_enabled: !is_emulator,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
        }
    }
    pub fn less_lag_ultra() -> Self {
        let is_emulator = is_emulator();
        let default_res = match is_emulator {
            true => ResolutionLevel::Res1920x1080,
            false => ResolutionLevel::Res1024x576,
        };
        RenderProfileSettings {
            buffer_mode: BufferMode::Double,
            index_mode: IndexMode::Immediate,
            default_resolution_level: default_res,
            dynamic_res_enabled: !is_emulator,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
        }
    }
    pub fn less_lag_doubles() -> Self {
        RenderProfileSettings {
            buffer_mode: BufferMode::Triple,
            index_mode: IndexMode::OneBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_res_enabled: true,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
        }
    }
    pub fn from_render_profile(rp: RenderProfile) -> Self {
        let mut rps = match rp.preset {
            RenderProfilePreset::Vanilla => RenderProfileSettings::vanilla(),
            RenderProfilePreset::LessLag => RenderProfileSettings::less_lag(),
            RenderProfilePreset::LessLagUltra => RenderProfileSettings::less_lag_ultra(),
            RenderProfilePreset::LessLagDoubles => RenderProfileSettings::less_lag_doubles(),
            _ => panic!("Must specific a valid render profile"),
        };
        rps.boost_enabled = rp.boost_enabled;
        rps
    }
    pub fn from_env() -> Self {
        RenderProfileSettings {
            buffer_mode: sync_guest::buffer_mode()
                .flatten()
                .unwrap_or(BufferMode::Triple),
            index_mode: sync_guest::index_mode()
                .flatten()
                .unwrap_or(IndexMode::TwoBehind),
            default_resolution_level: sync_guest::default_game_resolution_level()
                .flatten()
                .unwrap_or(ResolutionLevel::Res1920x1080),
            dynamic_res_enabled: sync_guest::dynamic_resolution_enabled().unwrap_or(false),
            vsync_enabled: sync_guest::vsync_enabled().unwrap_or(true),
            render_opts_enabled: sync_guest::render_opts_enabled().unwrap_or(false),
            boost_enabled: sync_guest::fps_boost_enabled().unwrap_or(false),
        }
    }
    pub fn from_bits(bits: u16) -> Option<Self> {
        Some(Self {
            buffer_mode: BufferMode::from_u32((bits & 0b11) as u32)?,
            index_mode: IndexMode::from_u32(((bits >> 2) & 0b11) as u32)?,
            default_resolution_level: ResolutionLevel::from_u32(((bits >> 4) & 0x7f) as u32)?,
            dynamic_res_enabled: ((bits >> 11) & 1) != 0,
            vsync_enabled: ((bits >> 12) & 1) != 0,
            render_opts_enabled: ((bits >> 13) & 1) != 0,
            boost_enabled: ((bits >> 14) & 1) != 0,
        })
    }
    pub const fn to_bits(&self) -> u16 {
        (self.buffer_mode as u16)
            | ((self.index_mode as u16) << 2)
            | ((self.default_resolution_level as u16) << 4)
            | ((self.dynamic_res_enabled as u16) << 11)
            | ((self.vsync_enabled as u16) << 12)
            | ((self.render_opts_enabled as u16) << 13)
            | ((self.boost_enabled as u16) << 14)
    }

    #[allow(dead_code)]
    pub fn buffer_mode(&self) -> BufferMode {
        self.buffer_mode
    }
    #[allow(dead_code)]
    pub fn index_mode(&self) -> IndexMode {
        self.index_mode
    }
    #[allow(dead_code)]
    pub fn default_resolution_level(&self) -> ResolutionLevel {
        self.default_resolution_level
    }
    #[allow(dead_code)]
    pub fn dynamic_res_enabled(&self) -> bool {
        self.dynamic_res_enabled
    }
    #[allow(dead_code)]
    pub fn vsync_enabled(&self) -> bool {
        self.vsync_enabled
    }
    #[allow(dead_code)]
    pub fn render_opts_enabled(&self) -> bool {
        self.render_opts_enabled
    }
    #[allow(dead_code)]
    pub fn boost_enabled(&self) -> bool {
        self.boost_enabled
    }
}

#[repr(u8)]
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderProfilePreset {
    #[default]
    Vanilla = 0,
    LessLag = 1,
    #[serde(alias = "LLUltra")]
    LessLagUltra = 2,
    #[serde(alias = "LLDoubles")]
    LessLagDoubles = 3,
    Custom = 4,
}

impl RenderProfilePreset {
    fn recommended(is_online: bool, is_doubles: bool) -> Self {
        if is_online {
            if is_doubles {
                return Self::LessLagDoubles;
            } else {
                if is_emulator() {
                    return Self::LessLagUltra;
                } else {
                    return Self::LessLag;
                }
            }
        }
        Self::Vanilla
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RenderProfile {
    preset: RenderProfilePreset,
    boost_enabled: bool,
}

impl RenderProfile {
    pub fn from_settings(rps: &RenderProfileSettings) -> Self {
        let preset = match (
            rps.buffer_mode,
            rps.index_mode,
            rps.vsync_enabled,
            rps.render_opts_enabled,
        ) {
            (BufferMode::Triple, IndexMode::TwoBehind, true, false) => RenderProfilePreset::Vanilla,
            (BufferMode::Double, IndexMode::OneBehind, false, true) => RenderProfilePreset::LessLag,
            (BufferMode::Double, IndexMode::Immediate, false, true) => {
                RenderProfilePreset::LessLagUltra
            }
            (BufferMode::Triple, IndexMode::OneBehind, false, true) => {
                RenderProfilePreset::LessLagDoubles
            }
            _ => RenderProfilePreset::Custom,
        };
        RenderProfile {
            preset,
            boost_enabled: rps.boost_enabled,
        }
    }
}

impl<'de> Deserialize<'de> for RenderProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let render_profile_string = String::deserialize(deserializer)?;
        let render_profile_str = render_profile_string.trim();

        let boost_enabled = render_profile_str.ends_with("+") && is_emulator();
        let render_profile_str = render_profile_str.trim_end_matches("+");
        let preset = RenderProfilePreset::deserialize(render_profile_str.into_deserializer())?;

        Ok(RenderProfile {
            preset,
            boost_enabled,
        })
    }
}

impl std::fmt::Display for RenderProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let preset_str = match self.preset {
            RenderProfilePreset::Vanilla => "Vanilla",
            RenderProfilePreset::LessLag => "LessLag",
            RenderProfilePreset::LessLagUltra => "LLUltra",
            RenderProfilePreset::LessLagDoubles => "LLDoubles",
            RenderProfilePreset::Custom => "Custom",
        };
        let boost_enabled_str = match self.boost_enabled {
            true => "++",
            false => "",
        };
        write!(f, "{}{}", preset_str, boost_enabled_str)
    }
}

pub struct RenderProfileManager {
    auto_mode: AtomicBool,
    selected_profile_settings: AtomicU16,
}

#[allow(dead_code)]
impl RenderProfileManager {
    const fn new() -> Self {
        RenderProfileManager {
            auto_mode: AtomicBool::new(true),
            selected_profile_settings: AtomicU16::new(RenderProfileSettings::vanilla().to_bits()),
        }
    }
    pub fn instance() -> &'static Self {
        static RENDER_PROFILE_MANAGER: RenderProfileManager = RenderProfileManager::new();
        &RENDER_PROFILE_MANAGER
    }
    pub fn selected_render_profile_settings(&self) -> RenderProfileSettings {
        RenderProfileSettings::from_bits(self.selected_profile_settings.load(Ordering::SeqCst))
            .unwrap()
    }
    pub fn active_render_profile_settings() -> RenderProfileSettings {
        RenderProfileSettings::from_env()
    }
    pub fn selected_render_profile(&self) -> RenderProfile {
        RenderProfile::from_settings(&self.selected_render_profile_settings())
    }
    pub fn active_render_profile() -> RenderProfile {
        RenderProfile::from_settings(&Self::active_render_profile_settings())
    }
    pub fn selected_render_profile_set_boost_enabled(&self, boost_enabled: bool) {
        let mut rps = self.selected_render_profile_settings();
        rps.boost_enabled = boost_enabled && is_emulator();
        self.selected_profile_settings
            .store(rps.to_bits(), Ordering::SeqCst);
    }
    pub fn apply_selected_profile_settings(&self) {
        Self::apply_render_profile_settings_immediate(&self.selected_render_profile_settings());
    }
    pub fn select_next_render_profile(&self) -> RenderProfile {
        let selected_rp = self.selected_render_profile();
        if self.is_auto_mode() {
            return selected_rp;
        }
        let next_preset = match selected_rp.preset {
            RenderProfilePreset::Vanilla => RenderProfilePreset::LessLag,
            RenderProfilePreset::LessLag => RenderProfilePreset::LessLagUltra,
            RenderProfilePreset::LessLagUltra => RenderProfilePreset::LessLagDoubles,
            RenderProfilePreset::LessLagDoubles => RenderProfilePreset::Vanilla,
            RenderProfilePreset::Custom => RenderProfilePreset::Vanilla,
        };
        let next_rp = RenderProfile {
            preset: next_preset,
            boost_enabled: selected_rp.boost_enabled,
        };
        let next_rps = RenderProfileSettings::from_render_profile(next_rp);
        self.selected_profile_settings
            .store(next_rps.to_bits(), Ordering::SeqCst);
        next_rp
    }
    pub fn select_prev_render_profile(&self) -> RenderProfile {
        let selected_rp = self.selected_render_profile();
        if self.is_auto_mode() {
            return selected_rp;
        }
        let prev_preset = match selected_rp.preset {
            RenderProfilePreset::Vanilla => RenderProfilePreset::LessLagDoubles,
            RenderProfilePreset::LessLag => RenderProfilePreset::Vanilla,
            RenderProfilePreset::LessLagUltra => RenderProfilePreset::LessLag,
            RenderProfilePreset::LessLagDoubles => RenderProfilePreset::LessLagUltra,
            RenderProfilePreset::Custom => RenderProfilePreset::Vanilla,
        };
        let prev_rp = RenderProfile {
            preset: prev_preset,
            boost_enabled: selected_rp.boost_enabled,
        };
        let prev_rps = RenderProfileSettings::from_render_profile(prev_rp);
        self.selected_profile_settings
            .store(prev_rps.to_bits(), Ordering::SeqCst);
        prev_rp
    }
    pub fn is_auto_mode(&self) -> bool {
        self.auto_mode.load(Ordering::SeqCst)
    }
    pub fn set_auto_mode(&self, auto_mode: bool) {
        self.auto_mode.store(auto_mode, Ordering::SeqCst);
        let num_connected_stations = StationConnectionManager::num_connected_stations();
        self.auto_select_profile(is_valid_online_mode(), num_connected_stations > 1);
    }
    pub fn poll(
        &self,
        input_snapshot: &InputSnapshot,
        prev: ninput::Buttons,
        next: ninput::Buttons,
    ) -> bool {
        let boost_buttons = input_snapshot.check_buttons_pressed_with_cooldown(&[
            ninput::Buttons::L | ninput::Buttons::R | ninput::Buttons::ZL | ninput::Buttons::X,
            ninput::Buttons::L | ninput::Buttons::R | ninput::Buttons::ZR | ninput::Buttons::X,
        ]);
        if !boost_buttons.is_empty() {
            let rps = self.selected_render_profile_settings();
            self.selected_render_profile_set_boost_enabled(!rps.boost_enabled);
            return true;
        }

        let cycle_buttons = input_snapshot.check_buttons_pressed_with_cooldown(&[prev, next]);
        if cycle_buttons == prev {
            if self.is_auto_mode() {
                self.set_auto_mode(false);
                let boost_enabled = self.selected_render_profile().boost_enabled;
                let rp = RenderProfile {
                    preset: RenderProfilePreset::Vanilla,
                    boost_enabled,
                };
                self.select_render_profile_immediate(rp);
            } else {
                if self.selected_render_profile().preset == RenderProfilePreset::Vanilla {
                    self.set_auto_mode(true);
                } else {
                    self.select_prev_render_profile();
                }
            }
            return true;
        } else if cycle_buttons == next {
            if self.is_auto_mode() {
                self.set_auto_mode(false);
                let boost_enabled = self.selected_render_profile().boost_enabled;
                let rp = RenderProfile {
                    preset: RenderProfilePreset::Vanilla,
                    boost_enabled,
                };
                self.select_render_profile_immediate(rp);
            } else {
                if self.selected_render_profile().preset == RenderProfilePreset::LessLagDoubles {
                    self.set_auto_mode(true);
                } else {
                    self.select_next_render_profile();
                }
            }
            return true;
        }
        false
    }
    pub fn recommended_render_profile(&self, is_online: bool, is_doubles: bool) -> RenderProfile {
        let rpc = RENDER_PROFILE_CONFIG.read().unwrap();
        let mrp = rpc.as_ref().and_then(|m| {
            if is_online {
                m.online_match.as_ref()
            } else {
                m.offline_match.as_ref()
            }
        });
        let rp = if is_doubles {
            mrp.and_then(|m| m.doubles)
        } else {
            mrp.and_then(|m| m.singles)
        };

        let rp = rp.unwrap_or_else(|| RenderProfile {
            preset: RenderProfilePreset::recommended(is_online, is_doubles),
            boost_enabled: self.selected_render_profile().boost_enabled,
        });
        rp
    }
    pub fn auto_select_profile(&self, is_online: bool, is_doubles: bool) {
        if !self.is_auto_mode() {
            return;
        }
        let rp = self.recommended_render_profile(is_online, is_doubles);
        self.select_render_profile_immediate(rp);
        println!("AUTO SELECT RP: {:?}", rp);
    }
    fn select_render_profile_immediate(&self, rp: RenderProfile) {
        self.selected_profile_settings.store(
            RenderProfileSettings::from_render_profile(rp).to_bits(),
            Ordering::SeqCst,
        );
    }
    fn apply_render_profile_settings_immediate(rps: &RenderProfileSettings) {
        let is_emulator = is_emulator();

        // apply resolution
        let _ = sync_guest::set_default_game_resolution_level(rps.default_resolution_level);
        let _ = sync_guest::set_dynamic_resolution_enabled(rps.dynamic_res_enabled);

        // apply render options
        if let Some(env_flags) = sync_guest::env_flags() {
            let flags = env_flags
                .with(
                    sync_guest::EnvironmentFlags::FPS_BOOST_ENABLED,
                    rps.boost_enabled && is_emulator,
                )
                .with(
                    sync_guest::EnvironmentFlags::VSYNC_ENABLED,
                    rps.vsync_enabled,
                )
                .with(
                    sync_guest::EnvironmentFlags::RENDER_OPTS_ENABLED,
                    rps.render_opts_enabled,
                )
                .with(
                    sync_guest::EnvironmentFlags::TRIPLE_ENABLED,
                    rps.buffer_mode == BufferMode::Triple,
                )
                .with_value(
                    sync_guest::EnvironmentFlags::INDEX_MODE,
                    rps.index_mode as u32,
                );
            let _ = sync_guest::replace_env_flags(flags);
        }

        // set overclock profile
        if !is_emulator {
            let oc_profile = match (RenderProfile::from_settings(&rps).preset, is_in_game()) {
                (_, false) => DockedProfile::Rest,
                (
                    RenderProfilePreset::Vanilla
                    | RenderProfilePreset::LessLag
                    | RenderProfilePreset::Custom,
                    _,
                ) => DockedProfile::Singles,
                (RenderProfilePreset::LessLagUltra | RenderProfilePreset::LessLagDoubles, _) => {
                    DockedProfile::Ffa
                }
            };
            let _ = sync_guest::profile::apply_docked_profile(oc_profile);
        }
    }
}

pub(crate) fn match_init() {
    let is_valid_online_mode = is_valid_online_mode();
    let is_connected = is_connected();
    let match_status = get_match_status();
    let in_real_online_match = is_connected && match_status != MatchStatus::Training;

    if in_real_online_match {
        if is_valid_online_mode {
            // apply selected profile if its a valid online match
            RenderProfileManager::instance()
                .auto_select_profile(true, match_status == MatchStatus::Doubles);
            RenderProfileManager::instance().apply_selected_profile_settings();
        } else {
            // ssbusync already enforces this, but leaving this here for clarity
            RenderProfileManager::apply_render_profile_settings_immediate(
                &RenderProfileSettings::vanilla(),
            );
        }
    } else {
        let rp = RenderProfileManager::instance()
            .recommended_render_profile(false, match_status == MatchStatus::Doubles);
        let rps = RenderProfileSettings::from_render_profile(rp);
        RenderProfileManager::apply_render_profile_settings_immediate(&rps);
    }
}

pub(crate) fn match_cleanup() {
    let rpc = RENDER_PROFILE_CONFIG.read().unwrap();
    let menu_rp = rpc
        .as_ref()
        .and_then(|c| c.menu)
        .unwrap_or_else(|| RenderProfile::default());
    RenderProfileManager::apply_render_profile_settings_immediate(
        &RenderProfileSettings::from_render_profile(menu_rp),
    );
}

fn try_load_config_file() -> std::io::Result<RenderProfileConfig> {
    let config_file = std::path::PathBuf::from(RENDER_PROFILE_CONFIG_FILE_PATH);
    if !config_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "SSBU Online Deluxe config file not found",
        ));
    }

    let contents = std::fs::read_to_string(RENDER_PROFILE_CONFIG_FILE_PATH)?;
    let config: RenderProfileConfig = toml::from_str(contents.as_str())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(config)
}

#[no_mangle]
pub unsafe extern "C" fn ssbu_online_deluxe_set_render_profile_config(
    render_profile_config: *const FFIRenderProfileConfig,
) -> bool {
    println!("[ssbu-online-deluxe] Setting config from external plugin...");
    let mut rpc = RENDER_PROFILE_CONFIG.write().unwrap();
    if render_profile_config.is_null() {
        return false;
    }
    *rpc = Some(RenderProfileConfig {
        menu: render_profile_config
            .as_ref()
            .and_then(|d| d.menu.as_ref().cloned()),
        offline_match: Some(MatchRenderProfiles {
            singles: render_profile_config
                .as_ref()
                .and_then(|d| d.offline_match.as_ref())
                .and_then(|d| d.singles.as_ref().cloned()),
            doubles: render_profile_config
                .as_ref()
                .and_then(|d| d.offline_match.as_ref())
                .and_then(|d| d.doubles.as_ref().cloned()),
        }),
        online_match: Some(MatchRenderProfiles {
            singles: render_profile_config
                .as_ref()
                .and_then(|d| d.online_match.as_ref())
                .and_then(|d| d.singles.as_ref().cloned()),
            doubles: render_profile_config
                .as_ref()
                .and_then(|d| d.online_match.as_ref())
                .and_then(|d| d.doubles.as_ref().cloned()),
        }),
    });
    return true;
}

pub(super) fn on_nro_load() {
    let mut rpc = RENDER_PROFILE_CONFIG.write().unwrap();
    if rpc.is_none() {
        println!("Render profile config not specified. Trying to load from file...");
        let render_profile_config = match try_load_config_file() {
            Err(err) => {
                println!("Unable to load config from file: {}", err);
                println!("Using default config...");
                RenderProfileConfig::default()
            }
            Ok(rpc) => {
                println!("Parsed render profile config file succesfully!");
                rpc
            }
        };
        *rpc = Some(render_profile_config);
    }
    println!("Loaded Render Profile Config:\n{:?}", *rpc);

    let menu_rp = rpc
        .as_ref()
        .and_then(|c| c.menu)
        .unwrap_or_else(|| RenderProfile::default());
    RenderProfileManager::apply_render_profile_settings_immediate(
        &RenderProfileSettings::from_render_profile(menu_rp),
    );
}
