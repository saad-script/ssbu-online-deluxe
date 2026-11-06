//! All PIA custom-comms packet versions and their parsers, in one place.
//!
//! Packets travel on our own PIA [`Channel`](ssbu_pia_interface::Channel), so
//! nothing older than v1 (the 4-byte payloads piggybacked on PIA's own packets
//! before the channel API existed) can ever arrive here.
//!
//! Every packet starts with a [`Header`]. `compat_version` is the oldest
//! version whose layout is a prefix of this packet, which lets peers read
//! packets from newer peers as long as the newer layout only appended fields.
//!
//! ## Adding a new wire-format version
//!
//! 1. Bump [`CURRENT_VERSION`].
//! 2. Add a new `mod vN` below with a `#[repr(C)]` `Packet` and change
//!    [`PiaPacket`] to point at it. The rest of the crate always deals with
//!    `PiaPacket`, so bumping the alias is the only downstream churn.
//! 3. If `vN::Packet` only appends fields to the previous layout, leave
//!    [`COMPAT_VERSION`] alone so older peers keep reading the prefix they know.
//!    Otherwise set it to `N`.
//! 4. Give the previous version an `impl From<&vM::Packet> for PiaPacket` that
//!    up-converts it and add a match arm for it in [`decode`].
//!
//! Older `mod vN` modules should be treated as frozen: never edit an existing
//! layout, only add new ones.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{net::latency_slider::StationLatency, render::profile::RenderProfileSettings};
use ultelier::sync_guest::{BufferMode, IndexMode, OverclockConfig, ResolutionLevel};

/// Wire-format version of the packets we send. Bump when the layout of
/// [`PiaPacket`] changes.
pub(super) const CURRENT_VERSION: u16 = 1;

/// Oldest version whose layout is a prefix of [`PiaPacket`]: peers at or above
/// this version can read our packets.
pub(super) const COMPAT_VERSION: u16 = 1;

/// The current on-wire packet type. All non-parsing code should reference this
/// alias so a version bump only requires editing this file.
pub(super) type PiaPacket = v1::Packet;

/// Shared by every version, so a peer can always tell what it is reading.
#[repr(C)]
#[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Debug, Clone, Copy)]
pub(super) struct Header {
    pub(super) version: u16,
    pub(super) compat_version: u16,
}

impl Header {
    pub(super) const fn current() -> Self {
        Self {
            version: CURRENT_VERSION,
            compat_version: COMPAT_VERSION,
        }
    }
}

/// Errors that can occur while decoding an incoming packet.
#[derive(Debug)]
#[allow(dead_code)] // fields used only via Debug in log output
pub(super) enum DecodeError {
    /// Packet was shorter than the layout expected for its declared version.
    InvalidData,
    /// The packet's layout is newer than anything this build can read.
    UnknownVersion(u16),
    /// The packet decoded, but one of its fields holds a value we don't know.
    InvalidField(&'static str),
}

/// Decoded contents of a packet from any known version.
#[derive(Debug, Clone)]
pub(super) struct StationState {
    pub(super) latency: StationLatency,
    pub(super) render_profile_settings: RenderProfileSettings,
}

/// Decode a raw incoming packet as any known version.
pub(super) fn decode(data: &[u8]) -> Result<StationState, DecodeError> {
    let (header, _) = Header::read_from_prefix(data).map_err(|_| DecodeError::InvalidData)?;
    let packet = match header.version {
        CURRENT_VERSION => read_current(data)?,
        // a newer peer whose layout starts with ours
        v if v > CURRENT_VERSION && header.compat_version <= CURRENT_VERSION => read_current(data)?,
        v => return Err(DecodeError::UnknownVersion(v)),
    };
    packet.to_station_state()
}

fn read_current(data: &[u8]) -> Result<PiaPacket, DecodeError> {
    PiaPacket::read_from_prefix(data)
        .map(|(p, _)| p)
        .map_err(|_| DecodeError::InvalidData)
}

// ---------------------------------------------------------------------------
// v1 — CURRENT
// ---------------------------------------------------------------------------
pub(super) mod v1 {
    use super::*;

    /// Sentinel for `dynamic_resolution_level` meaning dynamic res is off.
    const RES_NONE: u8 = u8::MAX;
    /// Sentinel for `WireLatency::frames` meaning auto hasn't resolved yet.
    const FRAMES_UNKNOWN: u8 = u8::MAX;

