//! Exercises the actual SDL event pump with a virtual gamepad, without a window.
use super::*;
use sdl2::controller::{Axis, Button};
use sdl2::sys;

struct Pad {
    index: i32,
    joystick: *mut sys::SDL_Joystick,
}

impl Pad {
    fn new() -> Self {
        let index = unsafe {
            sys::SDL_JoystickAttachVirtual(
                sys::SDL_JoystickType::SDL_JOYSTICK_TYPE_GAMECONTROLLER,
                6,
                15,
                0,
            )
        };
        assert!(index >= 0, "{}", sdl2::get_error());
        let joystick = unsafe { sys::SDL_JoystickOpen(index) };
        assert!(!joystick.is_null(), "{}", sdl2::get_error());
        Self { index, joystick }
    }

    fn button(&self, button: Button, down: bool) {
        assert_eq!(
            unsafe {
                sys::SDL_JoystickSetVirtualButton(self.joystick, button as i32, u8::from(down))
            },
            0
        );
        unsafe { sys::SDL_JoystickUpdate() };
    }

    fn axis(&self, axis: Axis, value: i16) {
        assert_eq!(
            unsafe { sys::SDL_JoystickSetVirtualAxis(self.joystick, axis as i32, value) },
            0
        );
        unsafe { sys::SDL_JoystickUpdate() };
    }
}

impl Drop for Pad {
    fn drop(&mut self) {
        unsafe {
            sys::SDL_JoystickClose(self.joystick);
            assert_eq!(sys::SDL_JoystickDetachVirtual(self.index), 0);
        }
    }
}

fn scans(input: &mut Input) -> Vec<u8> {
    assert!(input.pump());
    input.queue.drain(..).collect()
}

fn key(input: &Input, down: bool, repeat: bool) {
    let event = if down {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: None,
            scancode: Some(Scancode::Space),
            keymod: sdl2::keyboard::Mod::NOMOD,
            repeat,
        }
    } else {
        Event::KeyUp {
            timestamp: 0,
            window_id: 0,
            keycode: None,
            scancode: Some(Scancode::Space),
            keymod: sdl2::keyboard::Mod::NOMOD,
            repeat,
        }
    };
    input
        .sdl
        .as_ref()
        .unwrap()
        .event()
        .unwrap()
        .push_event(event)
        .unwrap();
}

fn focus(input: &Input, gained: bool) {
    input
        .sdl
        .as_ref()
        .unwrap()
        .event()
        .unwrap()
        .push_event(Event::Window {
            timestamp: 0,
            window_id: 0,
            win_event: if gained {
                WindowEvent::FocusGained
            } else {
                WindowEvent::FocusLost
            },
        })
        .unwrap();
}

