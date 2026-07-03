use std::sync::atomic::{AtomicU64, Ordering};

use crate::utils::duration_since_tick;

pub struct InputSnapshot<'a> {
    pressed_btns: ninput::Buttons,
    context: &'a PollerContext,
}

impl InputSnapshot<'_> {
    pub fn is_pressed(&self, buttons: ninput::Buttons) -> bool {
        self.pressed_btns.contains(buttons)
    }

    pub fn is_pressed_any(&self, buttons: ninput::Buttons) -> bool {
        self.pressed_btns.intersects(buttons)
    }

    pub fn check_buttons_pressed(&self, buttons: &[ninput::Buttons]) -> ninput::Buttons {
        for b in buttons {
            if self.pressed_btns.contains(*b) {
                return *b;
            }
        }
        return ninput::Buttons::empty();
    }

    pub fn is_pressed_with_cooldown(&self, buttons: ninput::Buttons) -> bool {
        let pressed = self.is_pressed(buttons);
        if !pressed {
            return false;
        }

        let last_tick = self.context.last_trigger_tick.load(Ordering::SeqCst);
        if last_tick != 0 {
            if duration_since_tick(last_tick) < self.context.cooldown {
                return false;
            }
        }

        let now_tick = unsafe { skyline::nn::os::GetSystemTick() };
        self.context
            .last_trigger_tick
            .store(now_tick, Ordering::SeqCst);
        true
    }

    pub fn is_pressed_any_with_cooldown(&self, buttons: ninput::Buttons) -> bool {
        let pressed = self.is_pressed_any(buttons);
        if !pressed {
            return false;
        }

        let last_tick = self.context.last_trigger_tick.load(Ordering::SeqCst);
        if last_tick != 0 {
            if duration_since_tick(last_tick) < self.context.cooldown {
                return false;
            }
        }

        let now_tick = unsafe { skyline::nn::os::GetSystemTick() };
        self.context
            .last_trigger_tick
            .store(now_tick, Ordering::SeqCst);
        true
    }

    pub fn check_buttons_pressed_with_cooldown(
        &self,
        buttons: &[ninput::Buttons],
    ) -> ninput::Buttons {
        let pressed = self.check_buttons_pressed(buttons);
        if pressed.is_empty() {
            return ninput::Buttons::empty();
        }

        let last_tick = self.context.last_trigger_tick.load(Ordering::SeqCst);
        if last_tick != 0 {
            if duration_since_tick(last_tick) < self.context.cooldown {
                return ninput::Buttons::empty();
            }
        }

        let now_tick = unsafe { skyline::nn::os::GetSystemTick() };
        self.context
            .last_trigger_tick
            .store(now_tick, Ordering::SeqCst);
        pressed
    }
}

pub struct PollerContext {
    last_trigger_tick: AtomicU64,
    cooldown: std::time::Duration,
}

impl PollerContext {
    pub const fn new(cooldown: std::time::Duration) -> Self {
        PollerContext {
            last_trigger_tick: AtomicU64::new(0),
            cooldown,
        }
    }
}

pub struct Poller<'a> {
    pressed_btns: AtomicU64,
    context: &'a PollerContext,
}

impl Poller<'_> {
    pub const fn new<'a>(context: &'a PollerContext) -> Poller<'a> {
        Poller {
            pressed_btns: AtomicU64::new(ninput::Buttons::empty().bits()),
            context,
        }
    }

    pub fn poll(&self) {
        self.pressed_btns
            .store(ninput::any::combined_buttons().bits(), Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.pressed_btns
            .store(ninput::Buttons::empty().bits(), Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> InputSnapshot<'_> {
        InputSnapshot {
            pressed_btns: unsafe {
                ninput::Buttons::from_bits_unchecked(self.pressed_btns.load(Ordering::SeqCst))
            },
            context: self.context,
        }
    }
}
