//! Gamepads (gilrs) translated to the original keyboard controls, without guest patches.
use crate::rg_log;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use std::collections::HashMap;

/// DOS scan codes of the keys the pad presses (extended keys carry bit 8).
mod scan {
    pub const SPACE: u16 = 0x39;
    pub const ESCAPE: u16 = 0x01;
    pub const LEFT_CTRL: u16 = 0x1d;
    pub const S: u16 = 0x1f;
    pub const I: u16 = 0x17;
    pub const ENTER: u16 = 0x1c;
    pub const LEFT_SHIFT: u16 = 0x2a;
    pub const LEFT_ALT: u16 = 0x38;
    pub const UP: u16 = 0x148;
    pub const DOWN: u16 = 0x150;
    pub const LEFT: u16 = 0x14b;
    pub const RIGHT: u16 = 0x14d;
}

/// Pad buttons and the original keys they press, in Xbox names: A jumps, X attacks, Y draws the
/// sword, Back opens the inventory, Start and B go back.
pub(super) const BUTTON_KEYS: &[(Button, u16)] = &[
    (Button::South, scan::SPACE),
    (Button::East, scan::ESCAPE),
    (Button::West, scan::LEFT_CTRL),
    (Button::North, scan::S),
    (Button::Select, scan::I),
    (Button::Start, scan::ESCAPE),
    (Button::LeftThumb, scan::ENTER),
    (Button::LeftTrigger, scan::LEFT_SHIFT),
    (Button::RightTrigger, scan::LEFT_ALT),
    (Button::DPadUp, scan::UP),
    (Button::DPadDown, scan::DOWN),
    (Button::DPadLeft, scan::LEFT),
    (Button::DPadRight, scan::RIGHT),
];

/// The stick as a digital direction: on past 50%, off again below 37.5%, so it neither drifts
/// nor chatters at the edge.
pub(super) fn direction(value: f32, previous: i8) -> i8 {
    if value <= -0.5 || (previous == -1 && value < -0.375) {
        -1
    } else if value >= 0.5 || (previous == 1 && value > 0.375) {
        1
    } else {
        0
    }
}

#[derive(Default, Clone, Copy)]
pub(super) struct Pad {
    pub(super) buttons: u32,
    pub(super) x: i8,
    /// Up is -1, as on the arrow keys.
    pub(super) y: i8,
}

impl Pad {
    pub(super) fn press(&mut self, button: Button, down: bool) -> bool {
        let Some(index) = BUTTON_KEYS.iter().position(|(b, _)| *b == button) else {
            return false;
        };
        if down {
            self.buttons |= 1 << index;
        } else {
            self.buttons &= !(1 << index);
        }
        true
    }

    pub(super) fn keys(&self, keys: &mut [bool; 512]) {
        for (index, (_, key)) in BUTTON_KEYS.iter().enumerate() {
            if self.buttons & (1 << index) != 0 {
                keys[usize::from(*key)] = true;
            }
        }
        let pick = |value: i8, negative: u16, positive: u16| match value {
            -1 => Some(negative),
            1 => Some(positive),
            _ => None,
        };
        for key in [
            pick(self.x, scan::LEFT, scan::RIGHT),
            pick(self.y, scan::UP, scan::DOWN),
        ]
        .into_iter()
        .flatten()
        {
            keys[usize::from(key)] = true;
        }
    }
}

pub(super) struct Controllers {
    gilrs: Gilrs,
    pads: HashMap<GamepadId, Pad>,
}

impl Controllers {
    pub(super) fn new() -> Result<Self, String> {
        let gilrs = Gilrs::new().map_err(|error| error.to_string())?;
        let mut controllers = Self {
            gilrs,
            pads: HashMap::new(),
        };
        let ids: Vec<_> = controllers.gilrs.gamepads().map(|(id, _)| id).collect();
        for id in ids {
            controllers.connect(id);
        }
        Ok(controllers)
    }

    fn connect(&mut self, id: GamepadId) {
        rg_log!(
            "Native controller connected: {}",
            self.gilrs.gamepad(id).name()
        );
        self.pads.insert(id, Pad::default());
        self.refresh_pad(id);
    }

    fn refresh_pad(&mut self, id: GamepadId) {
        let gamepad = self.gilrs.gamepad(id);
        let mut pad = Pad::default();
        for &(button, _) in BUTTON_KEYS {
            pad.press(button, gamepad.is_pressed(button));
        }
        pad.x = direction(gamepad.value(Axis::LeftStickX), 0);
        // gilrs reports up as positive.
        pad.y = direction(-gamepad.value(Axis::LeftStickY), 0);
        self.pads.insert(id, pad);
    }

    /// Re-reads every pad, for when the window regains focus.
    pub(super) fn refresh(&mut self) {
        let ids: Vec<_> = self.pads.keys().copied().collect();
        for id in ids {
            self.refresh_pad(id);
        }
    }

    pub(super) fn keys(&self) -> [bool; 512] {
        let mut keys = [false; 512];
        for pad in self.pads.values() {
            pad.keys(&mut keys);
        }
        keys
    }

    /// Applies the pending events in order (polling only the final state would lose short taps);
    /// whether any pad changed.
    pub(super) fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Some(event) = self.gilrs.next_event() {
            let id = event.id;
            match event.event {
                EventType::Connected => self.connect(id),
                EventType::Disconnected => {
                    self.pads.remove(&id);
                }
                EventType::ButtonPressed(button, _) | EventType::ButtonReleased(button, _) => {
                    let down = matches!(event.event, EventType::ButtonPressed(..));
                    if let Some(pad) = self.pads.get_mut(&id)
                        && !pad.press(button, down)
                    {
                        continue;
                    }
                }
                EventType::AxisChanged(Axis::LeftStickX, value, _) => {
                    if let Some(pad) = self.pads.get_mut(&id) {
                        pad.x = direction(value, pad.x);
                    }
                }
                EventType::AxisChanged(Axis::LeftStickY, value, _) => {
                    if let Some(pad) = self.pads.get_mut(&id) {
                        pad.y = direction(-value, pad.y);
                    }
                }
                _ => continue,
            }
            changed = true;
        }
        changed
    }
}