    #[repr(C)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Debug, Clone)]
    pub(in crate::net::pia) struct Packet {
        pub(in crate::net::pia) header: Header,
        pub(in crate::net::pia) latency: WireLatency,
        pub(in crate::net::pia) render_profile_settings: WireRenderProfileSettings,
    }

    #[repr(C)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Debug, Clone)]
    pub(in crate::net::pia) struct WireLatency {
        auto: u8,
        /// The chosen input buffer, or with `auto` the one the game picked
        /// at the start of the last match (`FRAMES_UNKNOWN` before that).
        frames: u8,
        _reserved: [u8; 2],
    }

    #[repr(C)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Debug, Clone)]
    pub(in crate::net::pia) struct WireRenderProfileSettings {
        buffer_mode: u8,
        index_mode: u8,
        default_resolution_level: u8,
        dynamic_resolution_level: u8,
        vsync_enabled: u8,
        render_opts_enabled: u8,
        boost_enabled: u8,
        overclock_enabled: u8,
        overclock_cpu_mhz: u32,
        overclock_gpu_mhz: u32,
        overclock_mem_mhz: u32,
    }

    impl Packet {
        pub(in crate::net::pia) fn new(
            latency: StationLatency,
            rps: &RenderProfileSettings,
        ) -> Self {
            let overclock = rps.overclock.unwrap_or(OverclockConfig::new(0, 0, 0));
            Self {
                header: Header::current(),
                latency: match latency {
                    StationLatency::Manual(frames) => WireLatency {
                        auto: 0,
                        frames,
                        _reserved: [0; 2],
                    },
                    StationLatency::Auto(frames) => WireLatency {
                        auto: 1,
                        frames: frames.unwrap_or(FRAMES_UNKNOWN),
                        _reserved: [0; 2],
                    },
                },
                render_profile_settings: WireRenderProfileSettings {
                    buffer_mode: rps.buffer_mode as u8,
                    index_mode: rps.index_mode as u8,
                    default_resolution_level: rps.default_resolution_level as u8,
                    dynamic_resolution_level: rps
                        .dynamic_resolution_level
                        .map_or(RES_NONE, |l| l as u8),
                    vsync_enabled: rps.vsync_enabled as u8,
                    render_opts_enabled: rps.render_opts_enabled as u8,
                    boost_enabled: rps.boost_enabled as u8,
                    overclock_enabled: rps.overclock.is_some() as u8,
                    overclock_cpu_mhz: overclock.cpu_hz,
                    overclock_gpu_mhz: overclock.gpu_hz,
                    overclock_mem_mhz: overclock.mem_hz,
                },
            }
        }

        pub(super) fn to_station_state(&self) -> Result<StationState, DecodeError> {
            let w = &self.render_profile_settings;
            let render_profile_settings = RenderProfileSettings {
                buffer_mode: BufferMode::from_u32(w.buffer_mode as u32)
                    .ok_or(DecodeError::InvalidField("buffer_mode"))?,
                index_mode: IndexMode::from_u32(w.index_mode as u32)
                    .ok_or(DecodeError::InvalidField("index_mode"))?,
                default_resolution_level: ResolutionLevel::from_u32(
                    w.default_resolution_level as u32,
                )
                .ok_or(DecodeError::InvalidField("default_resolution_level"))?,
                dynamic_resolution_level: match w.dynamic_resolution_level {
                    RES_NONE => None,
                    l => Some(
                        ResolutionLevel::from_u32(l as u32)
                            .ok_or(DecodeError::InvalidField("dynamic_resolution_level"))?,
                    ),
                },
                vsync_enabled: w.vsync_enabled != 0,
                render_opts_enabled: w.render_opts_enabled != 0,
                boost_enabled: w.boost_enabled != 0,
                overclock: (w.overclock_enabled != 0).then(|| {
                    OverclockConfig::new(
                        w.overclock_cpu_mhz,
                        w.overclock_gpu_mhz,
                        w.overclock_mem_mhz,
                    )
                }),
            };
            Ok(StationState {
                latency: match (self.latency.auto, self.latency.frames) {
                    (0, frames) => StationLatency::Manual(frames),
                    (_, FRAMES_UNKNOWN) => StationLatency::Auto(None),
                    (_, frames) => StationLatency::Auto(Some(frames)),
                },
                render_profile_settings,
            })
        }
    }
}
