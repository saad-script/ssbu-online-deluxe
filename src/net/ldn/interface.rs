#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use ssbu_pia_interface::NetworkStability;
use ultelier::sync_guest;

use crate::net::latency_slider::{Latency, LatencySliderManager};
use crate::utils;

extern "C" {
    #[link_name = "\u{1}_ZN2nn3ldn10InitializeEv"]
    pub fn initialize();

    #[link_name = "\u{1}_ZN2nn3ldn8FinalizeEv"]
    pub fn finalize();

    #[link_name = "\u{1}_ZN2nn3ldn11OpenStationEv"]
    pub fn open_station();

    #[link_name = "\u{1}_ZN2nn3ldn12CloseStationEv"]
    pub fn close_station();

    #[link_name = "\u{1}_ZN2nn3ldn14GetNetworkInfoEPNS0_11NetworkInfoE"]
    pub fn get_network_info(out_buffer_ptr: *mut NetworkInfo);

    #[link_name = "\u{1}_ZN2nn3ldn13CreateNetworkERKNS0_13NetworkConfigERKNS0_14SecurityConfigERKNS0_10UserConfigE"]
    pub fn create_network(network_config: u64, security_config: u64, user_config: u64);

    #[link_name = "\u{1}_ZN2nn3ldn7ConnectERKNS0_11NetworkInfoERKNS0_14SecurityConfigERKNS0_10UserConfigEiNS0_13ConnectOptionE"]
    pub fn connect_network(
        network_info: *mut NetworkInfo,
        security_config: u64,
        user_config: u64,
        local_comm_ver: i32,
        connect_option: u32,
    );

    #[link_name = "\u{1}_ZN2nn3ldn10DisconnectEv"]
    pub fn disconnect_network();

    #[link_name = "\u{1}_ZN2nn3ldn14DestroyNetworkEv"]
    pub fn destroy_network();

    #[link_name = "\u{1}_ZN2nn3ldn8GetStateEv"]
    pub fn get_network_state() -> NetworkState;

    #[link_name = "\u{1}_ZN2nn3ldn4ScanEPNS0_11NetworkInfoEPiiRKNS0_10ScanFilterEi"]
    pub fn scan_network(
        network_info: *mut NetworkInfo,
        num_networks_found: *mut i32,
        max_num_networks: i32,
        scan_filter: u64,
        _param_5: i32,
    );

    #[link_name = "\u{1}_ZN2nn3ldn14GetIpv4AddressEPNS0_11Ipv4AddressEPNS0_10SubnetMaskE"]
    pub fn get_ipv4_address(ip_address: *mut RawIPv4Address, subnet_mask: *mut RawIPv4Address);
}

pub fn get_network_address(port: u16) -> SocketAddr {
    let ip_buffer = [0; 4].as_mut_ptr() as *mut RawIPv4Address;
    let subnet_buffer = [0; 4].as_mut_ptr() as *mut RawIPv4Address;
    unsafe {
        get_ipv4_address(ip_buffer, subnet_buffer);
        let ip = (*ip_buffer).to_socket_address(port);
        return ip;
    }
}

#[repr(C, align(4))]
#[derive(Debug, Clone)]
pub struct RawIPv4Address(pub [u8; 4]);

impl RawIPv4Address {
    pub fn to_socket_address(&self, port: u16) -> SocketAddr {
        SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(self.0[3], self.0[2], self.0[1], self.0[0])),
            port,
        )
    }
}

pub fn try_get_network_info() -> std::io::Result<Box<NetworkInfo>> {
    unsafe {
        let state = get_network_state();
        match state {
            NetworkState::AccessPointCreated | NetworkState::StationConnected => {
                let mut network_info_buffer: [u8; 0x480] = [0; 0x480];
                let network_info_buffer = network_info_buffer.as_mut_ptr() as *mut NetworkInfo;

                get_network_info(network_info_buffer);
                Ok(Box::from_raw(network_info_buffer))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Unable to get network info",
            )),
        }
    }
}

