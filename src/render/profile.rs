use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

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
    /// Overclock (MHz). Only applied on console with `overclocker = true`.
    pub overclock: Option<OverclockOverride>,
    /// Base render resolution, e.g. `"1280x720"`.
    pub default_resolution: Option<ResolutionSetting>,
    /// Resolution used during heavy moves, e.g. `"854x480"`, or `"none"` to disable.
    pub dynamic_resolution: Option<ResolutionSetting>,
}

#[repr(C)]
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct OverclockOverride {
    pub cpu: u32,
    pub gpu: u32,
    pub mem: u32,
}

impl From<OverclockOverride> for OverclockProfile {
    fn from(oc: OverclockOverride) -> Self {
        OverclockProfile::Custom(OverclockConfig::new(oc.cpu, oc.gpu, oc.mem))
    }
}

/// A resolution level as written in the config file (`"1920x1080"`, `"720p"`, `"none"`, ...).
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
    pub fn new(preset: RenderProfilePreset, boost_enabled: bool) -> Self {
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

#[derive(Debug, Clone)]
pub struct RenderProfileSettings {
    buffer_mode: BufferMode,
    index_mode: IndexMode,
    default_resolution_level: ResolutionLevel,
    dynamic_resolution_level: Option<ResolutionLevel>,
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
            dynamic_resolution_level: None,
            vsync_enabled: true,
            render_opts_enabled: false,
            boost_enabled: false,
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
        rps.boost_enabled = rp.boost_enabled();

        // layer resolution overrides from config on top of the preset
        let rc = &CONFIG.load().render_config;
        if let Some(ov) = rc.render_profile_config.overrides.for_preset(rp.preset) {
            if let Some(default_res) = ov.default_resolution.and_then(|r| r.level()) {
                rps.default_resolution_level = default_res;
            }
            if let Some(dynamic_res) = ov.dynamic_resolution {
                rps.dynamic_resolution_level = dynamic_res.level();
            }
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
        }
    }
    // Bit layout (must match between `to_bits` and `from_bits`):
    //   [ 0.. 2)  buffer_mode              (2 bits)
    //   [ 2.. 4)  index_mode               (2 bits)
    //   [ 4.. 8)  default_resolution_level (4 bits)
    //   [ 8..12)  dynamic_resolution_level (4 bits; sentinel 0xF == None)
    //   [12..13)  vsync_enabled            (1 bit)
    //   [13..14)  render_opts_enabled      (1 bit)
    //   [14..15)  boost_enabled            (1 bit)
    const BUFFER_MODE_SHIFT: u16 = 0;
    const BUFFER_MODE_MASK: u16 = 0b11;
    const INDEX_MODE_SHIFT: u16 = 2;
    const INDEX_MODE_MASK: u16 = 0b11;
    const DEFAULT_RES_SHIFT: u16 = 4;
    const DEFAULT_RES_MASK: u16 = 0xF;
    const DYNAMIC_RES_SHIFT: u16 = 8;
    const DYNAMIC_RES_MASK: u16 = 0xF;
    const DYNAMIC_RES_NONE: u16 = 0xF;
    const VSYNC_SHIFT: u16 = 12;
    const RENDER_OPTS_SHIFT: u16 = 13;
    const BOOST_SHIFT: u16 = 14;

    pub fn from_bits(bits: u16) -> Option<Self> {
        Some(Self {
            buffer_mode: BufferMode::from_u32(
                ((bits >> Self::BUFFER_MODE_SHIFT) & Self::BUFFER_MODE_MASK) as u32,
            )?,
            index_mode: IndexMode::from_u32(
                ((bits >> Self::INDEX_MODE_SHIFT) & Self::INDEX_MODE_MASK) as u32,
            )?,
            default_resolution_level: ResolutionLevel::from_u32(
                ((bits >> Self::DEFAULT_RES_SHIFT) & Self::DEFAULT_RES_MASK) as u32,
            )?,
            dynamic_resolution_level: ResolutionLevel::from_u32(
                ((bits >> Self::DYNAMIC_RES_SHIFT) & Self::DYNAMIC_RES_MASK) as u32,
            ),
            vsync_enabled: ((bits >> Self::VSYNC_SHIFT) & 1) != 0,
            render_opts_enabled: ((bits >> Self::RENDER_OPTS_SHIFT) & 1) != 0,
            boost_enabled: ((bits >> Self::BOOST_SHIFT) & 1) != 0,
        })
    }
    pub const fn to_bits(&self) -> u16 {
        let dynamic_res_level: u16 = if let Some(l) = self.dynamic_resolution_level {
            (l as u16) & Self::DYNAMIC_RES_MASK
        } else {
            Self::DYNAMIC_RES_NONE
        };

        (((self.buffer_mode as u16) & Self::BUFFER_MODE_MASK) << Self::BUFFER_MODE_SHIFT)
            | (((self.index_mode as u16) & Self::INDEX_MODE_MASK) << Self::INDEX_MODE_SHIFT)
            | (((self.default_resolution_level as u16) & Self::DEFAULT_RES_MASK)
                << Self::DEFAULT_RES_SHIFT)
            | (dynamic_res_level << Self::DYNAMIC_RES_SHIFT)
            | ((self.vsync_enabled as u16) << Self::VSYNC_SHIFT)
            | ((self.render_opts_enabled as u16) << Self::RENDER_OPTS_SHIFT)
            | ((self.boost_enabled as u16) << Self::BOOST_SHIFT)
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
        let next_rp = RenderProfile::new(next_preset, selected_rp.boost_enabled());
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
        let prev_rp = RenderProfile::new(prev_preset, selected_rp.boost_enabled());
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
        let boost_buttons = input_snapshot
            .check_buttons_pressed_with_cooldown(&[unsafe { TOGGLE_FPS_BOOST_KEYBIND }]);
        if !boost_buttons.is_empty() {
            let rps = self.selected_render_profile_settings();
            self.selected_render_profile_set_boost_enabled(!rps.boost_enabled);
            return true;
        }

        let cycle_buttons = input_snapshot.check_buttons_pressed_with_cooldown(&[prev, next]);
        if cycle_buttons == prev {
            if self.is_auto_mode() {
                self.set_auto_mode(false);
                let selected_rp = self.selected_render_profile();
                let rp =
                    RenderProfile::new(RenderProfilePreset::Vanilla, selected_rp.boost_enabled());
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
            let preset = RenderProfile::from_settings(&rps).preset;
            let oc_override = rc
                .render_profile_config
                .overrides
                .for_preset(preset)
                .and_then(|ov| ov.overclock);
            let oc_profile = match (preset, get_match_status()) {
                (_, MatchStatus::Inactive) => None,
                // config override takes priority over the built-in mapping
                (_, _) if oc_override.is_some() => oc_override.map(OverclockProfile::from),
                (RenderProfilePreset::Vanilla, _) => None,
                (RenderProfilePreset::LessLagUltra, _) => Some(OverclockProfile::PerformanceFfa),
                (
                    RenderProfilePreset::LessLag
                    | RenderProfilePreset::LessLagDoubles
                    | RenderProfilePreset::Custom,
                    MatchStatus::Doubles,
                ) => Some(OverclockProfile::PerformanceFfa),
                (
                    RenderProfilePreset::LessLag
                    | RenderProfilePreset::LessLagDoubles
                    | RenderProfilePreset::Custom,
                    MatchStatus::Singles | MatchStatus::Training,
                ) => Some(OverclockProfile::PerformanceSingles),
            };
            if let Some(oc_profile) = oc_profile {
                sync_guest::profile::apply_profile(oc_profile);
            } else {
                sync_guest::profile::clear_profile();
            }
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
    let rc = &CONFIG.load().render_config;
    let menu_rp = rc.render_profile_config.menu;
    RenderProfileManager::apply_render_profile_settings_immediate(
        &RenderProfileSettings::from_render_profile(menu_rp),
    );
}

pub(super) fn on_nro_load() {
    let rc = &CONFIG.load().render_config;
    let menu_rp = rc.render_profile_config.menu;
    RenderProfileManager::apply_render_profile_settings_immediate(
        &RenderProfileSettings::from_render_profile(menu_rp),
    );
}

pub(super) fn install() {
    unsafe {
        TOGGLE_FPS_BOOST_KEYBIND = CONFIG.load().key_binding_config.toggle_fps_boost;
    }
}
