use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use serde::{de::IntoDeserializer, Deserialize, Deserializer};
use ssbu_pia_interface::StationConnectionManager;
use ultelier::sync_guest::{
    self, BufferMode, IndexMode, OverclockConfig, OverclockProfile, ResolutionLevel,
};

use crate::{
    input::InputSnapshot,
    net::{get_match_status, is_connected, is_valid_online_mode, MatchStatus},
    utils::is_emulator,
    CONFIG,
};

static mut TOGGLE_FPS_BOOST_KEYBIND: ninput::Buttons = ninput::Buttons::empty();

#[repr(C)]
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct RenderProfileConfig {
    pub menu: RenderProfile,
    pub offline_match: OfflineMatchRenderProfiles,
    pub online_match: OnlineMatchRenderProfiles,
    pub overrides: RenderProfileOverrides,
}

/// Per-preset overrides. Any field left unspecified inherits the preset's
/// built-in value.
#[repr(C)]
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct RenderProfileOverrides {
    #[serde(alias = "Vanilla")]
    pub vanilla: RenderProfileOverride,
    #[serde(alias = "LessLag")]
    pub less_lag: RenderProfileOverride,
    #[serde(alias = "LessLagUltra", alias = "LLUltra")]
    pub less_lag_ultra: RenderProfileOverride,
    #[serde(alias = "LessLagDoubles", alias = "LLDoubles")]
    pub less_lag_doubles: RenderProfileOverride,
}

