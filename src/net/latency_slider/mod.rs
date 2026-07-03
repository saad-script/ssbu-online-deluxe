use std::cmp::Ordering as CmpOrdering;
use std::sync::atomic::{AtomicI8, Ordering};

use skyline::hooks::InlineCtx;

use crate::input_poll::InputSnapshot;
use crate::net;

const MAX_INPUT_BUFFER: u8 = 25;
const VALUE_UNKNOWN: i8 = i8::MIN;
const VALUE_AUTO: i8 = -1;

static LAST_AUTO: AtomicI8 = AtomicI8::new(VALUE_UNKNOWN);

#[derive(Debug)]
pub struct Latency {
    buffer: AtomicI8,
}

impl Latency {
    pub fn next(&self) {
        let prev_delay = self.buffer.load(Ordering::SeqCst);
        self.buffer.store(
            (prev_delay + 1).min(MAX_INPUT_BUFFER as i8),
            Ordering::SeqCst,
        );
    }
    pub fn prev(&self) {
        let prev_delay = self.buffer.load(Ordering::SeqCst);
        self.buffer
            .store((prev_delay - 1).max(VALUE_AUTO), Ordering::SeqCst);
    }
    pub fn to_bits(&self) -> u8 {
        self.buffer.load(Ordering::SeqCst) as u8
    }
    pub fn from_bits(bits: u8) -> Self {
        Latency {
            buffer: AtomicI8::new(bits as i8),
        }
    }
    fn ord_key(&self) -> (u8, i16) {
        let value = self.buffer.load(Ordering::SeqCst);
        if value >= 0 {
            return (0, value as i16);
        }
        if value == VALUE_AUTO {
            return (1, 0);
        }
        (2, value as i16)
    }
    pub const fn auto() -> Self {
        Latency {
            buffer: AtomicI8::new(VALUE_AUTO),
        }
    }
    pub const fn unknown() -> Self {
        Latency {
            buffer: AtomicI8::new(VALUE_UNKNOWN),
        }
    }
}

impl Clone for Latency {
    fn clone(&self) -> Self {
        Latency {
            buffer: AtomicI8::new(self.buffer.load(Ordering::SeqCst)),
        }
    }
}

impl PartialEq for Latency {
    fn eq(&self, other: &Self) -> bool {
        self.buffer.load(Ordering::SeqCst) == other.buffer.load(Ordering::SeqCst)
    }
}

impl Eq for Latency {}

impl PartialOrd for Latency {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for Latency {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.ord_key().cmp(&other.ord_key())
    }
}

impl ToString for Latency {
    fn to_string(&self) -> String {
        let buffer = self.buffer.load(Ordering::SeqCst);
        let last_auto = LAST_AUTO.load(Ordering::SeqCst);
        match (buffer >= 0, last_auto >= 0) {
            (false, false) => String::from("Auto"),
            (false, true) => format!("Auto ({}f)", last_auto).to_string(),
            (true, _) => format!("{}f", buffer).to_string(),
        }
    }
}

pub struct LatencySliderManager {
    selected_latency: Latency,
    active_latency: Latency,
}

impl LatencySliderManager {
    const fn new() -> Self {
        LatencySliderManager {
            selected_latency: Latency::auto(),
            active_latency: Latency::unknown(),
        }
    }
    pub fn instance() -> &'static Self {
        static LATENCY_SLIDER_MANAGER: LatencySliderManager = LatencySliderManager::new();
        &LATENCY_SLIDER_MANAGER
    }
    pub fn selected_latency(&self) -> &Latency {
        &self.selected_latency
    }
    pub fn active_latency(&self) -> Option<&Latency> {
        let buffer = self.active_latency.buffer.load(Ordering::SeqCst);
        if buffer == VALUE_UNKNOWN {
            return None;
        }
        Some(&self.active_latency)
    }
    pub fn poll(
        &self,
        input_snapshot: &InputSnapshot,
        prev: ninput::Buttons,
        next: ninput::Buttons,
    ) -> bool {
        let pressed_buttons = input_snapshot.check_buttons_pressed_with_cooldown(&[prev, next]);
        if pressed_buttons == prev {
            self.selected_latency.prev();
            return true;
        } else if pressed_buttons == next {
            self.selected_latency.next();
            return true;
        }
        false
    }
}

#[skyline::hook(offset = 0x16ccc58, inline)]
unsafe fn set_online_latency(ctx: &InlineCtx) {
    if net::is_valid_online_mode() {
        println!("SET ONLINE LATENCY");
        let auto = *(ctx.registers[19].x() as *mut u8);
        LAST_AUTO.store(auto as i8, Ordering::SeqCst);
        let buffer = LatencySliderManager::instance()
            .selected_latency
            .buffer
            .load(Ordering::SeqCst);
        LatencySliderManager::instance()
            .active_latency
            .buffer
            .store(buffer, Ordering::SeqCst);
        if buffer >= 0 {
            *(ctx.registers[19].x() as *mut u8) = buffer as u8;
        }
    }
}

pub(crate) fn match_cleanup() {
    LatencySliderManager::instance()
        .active_latency
        .buffer
        .store(VALUE_UNKNOWN, Ordering::SeqCst);
}

pub(super) fn install() {
    skyline::install_hook!(set_online_latency);
}
