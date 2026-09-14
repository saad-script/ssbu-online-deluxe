use crate::{
    input::{InputSnapshot, Poller, PollerContext},
    CONFIG,
};

pub mod native;
pub mod overlay;

static POLLER_CONTEXT: PollerContext = PollerContext::new(std::time::Duration::from_millis(167));
pub static OVERLAY_POLLER: Poller = Poller::new(&POLLER_CONTEXT);
pub static NATIVE_POLLER: Poller = Poller::new(&POLLER_CONTEXT);

static mut TOGGLE_OVERLAY_KEYBIND: ninput::Buttons = ninput::Buttons::empty();

pub fn check_overlay_toggle_buttons_pressed(
    input_snapshot: &InputSnapshot,
    cooldown: bool,
) -> bool {
    if cooldown {
        input_snapshot.is_pressed_with_cooldown(unsafe { TOGGLE_OVERLAY_KEYBIND })
    } else {
        input_snapshot.is_pressed(unsafe { TOGGLE_OVERLAY_KEYBIND })
    }
}

pub(super) fn install() {
    unsafe {
        TOGGLE_OVERLAY_KEYBIND = CONFIG.load().key_binding_config.toggle_overlay;
    }
    native::install();
    overlay::install();
}