impl RenderProfileOverrides {
    pub fn for_preset(&self, preset: RenderProfilePreset) -> Option<&RenderProfileOverride> {
        match preset {
            RenderProfilePreset::Vanilla => Some(&self.vanilla),
            RenderProfilePreset::LessLag => Some(&self.less_lag),
            RenderProfilePreset::LessLagUltra => Some(&self.less_lag_ultra),
            RenderProfilePreset::LessLagDoubles => Some(&self.less_lag_doubles),
            RenderProfilePreset::Custom => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct RenderProfileOverride {
    pub overclock: Option<OverclockOverride>,
    pub default_resolution: Option<ResolutionSetting>,
    pub dynamic_resolution: Option<ResolutionSetting>,
}

#[repr(C)]
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct OverclockOverride {
    pub cpu: u32,
    pub gpu: u32,
    pub mem: u32,
}

impl RenderProfileOverride {
    fn apply_to(&self, rps: &mut RenderProfileSettings) {
        if let Some(oc) = self.overclock {
            rps.overclock = Some(oc.into());
        }
        if let Some(default_res) = self.default_resolution.and_then(|r| r.level()) {
            rps.default_resolution_level = default_res;
        }
        if let Some(dynamic_res) = self.dynamic_resolution {
            rps.dynamic_resolution_level = dynamic_res.level();
        }
    }
}

impl From<OverclockOverride> for OverclockConfig {
    fn from(oc: OverclockOverride) -> Self {
        OverclockConfig::new(oc.cpu, oc.gpu, oc.mem)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSetting {
    Level(ResolutionLevel),
    None,
}

impl ResolutionSetting {
    pub fn level(self) -> Option<ResolutionLevel> {
        match self {
            Self::Level(l) => Some(l),
            Self::None => None,
        }
    }
}

impl<'de> Deserialize<'de> for ResolutionSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.trim().to_ascii_lowercase();
        let setting = match s.as_str() {
            "none" | "off" | "disabled" => Self::None,
            "1920x1080" | "1080p" => Self::Level(ResolutionLevel::Res1920x1080),
            "1600x900" | "900p" => Self::Level(ResolutionLevel::Res1600x900),
            "1280x720" | "720p" => Self::Level(ResolutionLevel::Res1280x720),
            "1024x576" | "576p" => Self::Level(ResolutionLevel::Res1024x576),
            "854x480" | "480p" => Self::Level(ResolutionLevel::Res854x480),
            other => {
                return Err(serde::de::Error::custom(format!(
                    "invalid resolution '{other}'; expected one of 1920x1080, 1600x900, 1280x720, 1024x576, 854x480, none"
                )))
            }
        };
        Ok(setting)
    }
}

trait MatchRenderProfiles {
    fn singles(&self) -> RenderProfile;
    fn doubles(&self) -> RenderProfile;
}

#[repr(C)]
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct OfflineMatchRenderProfiles {
    singles: RenderProfile,
    doubles: RenderProfile,
}

impl MatchRenderProfiles for OfflineMatchRenderProfiles {
    fn singles(&self) -> RenderProfile {
        self.singles
    }
    fn doubles(&self) -> RenderProfile {
        self.doubles
    }
}

#[repr(C)]
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct OnlineMatchRenderProfiles {
    singles: RenderProfile,
    doubles: RenderProfile,
}

impl Default for OnlineMatchRenderProfiles {
    fn default() -> Self {
        let default_singles_preset = if is_emulator() {
            RenderProfilePreset::LessLagUltra
        } else {
            RenderProfilePreset::LessLag
        };
        Self {
            singles: RenderProfile {
                preset: default_singles_preset,
                fps_boost_status: FPSBoostStatus::Unspecified,
            },
            doubles: RenderProfile {
                preset: RenderProfilePreset::LessLagDoubles,
                fps_boost_status: FPSBoostStatus::Unspecified,
            },
        }
    }
}

impl MatchRenderProfiles for OnlineMatchRenderProfiles {
    fn singles(&self) -> RenderProfile {
        self.singles
    }
    fn doubles(&self) -> RenderProfile {
        self.doubles
    }
}

#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FPSBoostStatus {
    #[default]
    Unspecified = 0,
    Enabled = 1,
    Disabled = 2,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RenderProfile {
    preset: RenderProfilePreset,
    fps_boost_status: FPSBoostStatus,
}

impl RenderProfile {
    pub const fn new(preset: RenderProfilePreset, boost_enabled: bool) -> Self {
        let fps_boost_status = if boost_enabled {
            FPSBoostStatus::Enabled
        } else {
            FPSBoostStatus::Disabled
        };
        Self {
            preset,
            fps_boost_status,
        }
    }
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
        Self::new(preset, rps.boost_enabled)
    }
    pub fn boost_enabled(&self) -> bool {
        self.fps_boost_status == FPSBoostStatus::Enabled
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

        Ok(Self::new(preset, boost_enabled))
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
        let boost_enabled_str = match self.boost_enabled() {
            true => "++",
            false => "",
        };
        write!(f, "{}{}", preset_str, boost_enabled_str)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderProfileSettings {
    pub(crate) buffer_mode: BufferMode,
    pub(crate) index_mode: IndexMode,
    pub(crate) default_resolution_level: ResolutionLevel,
    pub(crate) dynamic_resolution_level: Option<ResolutionLevel>,
    pub(crate) vsync_enabled: bool,
    pub(crate) render_opts_enabled: bool,
    pub(crate) boost_enabled: bool,
    pub(crate) overclock: Option<OverclockConfig>,
}

impl RenderProfileSettings {
    pub const fn vanilla() -> Self {
        RenderProfileSettings {
            buffer_mode: BufferMode::Triple,
            index_mode: IndexMode::TwoBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_resolution_level: None,
            vsync_enabled: true,
            render_opts_enabled: false,
            boost_enabled: false,
            overclock: None,
        }
    }
    pub fn less_lag() -> Self {
        let dynamic_res_level = match is_emulator() {
            true => None,
            false => Some(ResolutionLevel::Res1280x720),
        };
        RenderProfileSettings {
            buffer_mode: BufferMode::Double,
            index_mode: IndexMode::OneBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_resolution_level: dynamic_res_level,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
            overclock: None,
        }
    }
    pub fn less_lag_ultra() -> Self {
        let (default_res_level, dynamic_res_level) = match is_emulator() {
            true => (ResolutionLevel::Res1920x1080, None),
            false => (
                ResolutionLevel::Res1024x576,
                Some(ResolutionLevel::Res854x480),
            ),
        };
        RenderProfileSettings {
            buffer_mode: BufferMode::Double,
            index_mode: IndexMode::Immediate,
            default_resolution_level: default_res_level,
            dynamic_resolution_level: dynamic_res_level,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
            overclock: None,
        }
    }
    pub fn less_lag_doubles() -> Self {
        let dynamic_res_level = match is_emulator() {
            true => None,
            false => Some(ResolutionLevel::Res1280x720),
        };
        RenderProfileSettings {
            buffer_mode: BufferMode::Triple,
            index_mode: IndexMode::OneBehind,
            default_resolution_level: ResolutionLevel::Res1920x1080,
            dynamic_resolution_level: dynamic_res_level,
            vsync_enabled: false,
            render_opts_enabled: true,
            boost_enabled: false,
            overclock: None,
        }
    }
    fn preset_overclock(preset: RenderProfilePreset, is_doubles: bool) -> Option<OverclockProfile> {
        match (preset, is_doubles) {
            (RenderProfilePreset::Vanilla, _) => None,
            (RenderProfilePreset::LessLagUltra, _) => Some(OverclockProfile::PerformanceFfa),
            (
                RenderProfilePreset::LessLag
                | RenderProfilePreset::LessLagDoubles
                | RenderProfilePreset::Custom,
                true,
            ) => Some(OverclockProfile::PerformanceFfa),
            (
                RenderProfilePreset::LessLag
                | RenderProfilePreset::LessLagDoubles
                | RenderProfilePreset::Custom,
                false,
            ) => Some(OverclockProfile::PerformanceSingles),
        }
    }
    pub fn from_render_profile(rp: RenderProfile, is_doubles: bool) -> Self {
        let mut rps = match rp.preset {
            RenderProfilePreset::Vanilla => RenderProfileSettings::vanilla(),
            RenderProfilePreset::LessLag => RenderProfileSettings::less_lag(),
            RenderProfilePreset::LessLagUltra => RenderProfileSettings::less_lag_ultra(),
            RenderProfilePreset::LessLagDoubles => RenderProfileSettings::less_lag_doubles(),
            _ => panic!("Must specific a valid render profile"),
        };
        rps.boost_enabled = rp.boost_enabled();
        rps.overclock = Self::preset_overclock(rp.preset, is_doubles).map(OverclockProfile::config);

        let rc = &CONFIG.load().render_config;
        if let Some(ov) = rc.render_profile_config.overrides.for_preset(rp.preset) {
            ov.apply_to(&mut rps);
        }

        if !rc.overclocker || is_emulator() {
            rps.overclock = None;
        }
        rps
    }
    pub fn from_env() -> Self {
        RenderProfileSettings {
            buffer_mode: sync_guest::buffer_mode().unwrap_or(BufferMode::Triple),
            index_mode: sync_guest::index_mode().unwrap_or(IndexMode::TwoBehind),
            default_resolution_level: sync_guest::default_resolution_level()
                .unwrap_or(ResolutionLevel::Res1920x1080),
            dynamic_resolution_level: sync_guest::dynamic_resolution_level().unwrap_or(None),
            vsync_enabled: sync_guest::vsync_enabled().unwrap_or(true),
            render_opts_enabled: sync_guest::render_opts_enabled().unwrap_or(false),
            boost_enabled: sync_guest::fps_boost_enabled().unwrap_or(false),
            overclock: sync_guest::profile::cached_overclock_profile()
                .map(OverclockProfile::config),
        }
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
    pub fn dynamic_resolution_level(&self) -> Option<ResolutionLevel> {
        self.dynamic_resolution_level
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
    #[allow(dead_code)]
    pub fn overclock(&self) -> Option<OverclockConfig> {
        self.overclock
    }
}

pub struct RenderProfileManager {
    auto_mode: AtomicBool,
    is_doubles: AtomicBool,
    selected_profile: Mutex<RenderProfile>,
}

#[allow(dead_code)]
impl RenderProfileManager {
    const fn new() -> Self {
        RenderProfileManager {
            auto_mode: AtomicBool::new(true),
            is_doubles: AtomicBool::new(false),
            selected_profile: Mutex::new(RenderProfile::new(RenderProfilePreset::Vanilla, false)),
        }
    }
    pub fn instance() -> &'static Self {
        static RENDER_PROFILE_MANAGER: RenderProfileManager = RenderProfileManager::new();
        &RENDER_PROFILE_MANAGER
    }
    pub fn selected_render_profile_settings(&self) -> RenderProfileSettings {
        RenderProfileSettings::from_render_profile(
            self.selected_render_profile(),
            self.is_doubles.load(Ordering::SeqCst),
        )
    }
    pub fn active_render_profile_settings() -> RenderProfileSettings {
        RenderProfileSettings::from_env()
    }
    pub fn selected_render_profile(&self) -> RenderProfile {
        *self.selected_profile.lock().unwrap()
    }
    pub fn active_render_profile() -> RenderProfile {
        RenderProfile::from_settings(&Self::active_render_profile_settings())
    }
    pub fn selected_render_profile_set_boost_enabled(&self, boost_enabled: bool) {
        let mut selected_rp = self.selected_profile.lock().unwrap();
        *selected_rp = RenderProfile::new(selected_rp.preset, boost_enabled && is_emulator());
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
        let next_rp = RenderProfile::new(next_preset, selected_rp.boost_enabled());
        self.select_render_profile_immediate(next_rp);
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
        let prev_rp = RenderProfile::new(prev_preset, selected_rp.boost_enabled());
        self.select_render_profile_immediate(prev_rp);
        prev_rp
    }
    pub fn is_auto_mode(&self) -> bool {
        self.auto_mode.load(Ordering::SeqCst)
    }
    pub fn set_auto_mode(&self, auto_mode: bool) {
        self.auto_mode.store(auto_mode, Ordering::SeqCst);
        let num_connected_stations = StationConnectionManager::num_connected_stations();
        self.refresh_profile(is_valid_online_mode(), num_connected_stations > 1);
    }
    pub fn poll(
        &self,
        input_snapshot: &InputSnapshot,
        prev: ninput::Buttons,
        next: ninput::Buttons,
    ) -> bool {
        let boost_buttons = input_snapshot
            .check_buttons_pressed_with_cooldown(&[unsafe { TOGGLE_FPS_BOOST_KEYBIND }]);
        if !boost_buttons.is_empty() {
            let rp = self.selected_render_profile();
            self.selected_render_profile_set_boost_enabled(!rp.boost_enabled());
            return true;
        }

        let cycle_buttons = input_snapshot.check_buttons_pressed_with_cooldown(&[prev, next]);
        if cycle_buttons == prev {
            if self.is_auto_mode() {
                self.set_auto_mode(false);
                let selected_rp = self.selected_render_profile();
                let rp = RenderProfile::new(
                    RenderProfilePreset::LessLagDoubles,
                    selected_rp.boost_enabled(),
                );
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
                let selected_rp = self.selected_render_profile();
                let rp =
                    RenderProfile::new(RenderProfilePreset::Vanilla, selected_rp.boost_enabled());
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
        let rc = &CONFIG.load().render_config;
        let mrp: &dyn MatchRenderProfiles = if is_online {
            &rc.render_profile_config.online_match
        } else {
            &rc.render_profile_config.offline_match
        };
        let mut rp = if is_doubles {
            mrp.doubles().clone()
        } else {
            mrp.singles().clone()
        };

        // if boost status is unspecified, inherit from prev render profile
        if rp.fps_boost_status == FPSBoostStatus::Unspecified {
            rp.fps_boost_status = match self.selected_render_profile().boost_enabled() {
                true => FPSBoostStatus::Enabled,
                false => FPSBoostStatus::Disabled,
            };
        }

        rp
    }
    pub fn refresh_profile(&self, is_online: bool, is_doubles: bool) {
        self.is_doubles.store(is_doubles, Ordering::SeqCst);
        if !self.is_auto_mode() {
            return;
        }
        let rp = self.recommended_render_profile(is_online, is_doubles);
        self.select_render_profile_immediate(rp);
        println!("AUTO SELECT RP: {:?}", rp);
    }
    fn select_render_profile_immediate(&self, rp: RenderProfile) {
        *self.selected_profile.lock().unwrap() = rp;
    }
    fn apply_render_profile_settings_immediate(rps: &RenderProfileSettings) {
        let is_emulator = is_emulator();

        // apply resolution
        let _ = sync_guest::set_default_resolution_level(rps.default_resolution_level);
        let _ = sync_guest::set_dynamic_resolution_level(rps.dynamic_resolution_level);

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
        let rc = &CONFIG.load().render_config;
        if rc.overclocker && !is_emulator {
            match rps.overclock {
                Some(oc) => sync_guest::profile::apply_profile(OverclockProfile::from_config(oc)),
                None => sync_guest::profile::clear_profile(),
            };
        }
    }
}

pub(crate) fn match_init() {
    let is_valid_online_mode = is_valid_online_mode();
    let is_connected = is_connected();
    let match_status = get_match_status();
    let is_doubles = match_status == MatchStatus::Doubles;
    let in_real_online_match = is_connected && match_status != MatchStatus::Training;

    if in_real_online_match {
        if is_valid_online_mode {
            // apply selected profile if its a valid online match
            RenderProfileManager::instance().refresh_profile(true, is_doubles);
            RenderProfileManager::instance().apply_selected_profile_settings();
        } else {
            // ssbusync already enforces this, but leaving this here for clarity
            RenderProfileManager::apply_render_profile_settings_immediate(
                &RenderProfileSettings::vanilla(),
            );
        }
    } else {
        let rp = RenderProfileManager::instance().recommended_render_profile(false, is_doubles);
        let rps = RenderProfileSettings::from_render_profile(rp, is_doubles);
        RenderProfileManager::apply_render_profile_settings_immediate(&rps);
    }
}

fn apply_menu_profile() {
    let rc = &CONFIG.load().render_config;
    let mut rps = RenderProfileSettings::from_render_profile(rc.render_profile_config.menu, false);
    rps.overclock = None;
    RenderProfileManager::apply_render_profile_settings_immediate(&rps);
}

pub(crate) fn match_cleanup() {
    apply_menu_profile();
}

pub(super) fn on_nro_load() {
    apply_menu_profile();
}

pub(super) fn install() {
    unsafe {
        TOGGLE_FPS_BOOST_KEYBIND = CONFIG.load().key_binding_config.toggle_fps_boost;
    }
}