#[test]
fn virtual_controllers_deliver_ordered_scans_and_release_only_their_own_keys() {
    sdl2::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
    let sdl = sdl2::init().unwrap();
    let subsystem = sdl.game_controller().unwrap();
    // Connect before runtime controller discovery, as a player normally would.
    let pad = Pad::new();
    pad.button(Button::A, true);
    let mut input = Input {
        events: Some(sdl.event_pump().unwrap()),
        focused: true,
        sdl: Some(sdl),
        ..Input::default()
    };
    // The runtime initializes its own controller subsystem on its first pump.
    assert_eq!(scans(&mut input), [0x39]);
    pad.button(Button::A, false);
    assert_eq!(scans(&mut input), [0xb9]);
    for (button, make, release) in [
        (Button::A, vec![0x39], vec![0xb9]),
        (Button::X, vec![0x1d], vec![0x9d]),
        (Button::B, vec![1], vec![0x81]),
        (Button::Y, vec![0x1f], vec![0x9f]),
        (Button::LeftShoulder, vec![0x2a], vec![0xaa]),
        (Button::RightShoulder, vec![0x38], vec![0xb8]),
        (Button::Start, vec![1], vec![0x81]),
        (Button::Back, vec![0x17], vec![0x97]),
        (Button::LeftStick, vec![0x1c], vec![0x9c]),
        (Button::DPadUp, vec![0xe0, 0x48], vec![0xe0, 0xc8]),
        (Button::DPadDown, vec![0xe0, 0x50], vec![0xe0, 0xd0]),
        (Button::DPadLeft, vec![0xe0, 0x4b], vec![0xe0, 0xcb]),
        (Button::DPadRight, vec![0xe0, 0x4d], vec![0xe0, 0xcd]),
    ] {
        pad.button(button, true);
        assert_eq!(scans(&mut input), make, "{button:?}");
        assert!(scans(&mut input).is_empty(), "held button must not repeat");
        pad.button(button, false);
        assert_eq!(scans(&mut input), release, "{button:?}");
    }
    // Two controls on the same pad may map to Escape without an early release.
    pad.button(Button::B, true);
    pad.button(Button::Start, true);
    assert_eq!(scans(&mut input), [1]);
    pad.button(Button::B, false);
    assert!(scans(&mut input).is_empty());
    pad.button(Button::Start, false);
    assert_eq!(scans(&mut input), [0x81]);
    pad.axis(Axis::RightX, i16::MAX);
    pad.axis(Axis::TriggerLeft, i16::MAX);
    assert!(scans(&mut input).is_empty());

    // A tap entirely between engine polls must still reach the game.
    pad.button(Button::A, true);
    pad.button(Button::A, false);
    assert_eq!(scans(&mut input), [0x39, 0xb9]);

    // Stick drift is ignored. Hysteresis avoids chatter at the activation edge.
    pad.axis(Axis::LeftX, 8000);
    assert!(scans(&mut input).is_empty());
    pad.axis(Axis::LeftX, 17000);
    assert_eq!(scans(&mut input), [0xe0, 0x4d]);
    pad.axis(Axis::LeftX, 15000);
    assert!(scans(&mut input).is_empty());
    pad.button(Button::DPadRight, true);
    pad.axis(Axis::LeftX, 0);
    assert!(scans(&mut input).is_empty());
    pad.button(Button::DPadRight, false);
    assert_eq!(scans(&mut input), [0xe0, 0xcd]);
    pad.axis(Axis::LeftY, i16::MIN);
    assert_eq!(scans(&mut input), [0xe0, 0x48]);
    pad.axis(Axis::LeftY, i16::MAX);
    assert_eq!(scans(&mut input), [0xe0, 0xc8, 0xe0, 0x50]);
    pad.axis(Axis::LeftY, 0);
    assert_eq!(scans(&mut input), [0xe0, 0xd0]);

    // Either source may release first, without releasing the other's hold.
    key(&input, true, false);
    assert_eq!(scans(&mut input), [0x39]);
    pad.button(Button::A, true);
    assert!(scans(&mut input).is_empty());
    key(&input, false, false);
    assert!(scans(&mut input).is_empty());
    pad.button(Button::A, false);
    assert_eq!(scans(&mut input), [0xb9]);
    pad.button(Button::A, true);
    assert_eq!(scans(&mut input), [0x39]);
    key(&input, true, false);
    assert!(scans(&mut input).is_empty());
    pad.button(Button::A, false);
    assert!(scans(&mut input).is_empty());
    key(&input, true, true);
    assert_eq!(scans(&mut input), [0x39], "keyboard repeats are preserved");
    key(&input, false, false);
    assert_eq!(scans(&mut input), [0xb9]);

    pad.button(Button::A, true);
    assert_eq!(scans(&mut input), [0x39]);
    focus(&input, false);
    assert_eq!(scans(&mut input), [0xb9]);
    pad.button(Button::X, true);
    assert!(scans(&mut input).is_empty());
    focus(&input, true);
    assert_eq!(scans(&mut input), [0x1d, 0x39]);
    pad.button(Button::X, false);
    pad.button(Button::A, false);
    assert_eq!(scans(&mut input), [0x9d, 0xb9]);

    // Hotplug a second controller. Shared holds survive another pad's removal.
    let second = Pad::new();
    assert!(scans(&mut input).is_empty());
    pad.button(Button::A, true);
    second.button(Button::A, true);
    assert_eq!(scans(&mut input), [0x39]);
    drop(second);
    assert!(scans(&mut input).is_empty());
    let disconnected_id = unsafe { sys::SDL_JoystickInstanceID(pad.joystick) } as u32;
    drop(pad);
    assert_eq!(scans(&mut input), [0xb9]);
    let reconnected = Pad::new();
    assert!(scans(&mut input).is_empty());
    input
        .sdl
        .as_ref()
        .unwrap()
        .event()
        .unwrap()
        .push_event(Event::ControllerButtonDown {
            timestamp: 0,
            which: disconnected_id,
            button: Button::X,
        })
        .unwrap();
    assert!(
        scans(&mut input).is_empty(),
        "ignore stale disconnected instance IDs"
    );
    reconnected.button(Button::A, true);
    assert_eq!(scans(&mut input), [0x39]);
    // A pad removed while unfocused must not restore phantom holds on return.
    focus(&input, false);
    assert_eq!(scans(&mut input), [0xb9]);
    drop(reconnected);
    assert!(scans(&mut input).is_empty());
    focus(&input, true);
    assert!(scans(&mut input).is_empty());
    assert!(!input.held.iter().any(|held| *held));
    drop(input);
    drop(subsystem);
}