pub fn get_network_role() -> NetworkRole {
    unsafe {
        let state = get_network_state();
        match state {
            NetworkState::AccessPointCreated => NetworkRole::Host,
            NetworkState::StationConnected => NetworkRole::Client,
            _ => NetworkRole::None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub ipv4_address: [u8; 4],            // 0x0, 4 bytes
    pub mac_address: [u8; 6],             // 0x4, 6 bytes
    pub node_id: u8,                      // 0xA, 1 byte
    pub is_connected: u8,                 // 0xB, 1 byte
    pub user_name: [u8; 33],              // 0xC, 33 bytes (0x21 in decimal)
    _reserved_1: u8,                      // 0x2D, 1 byte
    pub local_communication_version: u16, // 0x2E, 2 bytes
    _reserved_2: [u8; 16],                // 0x30, 16 bytes
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub local_communication_id: u64,        // 0x0, 8 bytes
    _reserved_1: u16,                       // 0x8, 2 bytes
    pub arbitrary_user_data: u16,           // 0xA, 2 bytes
    _reserved_2: u32,                       // 0xC, 4 bytes
    pub security_parameter_last: [u8; 16],  // 0x10, 16 bytes
    pub mac_address: [u8; 6],               // 0x20, 6 bytes
    pub ssid: [u8; 34],                     // 0x26, 34 bytes (0x22 in decimal)
    pub network_channel: i16,               // 0x48, 2 bytes
    pub link_level: i8,                     // 0x4A, 1 byte
    pub action_frame_indicator: u8,         // 0x4B, 1 byte
    _padding_1: [u8; 4],                    // 0x4C, 4 bytes
    pub security_parameter_first: [u8; 16], // 0x50, 16 bytes
    pub same_as_security_config: u16,       // 0x60, 2 bytes
    pub accept_policy: u8,                  // 0x62, 1 byte
    pub action_frame_set: u8,               // 0x63, 1 byte
    _padding_2: [u8; 2],                    // 0x64, 2 bytes
    pub maximum_participants: u8,           // 0x66, 1 byte
    pub participant_num: u8,                // 0x67, 1 byte
    pub node_info_array: [NodeInfo; 8],     // 0x68, 0x200 (512 bytes, 8 * 64 bytes each)
    _reserved_3: [u8; 2],                   // 0x268, 2 bytes
    pub advertise_data_size: u16,           // 0x26A, 2 bytes
    pub advertise_data: [u8; 384],          // 0x26C, 0x180 (384 bytes)
    _reserved_4: [u8; 140],                 // 0x3EC, 140 bytes
    pub random_authentication_id: [u8; 8],  // 0x478, 8 bytes
}

#[repr(u32)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum NetworkState {
    None = 0,
    Initialized = 1,
    AccessPoint = 2,
    AccessPointCreated = 3,
    Station = 4,
    StationConnected = 5,
    Error = 6,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkRole {
    None,
    Host,
    Client,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkThreadType {
    Listener,
    Sender,
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkPacketType {
    Ping = 0,
    Pong = 1,
}

#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub packet_type: NetworkPacketType,
    pub delay: Latency,
    pub less_lag_enabled: bool,
    timestamp: u64,
}

impl NetworkPacket {
    pub fn create_ping_packet() -> Self {
        let timestamp: u64;
        unsafe {
            timestamp = skyline::nn::os::GetSystemTick() as u64;
        }
        NetworkPacket {
            packet_type: NetworkPacketType::Ping,
            delay: LatencySliderManager::instance().selected_latency().clone(),
            less_lag_enabled: !sync_guest::is_vanilla_runtime().unwrap_or(true),
            timestamp,
        }
    }

    pub fn create_pong_packet(packet: &NetworkPacket) -> Self {
        NetworkPacket {
            packet_type: NetworkPacketType::Pong,
            delay: LatencySliderManager::instance().selected_latency().clone(),
            less_lag_enabled: !sync_guest::is_vanilla_runtime().unwrap_or(true),
            timestamp: packet.timestamp,
        }
    }

    pub fn get_time_elapsed(&self) -> Duration {
        utils::duration_since_tick(self.timestamp)
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        unsafe { std::ptr::read(bytes.as_ptr() as *const Self) }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkDiagnostics {
    pings: [u64; 100],
    counter: usize,
    sum: u64,
    filled: bool,
}

impl NetworkDiagnostics {
    pub const fn new() -> Self {
        NetworkDiagnostics {
            pings: [0; 100],
            counter: 0,
            sum: 0,
            filled: false,
        }
    }

    pub fn register_ping(&mut self, ping: u64) {
        self.sum -= self.pings[self.counter];
        self.sum += ping;
        self.pings[self.counter] = ping;
        self.counter = (self.counter + 1) % 100;
        if self.counter == 0 {
            self.filled = true;
        }
    }

    pub fn get_avg_ping(&self) -> Option<u64> {
        match (self.filled, self.counter == 0) {
            (true, _) => Some(self.sum / 100),
            (false, false) => Some(self.sum / self.counter as u64),
            (false, true) => None,
        }
    }

    pub fn get_network_stability(&self) -> NetworkStability {
        let avg = match self.get_avg_ping() {
            Some(a) => a,
            None => {
                return NetworkStability::Stable;
            }
        };

        let mut var_sum: u64 = 0;
        let end = if self.filled { 100 } else { self.counter + 1 };
        for i in 0..end {
            var_sum += (self.pings[i] - avg) * (self.pings[i] - avg)
        }

        let variance = if self.filled {
            var_sum as f64 / 99.0
        } else {
            var_sum as f64 / self.counter as f64
        };

        if variance <= 25.0 {
            return NetworkStability::Stable;
        } else if variance <= 100.0 {
            return NetworkStability::Inconsistent;
        } else {
            return NetworkStability::Unstable;
        }
    }

    pub fn reset(&mut self) {
        self.pings = [0; 100];
        self.counter = 0;
        self.sum = 0;
        self.filled = false;
    }
}

#[derive(Debug)]
pub struct PlayerNetInfo {
    connected: AtomicBool,
    pub delay: Latency,
    pub less_lag_enabled: AtomicBool,
    pub net_diagnostics: Mutex<NetworkDiagnostics>,
}

impl PlayerNetInfo {
    pub const fn default() -> Self {
        PlayerNetInfo {
            connected: AtomicBool::new(false),
            delay: Latency::auto(),
            less_lag_enabled: AtomicBool::new(false),
            net_diagnostics: Mutex::new(NetworkDiagnostics::new()),
        }
    }

    pub fn set_connected(&self, connected: bool) {
        if let Ok(_updated) = self.connected.compare_exchange(
            !connected,
            connected,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            if !connected {
                self.net_diagnostics.lock().unwrap().reset();
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        return self.connected.load(Ordering::SeqCst);
    }
}

pub trait UdpSocketExt {
    fn read(&self, buf: &mut [u8], blocking: bool) -> std::io::Result<(NetworkPacket, SocketAddr)>;
    fn write(&self, addr: &SocketAddr, packet: NetworkPacket) -> std::io::Result<usize>;
}

impl UdpSocketExt for UdpSocket {
    fn read(
        &self,
        buf: &mut [u8],
        blocking: bool,
    ) -> std::io::Result<(NetworkPacket, std::net::SocketAddr)> {
        self.set_nonblocking(!blocking)?;
        let (num_bytes, src_addr) = match self.recv_from(buf) {
            Ok((num_bytes, src_addr)) => (num_bytes, src_addr),
            Err(e) => {
                if e.raw_os_error() == Some(11) {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                } else {
                    return Err(e);
                }
            }
        };

        Ok((NetworkPacket::from_bytes(&buf[0..num_bytes]), src_addr))
    }

    fn write(&self, addr: &SocketAddr, packet: NetworkPacket) -> std::io::Result<usize> {
        self.send_to(packet.to_bytes(), addr)
    }
}
