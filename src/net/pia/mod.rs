use std::{sync::LazyLock, time::Duration};

use arc_swap::ArcSwap;
use zerocopy::IntoBytes;

use ssbu_pia_interface::{
    self, Channel, ConnectedStation, ConnectionChangedEvent, StationConnectionManager,
};

use crate::{
    net::{
        is_in_real_game, is_valid_online_mode,
        latency_slider::{LatencySliderManager, StationLatency},
    },
    render::profile::{RenderProfile, RenderProfileManager},
};

mod version;

use version::StationState;

type PiaCustomNetPacket = version::PiaPacket;

const CHANNEL_NAME: &str = "ssbu-online-deluxe";
const NET_THREAD_TICK: Duration = Duration::from_millis(100);
const NET_THREAD_PRIORITY: i32 = 24;
static STATION_TABLE: LazyLock<ArcSwap<Vec<StationNetInfo>>> =
    LazyLock::new(|| ArcSwap::from_pointee(Vec::with_capacity(8)));

#[derive(Debug, Clone)]
struct StationNetInfo {
    id: u64,
    state: StationState,
}

fn with_station_state<R>(id: u64, f: impl FnOnce(&StationState) -> R) -> Option<R> {
    STATION_TABLE
        .load()
        .iter()
        .find(|s| s.id == id)
        .map(|s| f(&s.state))
}

pub trait StationExt {
    fn get_latency(&self) -> Option<StationLatency>;
    fn get_render_profile(&self) -> Option<RenderProfile>;
}

impl StationExt for ConnectedStation {
    fn get_latency(&self) -> Option<StationLatency> {
        with_station_state(self.get_id(), |s| s.latency)
    }
    fn get_render_profile(&self) -> Option<RenderProfile> {
        with_station_state(self.get_id(), |s| {
            RenderProfile::from_settings(&s.render_profile_settings)
        })
    }
}

fn on_station_connection_changed(
    event: ConnectionChangedEvent,
    station: ConnectedStation,
    new_num_connected: usize,
) {
    if event == ConnectionChangedEvent::StationDisconnected {
        let id = station.get_id();
        STATION_TABLE.rcu(|table| {
            table
                .iter()
                .filter(|s| s.id != id)
                .cloned()
                .collect::<Vec<_>>()
        });
    }

    RenderProfileManager::instance().refresh_profile(is_valid_online_mode(), new_num_connected > 1);
}

fn receive_station_states(channel: Channel, stations: &[ConnectedStation], buf: &mut [u8]) {
    let mut received = Vec::new();
    for station in stations {
        let mut latest = None;
        while let Some(len) = station.recv_packet(channel, buf) {
            match version::decode(&buf[..len]) {
                Ok(state) => latest = Some(state),
                Err(e) => println!(
                    "Error parsing incoming data from station {}: {:?}",
                    station.get_id(),
                    e
                ),
            }
        }
        if let Some(state) = latest {
            received.push(StationNetInfo {
                id: station.get_id(),
                state,
            });
        }
    }

    let connected_ids: Vec<u64> = stations.iter().map(|s| s.get_id()).collect();
    let has_stale = STATION_TABLE
        .load()
        .iter()
        .any(|s| !connected_ids.contains(&s.id));
    if received.is_empty() && !has_stale {
        return;
    }

    STATION_TABLE.rcu(|table| {
        let mut table: Vec<StationNetInfo> = table
            .iter()
            .filter(|s| connected_ids.contains(&s.id) && !received.iter().any(|r| r.id == s.id))
            .cloned()
            .collect();
        table.extend(received.iter().cloned());
        table
    });
}

fn send_local_state(channel: Channel, stations: &[ConnectedStation]) {
    let latency = LatencySliderManager::instance().station_latency();
    let rps = match is_in_real_game() {
        false => RenderProfileManager::instance().selected_render_profile_settings(),
        true => RenderProfileManager::active_render_profile_settings(),
    };
    let packet = PiaCustomNetPacket::new(latency, &rps);
    for station in stations.iter().filter(|s| s.is_modded()) {
        station.send_packet(channel, packet.as_bytes());
    }
}

fn net_thread_main(channel: Channel) {
    unsafe {
        skyline::nn::os::ChangeThreadPriority(
            skyline::nn::os::GetCurrentThread(),
            NET_THREAD_PRIORITY,
        );
    }
    let mut buf = vec![0u8; StationConnectionManager::max_packet_size()];
    loop {
        std::thread::sleep(NET_THREAD_TICK);
        let stations = StationConnectionManager::get_connected_stations();
        receive_station_states(channel, &stations, &mut buf);
        if !stations.is_empty() {
            send_local_state(channel, &stations);
        }
    }
}

#[cfg(feature = "dummy_connection")]
fn dummy_connection_count() -> usize {
    option_env!("DUMMY_CONNECTIONS")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3)
}

pub(super) fn install() {
    ssbu_pia_interface::install();
    StationConnectionManager::set_enabled(true);
    StationConnectionManager::register_station_connection_changed_callback(
        on_station_connection_changed,
    );

    match Channel::claim(CHANNEL_NAME) {
        Some(channel) => {
            std::thread::Builder::new()
                .name(String::from("pia-net"))
                .stack_size(0x10000)
                .spawn(move || net_thread_main(channel))
                .expect("Unable to spawn PIA network thread!");
        }
        None => println!(
            "Unable to claim PIA channel '{}', station info will not be exchanged",
            CHANNEL_NAME
        ),
    }

    #[cfg(feature = "dummy_connection")]
    ssbu_pia_interface::setup_dummy_connection(dummy_connection_count());
}
