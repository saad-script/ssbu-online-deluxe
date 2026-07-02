use std::sync::{
    atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering},
    Arc, LazyLock, Mutex,
};

use arc_swap::ArcSwap;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use ssbu_pia_interface::{
    self, ConnectedStation, ConnectionChangedEvent, StationConnectionManager,
};

use crate::{
    net::{
        is_in_real_game, is_valid_online_mode,
        latency_slider::{Latency, LatencySliderManager},
    },
    render::profile::{RenderProfile, RenderProfileManager, RenderProfileSettings},
};

const PIA_CUSTOM_COMMS_VERSION: u8 = 3;

#[derive(Debug)]
enum PiaCommsError {
    VersionMismatch,
    InvalidData,
}

static CONNECTED_STATION_TABLE_SYNCED_INTERNAL: LazyLock<Mutex<Vec<StationNetInfo>>> =
    LazyLock::new(|| Mutex::new(Vec::with_capacity(8)));
static CONNECTED_STATION_TABLE_ATOMIC_VIEW: LazyLock<ArcSwap<Vec<StationNetInfo>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::with_capacity(8)));

#[derive(Debug)]
struct StationNetInfo {
    id: u64,
    is_valid_comms: AtomicBool,
    latency_bits: AtomicU8,
    render_profile_settings_bits: AtomicU16,
}

impl Clone for StationNetInfo {
    fn clone(&self) -> Self {
        StationNetInfo {
            id: self.id,
            is_valid_comms: AtomicBool::new(self.is_valid_comms.load(Ordering::SeqCst)),
            latency_bits: AtomicU8::new(self.latency_bits.load(Ordering::SeqCst)),
            render_profile_settings_bits: AtomicU16::new(
                self.render_profile_settings_bits.load(Ordering::SeqCst),
            ),
        }
    }
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, IntoBytes, KnownLayout, Unaligned, Debug)]
struct PiaCustomNetPacket {
    version: u8,
    latency_bits: u8,
    render_profile_settings_bits: u16,
}

pub trait StationExt {
    fn get_latency(&self) -> Option<Latency>;
    fn get_render_profile_settings(&self) -> Option<RenderProfileSettings>;
    fn get_render_profile(&self) -> Option<RenderProfile>;
}

impl StationExt for ConnectedStation {
    fn get_latency(&self) -> Option<Latency> {
        let id = self.get_id();
        let stations_table = CONNECTED_STATION_TABLE_ATOMIC_VIEW.load();
        stations_table
            .iter()
            .find(|s| s.id == id && s.is_valid_comms.load(Ordering::SeqCst))
            .map(|s| Latency::from_bits(s.latency_bits.load(Ordering::SeqCst)))
    }
    fn get_render_profile_settings(&self) -> Option<RenderProfileSettings> {
        if !self.is_modded() {
            return None;
        }
        let id = self.get_id();
        let stations_table = CONNECTED_STATION_TABLE_ATOMIC_VIEW.load();
        stations_table
            .iter()
            .find(|s| s.id == id && s.is_valid_comms.load(Ordering::SeqCst))
            .and_then(|s| {
                RenderProfileSettings::from_bits(
                    s.render_profile_settings_bits.load(Ordering::SeqCst),
                )
            })
    }
    fn get_render_profile(&self) -> Option<RenderProfile> {
        self.get_render_profile_settings()
            .map(|rps| RenderProfile::from_settings(&rps))
    }
}

fn normalize_and_parse_data(data: &[u8]) -> Result<PiaCustomNetPacket, PiaCommsError> {
    let mut data = match PiaCustomNetPacket::read_from_bytes(data) {
        Ok(data) => data,
        Err(_) => return Err(PiaCommsError::InvalidData),
    };
    if data.version == PIA_CUSTOM_COMMS_VERSION {
        return Ok(data);
    } else if data.version == 2 {
        data.render_profile_settings_bits &= !(1 << 14);
        data.render_profile_settings_bits &= !(1 << 15);
        return Ok(data);
    }
    return Err(PiaCommsError::VersionMismatch);
}

fn on_station_connection_changed(
    event: ConnectionChangedEvent,
    station: ConnectedStation,
    new_num_connected: usize,
) {
    let id = station.get_id();
    let mut stations_table = CONNECTED_STATION_TABLE_SYNCED_INTERNAL.lock().unwrap();
    if event == ConnectionChangedEvent::StationConnected {
        stations_table.push(StationNetInfo {
            id,
            is_valid_comms: AtomicBool::new(false),
            latency_bits: AtomicU8::new(Latency::unknown().to_bits()),
            render_profile_settings_bits: AtomicU16::new(
                RenderProfileSettings::vanilla().to_bits(),
            ),
        });
    } else if event == ConnectionChangedEvent::StationDisconnected {
        if let Some(i) = stations_table.iter().position(|s| s.id == id) {
            stations_table.remove(i);
        }
    }

    CONNECTED_STATION_TABLE_ATOMIC_VIEW.store(Arc::new(stations_table.clone()));

    RenderProfileManager::instance()
        .auto_select_profile(is_valid_online_mode(), new_num_connected > 1);
}

fn send_pia_data_hook(_station: ConnectedStation, data: &mut [u8]) {
    for byte in data.iter_mut() {
        *byte = 0;
    }
}

fn receive_pia_data_hook(station: ConnectedStation, data: &[u8]) {
    let id = station.get_id();
    let stations_table = CONNECTED_STATION_TABLE_ATOMIC_VIEW.load();
    if let Some(station) = stations_table.iter().find(|s| s.id == id) {
        match normalize_and_parse_data(data) {
            Ok(d) => {
                station.is_valid_comms.store(true, Ordering::SeqCst);
                station.latency_bits.store(d.latency_bits, Ordering::SeqCst);
                station
                    .render_profile_settings_bits
                    .store(d.render_profile_settings_bits, Ordering::SeqCst);
            }
            Err(e) => {
                println!("Error parsing incoming data: {:?}", e);
            }
        }
    }
}

pub(super) fn install() {
    ssbu_pia_interface::install();
    StationConnectionManager::set_enabled(true);
    StationConnectionManager::register_station_connection_changed_callback(
        on_station_connection_changed,
    );
    StationConnectionManager::register_station_data_send_hook(send_pia_data_hook);
    StationConnectionManager::register_station_data_received_hook(receive_pia_data_hook);

    #[cfg(feature = "dummy_connection")]
    ssbu_pia_interface::setup_dummy_connection(3);
}
