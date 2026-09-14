#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer};

use crate::utils::duration_since_tick;

fn str_to_button(button_str: &str) -> Result<ninput::Buttons, ()> {
    match button_str {
        "A" => Ok(ninput::Buttons::A),
        "B" => Ok(ninput::Buttons::B),
        "X" => Ok(ninput::Buttons::X),
        "Y" => Ok(ninput::Buttons::Y),
        "UP" => Ok(ninput::Buttons::UP),
        "DOWN" => Ok(ninput::Buttons::DOWN),
        "LEFT" => Ok(ninput::Buttons::LEFT),
        "RIGHT" => Ok(ninput::Buttons::RIGHT),
        "MINUS" => Ok(ninput::Buttons::MINUS),
        "PLUS" | "START" => Ok(ninput::Buttons::PLUS),
        "L" => Ok(ninput::Buttons::L),
        "R" => Ok(ninput::Buttons::R),
        "ZL" => Ok(ninput::Buttons::ZL),
        "ZR" | "Z" => Ok(ninput::Buttons::ZR),
        "STICK_L" => Ok(ninput::Buttons::STICK_L),
        "STICK_R" => Ok(ninput::Buttons::STICK_R),
        _ => Err(()),
    }
}

fn str_to_button_combo(button_combo_str: &str) -> Result<ninput::Buttons, ()> {
    let button_strs = button_combo_str.split("+");
    let mut button_combo = ninput::Buttons::empty();
    for button_str in button_strs {
        button_combo |= str_to_button(button_str)?;
    }
    Ok(button_combo)
}

fn normalize_key_binding_str(s: &str) -> String {
    s.to_uppercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[derive(Debug, Clone)]
pub struct KeyBindingConfig {
    pub toggle_overlay: ninput::Buttons,
    pub toggle_fps_boost: ninput::Buttons,
    pub toggle_streamer_mode: ninput::Buttons,
}

impl Default for KeyBindingConfig {
    fn default() -> Self {
        Self {
            toggle_overlay: ninput::Buttons::RIGHT | ninput::Buttons::L | ninput::Buttons::R,
            toggle_fps_boost: ninput::Buttons::X
                | ninput::Buttons::L
                | ninput::Buttons::R
                | ninput::Buttons::ZR,
            toggle_streamer_mode: ninput::Buttons::Y
                | ninput::Buttons::L
                | ninput::Buttons::R
                | ninput::Buttons::ZR,
        }
    }
}

#[derive(Deserialize)]
struct RawKeyBindingConfig {
    toggle_overlay: String,
    toggle_fps_boost: String,
    toggle_streamer_mode: String,
}

impl<'de> Deserialize<'de> for KeyBindingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawKeyBindingConfig::deserialize(deserializer)?;

        let toggle_overlay = normalize_key_binding_str(&raw.toggle_overlay);
        let toggle_overlay = str_to_button_combo(toggle_overlay.as_str()).map_err(|_| {
            serde::de::Error::custom(format!(
                "invalid toggle_overlay key binding: {:?}",
                raw.toggle_overlay
            ))
        })?;

        let toggle_fps_boost = normalize_key_binding_str(&raw.toggle_fps_boost);
        let toggle_fps_boost = str_to_button_combo(toggle_fps_boost.as_str()).map_err(|_| {
            serde::de::Error::custom(format!(
                "invalid toggle_fps_boost key binding: {:?}",
                raw.toggle_fps_boost
            ))
        })?;

        let toggle_streamer_mode = normalize_key_binding_str(&raw.toggle_streamer_mode);
        let toggle_streamer_mode =
            str_to_button_combo(toggle_streamer_mode.as_str()).map_err(|_| {
                serde::de::Error::custom(format!(
                    "invalid toggle_streamer_mode key binding: {:?}",
                    raw.toggle_fps_boost
                ))
            })?;

        Ok(Self {
            toggle_overlay,
            toggle_fps_boost,
            toggle_streamer_mode,
        })
    }
}

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

    pub fn snapshot(&self) -> InputSnapshot<'_> {
        InputSnapshot {
            pressed_btns: unsafe {
                ninput::Buttons::from_bits_unchecked(self.pressed_btns.load(Ordering::SeqCst))
            },
            context: self.context,
        }
    }
}
