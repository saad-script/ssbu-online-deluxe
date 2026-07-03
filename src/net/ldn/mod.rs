use std::sync::atomic::{AtomicU8, Ordering};

pub mod interface;

static LAST_SCANNED_NUM_NETWORKS: AtomicU8 = AtomicU8::new(0);

#[skyline::hook(replace = interface::initialize)]
fn on_initialized() {
    LAST_SCANNED_NUM_NETWORKS.store(0, Ordering::SeqCst);
    call_original!();
    println!("LDN Initialized");
}

#[skyline::hook(replace = interface::scan_network)]
unsafe fn on_scan_network(
    network_info: *mut interface::NetworkInfo,
    num_networks_found: *mut i32,
    max_num_networks: i32,
    scan_filter: u64,
    param_5: i32,
) {
    call_original!(
        network_info,
        num_networks_found,
        max_num_networks,
        scan_filter,
        param_5
    );
    println!("LDN Network Scan: {}", *num_networks_found);
    LAST_SCANNED_NUM_NETWORKS.store(*num_networks_found as u8, Ordering::SeqCst);
}

pub fn num_available_networks() -> u8 {
    let net_state = unsafe { interface::get_network_state() };
    match net_state {
        interface::NetworkState::Initialized => LAST_SCANNED_NUM_NETWORKS.load(Ordering::SeqCst),
        _ => 0,
    }
}

pub(super) fn install() {
    skyline::install_hooks!(on_initialized, on_scan_network);
}
