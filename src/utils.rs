#![allow(dead_code)]

use skyline::nn::os::Tick;
use std::time::Duration;

extern "C" {
    #[link_name = "\u{1}_ZN2nn2os22GetSystemTickFrequencyEv"]
    fn get_tick_freq_internal() -> u64;
}

#[inline]
pub fn get_tick_frequency() -> u64 {
    unsafe { get_tick_freq_internal() }
}

#[inline]
pub fn duration_since_tick(tick: Tick) -> Duration {
    unsafe {
        let elapsed_ticks = skyline::nn::os::GetSystemTick() - tick;
        Duration::from_secs_f64(elapsed_ticks as f64 / get_tick_freq_internal() as f64)
    }
}

#[inline]
pub fn is_emulator() -> bool {
    unsafe {
        let base_address = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
        return base_address == 0x80004000 || base_address == 0x8504000;
    }
}

#[inline]
pub fn lookup_symbol_addr(symbol: &'static [u8]) -> Option<usize> {
    let mut addr = 0usize;
    unsafe {
        if skyline::nn::ro::LookupSymbol(&mut addr, symbol.as_ptr()) == 0 && addr != 0 {
            Some(addr)
        } else {
            None
        }
    }
}
