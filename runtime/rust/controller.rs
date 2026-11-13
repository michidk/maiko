//! SDL gamepads translated to the original keyboard controls, without guest patches.
use super::dos_scan;
use sdl2::GameControllerSubsystem;
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::keyboard::Scancode;

const BUTTON_KEYS: &[(Button, Scancode)] = &[
    (Button::A, Scancode::Space),
    (Button::B, Scancode::Escape),
    (Button::X, Scancode::LCtrl),
    (Button::Y, Scancode::S),
    (Button::Back, Scancode::I),
    (Button::Start, Scancode::Escape),
    (Button::LeftStick, Scancode::Return),
    (Button::LeftShoulder, Scancode::LShift),
    (Button::RightShoulder, Scancode::LAlt),
    (Button::DPadUp, Scancode::Up),
    (Button::DPadDown, Scancode::Down),
    (Button::DPadLeft, Scancode::Left),
    (Button::DPadRight, Scancode::Right),
];

// Separate activation and release thresholds prevent drift and edge chatter.
fn direction(value: i16, previous: i8) -> i8 {
    if value <= -16384 || (previous == -1 && value < -12288) {
        -1
    } else if value >= 16384 || (previous == 1 && value > 12288) {
        1
    } else {
        0
    }
}

struct Device {
    handle: GameController,
    buttons: u32,
    x: i8,
    y: i8,
}

impl Device {
    fn refresh(&mut self) {
        self.buttons = 0;
        for &(button, _) in BUTTON_KEYS {
            if self.handle.button(button) {
                self.buttons |= 1 << button as u32;
            }
        }
        self.x = direction(self.handle.axis(Axis::LeftX), 0);
        self.y = direction(self.handle.axis(Axis::LeftY), 0);
    }

    fn keys(&self, keys: &mut [bool; 512]) {
        for &(button, key) in BUTTON_KEYS {
            if self.buttons & (1 << button as u32) != 0 {
                keys[usize::from(dos_scan(key))] = true;
            }
        }
        for key in [
            match self.x {
                -1 => Some(Scancode::Left),
                1 => Some(Scancode::Right),
                _ => None,
            },
            match self.y {
                -1 => Some(Scancode::Up),
                1 => Some(Scancode::Down),
                _ => None,
            },
        ]
        .into_iter()
        .flatten()
        {
            keys[usize::from(dos_scan(key))] = true;
        }
    }
}

pub(super) struct Controllers {
    subsystem: GameControllerSubsystem,
    devices: Vec<Device>,
}

impl Controllers {
    pub(super) fn new(sdl: &sdl2::Sdl) -> Result<Self, String> {
        let subsystem = sdl.game_controller()?;
        let count = subsystem.num_joysticks()?;
        let mut controllers = Self {
            subsystem,
            devices: Vec::new(),
        };
        for index in 0..count {
            controllers.open(index);
        }
        Ok(controllers)
    }

    fn open(&mut self, index: u32) {
        if !self.subsystem.is_game_controller(index) {
            return;
        }
        match self.subsystem.open(index) {
            Ok(handle) => {
                if self
                    .devices
                    .iter()
                    .any(|device| device.handle.instance_id() == handle.instance_id())
                {
                    return;
                }
                eprintln!("Native controller connected: {}", handle.name());
                let mut device = Device {
                    handle,
                    buttons: 0,
                    x: 0,
                    y: 0,
                };
                device.refresh();
                self.devices.push(device);
            }
            Err(error) => eprintln!("Native controller could not open: {error}"),
        }
    }

    pub(super) fn refresh(&mut self) {
        for device in &mut self.devices {
            device.refresh();
        }
    }

    pub(super) fn keys(&self) -> [bool; 512] {
        let mut keys = [false; 512];
        for device in &self.devices {
            device.keys(&mut keys);
        }
        keys
    }

    // Process events in order: polling only the final state would lose short taps.
    pub(super) fn event(&mut self, event: &Event) -> bool {
        let id = match *event {
            Event::ControllerDeviceAdded { which, .. } => {
                // Added events carry an enumeration index, all others an instance ID.
                self.open(which);
                return true;
            }
            Event::ControllerDeviceRemoved { which, .. } => {
                self.devices
                    .retain(|device| device.handle.instance_id() != which);
                return true;
            }
            Event::ControllerButtonDown { which, .. }
            | Event::ControllerButtonUp { which, .. }
            | Event::ControllerAxisMotion { which, .. }
            | Event::ControllerDeviceRemapped { which, .. } => which,
            _ => return false,
        };
        let Some(device) = self
            .devices
            .iter_mut()
            .find(|device| device.handle.instance_id() == id)
        else {
            return false;
        };
        match *event {
            Event::ControllerButtonDown { button, .. } => device.buttons |= 1 << button as u32,
            Event::ControllerButtonUp { button, .. } => device.buttons &= !(1 << button as u32),
            Event::ControllerAxisMotion {
                axis: Axis::LeftX,
                value,
                ..
            } => device.x = direction(value, device.x),
            Event::ControllerAxisMotion {
                axis: Axis::LeftY,
                value,
                ..
            } => device.y = direction(value, device.y),
            Event::ControllerDeviceRemapped { .. } => device.refresh(),
            _ => return false,
        }
        true
    }
}
