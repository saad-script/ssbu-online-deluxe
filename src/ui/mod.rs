use crate::input_poll::{InputSnapshot, Poller, PollerContext};

pub mod native;
pub mod overlay;

static POLLER_CONTEXT: PollerContext = PollerContext::new(std::time::Duration::from_millis(167));
pub static OVERLAY_POLLER: Poller = Poller::new(&POLLER_CONTEXT);
pub static NATIVE_POLLER: Poller = Poller::new(&POLLER_CONTEXT);

pub fn check_overlay_toggle_buttons_pressed(input_snapshot: &InputSnapshot) -> ninput::Buttons {
    input_snapshot.check_buttons_pressed(&[
        ninput::Buttons::ZL | ninput::Buttons::ZR | ninput::Buttons::DOWN,
        ninput::Buttons::L | ninput::Buttons::R | ninput::Buttons::DOWN,
    ])
}

pub(super) fn install() {
    overlay::install();
}
