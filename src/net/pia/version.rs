//! All PIA custom-comms packet versions and their parsers, in one place.
//!
//! ## Adding a new wire-format version
//!
//! 1. Bump [`CURRENT_VERSION`].
//! 2. Change [`Current`] to point at your new `vN::Packet` type. The rest of
//!    the crate always deals with `Current` (re-exported from
//!    [`super::PiaCustomNetPacket`]), so bumping the alias is the only
//!    downstream churn.
//! 3. Add a new `mod vN` below with:
//!      - a `#[repr(C, packed)]` `Packet` struct
//!      - an `impl From<Packet> for Current` that up-converts to the new layout
//! 4. Add a match arm for `N` in [`decode`].
//!
//! Older `mod vN` modules should be treated as frozen: never edit an existing
//! layout, only add new ones.

use std::borrow::Cow;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::render::profile::RenderProfileSettings;

/// Wire-format version number transmitted in the `version` field of every
/// packet. Bump when the layout of [`Current`] changes.
pub(super) const CURRENT_VERSION: u8 = 4;

/// The current on-wire packet type. All non-parsing code should reference this
/// alias so a version bump only requires editing this file.
pub(super) type PiaPacket = v4::Packet;

/// Errors that can occur while decoding an incoming packet.
#[derive(Debug)]
#[allow(dead_code)] // fields used only via Debug in log output
pub(super) enum DecodeError {
    /// Packet was too short to even read the version byte, or its size did
    /// not match the layout expected for its declared version.
    InvalidData,
    /// The `version` byte does not correspond to any known packet version.
    UnknownVersion(u8),
}

/// Decode a raw incoming packet as any known version and up-convert it to
/// [`PiaPacket`].
///
/// Returns a [`Cow`]:
///   - `Cow::Borrowed` for the current version (zero-copy view directly into
///     `data`),
///   - `Cow::Owned`    for legacy versions (an up-converted packet built on
///     the stack; unavoidable since older layouts differ).
pub(super) fn decode(data: &[u8]) -> Result<Cow<'_, PiaPacket>, DecodeError> {
    let version = *data.first().ok_or(DecodeError::InvalidData)?;
    match version {
        4 => v4::Packet::ref_from_bytes(data)
            .map(Cow::Borrowed)
            .map_err(|_| DecodeError::InvalidData),
        3 => v3::Packet::ref_from_bytes(data)
            .map(|p| Cow::Owned(PiaPacket::from(p)))
            .map_err(|_| DecodeError::InvalidData),
        2 => v2::Packet::ref_from_bytes(data)
            .map(|p| Cow::Owned(PiaPacket::from(p)))
            .map_err(|_| DecodeError::InvalidData),
        v => Err(DecodeError::UnknownVersion(v)),
    }
}

// ---------------------------------------------------------------------------
// Shared legacy bit-layout (v2 & v3)
// ---------------------------------------------------------------------------
//
// Both v2 and v3 used the same u16 bit layout for `render_profile_settings`
// except for the top two bits:
//
//   [ 0.. 2)  buffer_mode
//   [ 2.. 4)  index_mode
//   [ 4..11)  default_resolution_level  (7 bits, values 0..=4)
//   [11..12)  dynamic_res_enabled       (bool)
//   [12..13)  vsync_enabled
//   [13..14)  render_opts_enabled
//   [14..16)  v2: overclock_profile (2 bits, discarded)
//             v3: boost_enabled     (1 bit at 14)
//
// `dynamic_res_enabled` was a bool; the actual resolution level was derived
// from the preset at apply time. For display / preset derivation only
// buffer/index/vsync/render_opts matter, so any valid level works when
// translating to the new `Option<ResolutionLevel>` field.

struct LegacyRps {
    buffer_index: u16, // packed buffer_mode | index_mode (bits 0..4)
    default_res: u16,  // 0..=4
    dynamic_res_enabled: bool,
    vsync_enabled: bool,
    render_opts_enabled: bool,
    boost_enabled: bool,
}

impl LegacyRps {
    fn from_v2_or_v3_bits(bits: u16, has_boost_bit: bool) -> Self {
        Self {
            buffer_index: bits & 0b1111,
            default_res: (bits >> 4) & 0x7F,
            dynamic_res_enabled: ((bits >> 11) & 1) != 0,
            vsync_enabled: ((bits >> 12) & 1) != 0,
            render_opts_enabled: ((bits >> 13) & 1) != 0,
            boost_enabled: has_boost_bit && ((bits >> 14) & 1) != 0,
        }
    }

    /// Re-encode as a v4 `RenderProfileSettings` bit-blob.
    fn to_v4_rps_bits(&self) -> u16 {
        let default_res = if self.default_res <= 0xF {
            self.default_res
        } else {
            (RenderProfileSettings::vanilla().to_bits() >> 4) & 0xF
        };

        // Pick an arbitrary valid resolution level (Res854x480 = 4) when the
        // peer had dynamic res enabled; sentinel 0xF == None otherwise.
        let dynamic_res_field: u16 = if self.dynamic_res_enabled { 4 } else { 0xF };

        self.buffer_index
            | (default_res << 4)
            | (dynamic_res_field << 8)
            | ((self.vsync_enabled as u16) << 12)
            | ((self.render_opts_enabled as u16) << 13)
            | ((self.boost_enabled as u16) << 14)
    }
}

// ---------------------------------------------------------------------------
// v4 — CURRENT
// ---------------------------------------------------------------------------
pub(super) mod v4 {
    use super::*;

    /// Current wire-format layout.
    ///
    /// `render_profile_settings_bits` follows the layout documented on
    /// `RenderProfileSettings::to_bits` (15 bits, 4-bit resolution fields).
    #[repr(C, packed)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Unaligned, Debug, Clone)]
    pub(in crate::net::pia) struct Packet {
        pub(in crate::net::pia) version: u8,
        pub(in crate::net::pia) latency_bits: u8,
        pub(in crate::net::pia) render_profile_settings_bits: u16,
    }
}

// ---------------------------------------------------------------------------
// v3 (released in v1.4.0 / v1.4.1)
// ---------------------------------------------------------------------------
mod v3 {
    use super::*;

    #[repr(C, packed)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Unaligned, Debug)]
    pub(super) struct Packet {
        pub version: u8,
        pub latency_bits: u8,
        pub render_profile_settings_bits: u16,
    }

    impl From<&Packet> for PiaPacket {
        fn from(p: &Packet) -> PiaPacket {
            let rps = LegacyRps::from_v2_or_v3_bits(
                p.render_profile_settings_bits,
                /* has_boost_bit = */ true,
            );
            PiaPacket {
                version: CURRENT_VERSION,
                latency_bits: p.latency_bits,
                render_profile_settings_bits: rps.to_v4_rps_bits(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// v2
// ---------------------------------------------------------------------------
mod v2 {
    use super::*;

    #[repr(C, packed)]
    #[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Unaligned, Debug)]
    pub(super) struct Packet {
        pub version: u8,
        pub latency_bits: u8,
        pub render_profile_settings_bits: u16,
    }

    impl From<&Packet> for PiaPacket {
        fn from(p: &Packet) -> PiaPacket {
            let rps = LegacyRps::from_v2_or_v3_bits(
                p.render_profile_settings_bits,
                /* has_boost_bit = */ false,
            );
            PiaPacket {
                version: CURRENT_VERSION,
                latency_bits: p.latency_bits,
                render_profile_settings_bits: rps.to_v4_rps_bits(),
            }
        }
    }
}
