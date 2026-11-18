//! Pads and keyboard sharing the original keys, driven through the same paths as real devices.
use super::controller::{BUTTON_KEYS, Pad, direction};
use super::*;
use gilrs::Button;

fn scans(input: &mut Input) -> Vec<u8> {
    input.queue.drain(..).collect()
}

/// The keys of every pad, as the input layer receives them after each pad event.
fn apply(input: &mut Input, pads: &[Pad]) {
    let mut keys = [false; 512];
    for pad in pads {
        pad.keys(&mut keys);
    }
    if input.focused {
        input.apply_controller(keys);
    } else {
        input.apply_controller([false; 512]);
    }
}

fn press(input: &mut Input, pads: &mut [Pad], pad: usize, button: Button, down: bool) {
    assert!(pads[pad].press(button, down));
    apply(input, pads);
}

#[test]
fn every_button_presses_its_key_once_and_releases_it() {
    let mut input = Input {
        focused: true,
        ..Input::default()
    };
    let mut pads = [Pad::default()];
    for (button, make, release) in [
        (Button::South, vec![0x39], vec![0xb9]),
        (Button::West, vec![0x1d], vec![0x9d]),
        (Button::East, vec![1], vec![0x81]),
        (Button::North, vec![0x1f], vec![0x9f]),
        (Button::LeftTrigger, vec![0x2a], vec![0xaa]),
        (Button::RightTrigger, vec![0x38], vec![0xb8]),
        (Button::Start, vec![1], vec![0x81]),
        (Button::Select, vec![0x17], vec![0x97]),
        (Button::LeftThumb, vec![0x1c], vec![0x9c]),
        (Button::DPadUp, vec![0xe0, 0x48], vec![0xe0, 0xc8]),
        (Button::DPadDown, vec![0xe0, 0x50], vec![0xe0, 0xd0]),
        (Button::DPadLeft, vec![0xe0, 0x4b], vec![0xe0, 0xcb]),
        (Button::DPadRight, vec![0xe0, 0x4d], vec![0xe0, 0xcd]),
    ] {
        press(&mut input, &mut pads, 0, button, true);
        assert_eq!(scans(&mut input), make, "{button:?}");
        apply(&mut input, &pads);
        assert!(scans(&mut input).is_empty(), "held button must not repeat");
        press(&mut input, &mut pads, 0, button, false);
        assert_eq!(scans(&mut input), release, "{button:?}");
    }
    assert_eq!(BUTTON_KEYS.len(), 13);
    // Two controls mapped to Escape release it only when both are up.
    press(&mut input, &mut pads, 0, Button::East, true);
    press(&mut input, &mut pads, 0, Button::Start, true);
    assert_eq!(scans(&mut input), [1]);
    press(&mut input, &mut pads, 0, Button::East, false);
    assert!(scans(&mut input).is_empty());
    press(&mut input, &mut pads, 0, Button::Start, false);
    assert_eq!(scans(&mut input), [0x81]);
    assert!(!pads[0].press(Button::RightTrigger2, true), "unmapped buttons are ignored");
}

#[test]
fn the_stick_ignores_drift_and_does_not_chatter() {
    assert_eq!(direction(0.25, 0), 0);
    assert_eq!(direction(0.52, 0), 1);
    assert_eq!(direction(0.45, 1), 1, "stays on above the release threshold");
    assert_eq!(direction(0.3, 1), 0);
    assert_eq!(direction(-1.0, 0), -1);
    let mut input = Input {
        focused: true,
        ..Input::default()
    };
    let mut pads = [Pad::default()];
    pads[0].y = -1;
    apply(&mut input, &pads);
    assert_eq!(scans(&mut input), [0xe0, 0x48]);
    pads[0].y = 1;
    apply(&mut input, &pads);
    assert_eq!(
        scans(&mut input),
        [0xe0, 0xc8, 0xe0, 0x50],
        "a reversal releases before it presses"
    );
    pads[0].y = 0;
    apply(&mut input, &pads);
    assert_eq!(scans(&mut input), [0xe0, 0xd0]);
}

#[test]
fn keyboard_and_pads_share_keys_across_focus_and_hotplug() {
    let mut input = Input {
        focused: true,
        ..Input::default()
    };
    let mut pads = vec![Pad::default()];
    // Either source may release first without releasing the other's hold.
    input.keyboard(0x39, true, false);
    assert_eq!(scans(&mut input), [0x39]);
    press(&mut input, &mut pads, 0, Button::South, true);
    assert!(scans(&mut input).is_empty());
    input.keyboard(0x39, false, false);
    assert!(scans(&mut input).is_empty());
    press(&mut input, &mut pads, 0, Button::South, false);
    assert_eq!(scans(&mut input), [0xb9]);
    press(&mut input, &mut pads, 0, Button::South, true);
    input.keyboard(0x39, true, false);
    press(&mut input, &mut pads, 0, Button::South, false);
    assert_eq!(scans(&mut input), [0x39]);
    input.keyboard(0x39, true, true);
    assert_eq!(scans(&mut input), [0x39], "keyboard repeats are preserved");
    input.keyboard(0x39, false, false);
    assert_eq!(scans(&mut input), [0xb9]);

    // Losing focus releases everything; returning restores what the pads still hold.
    press(&mut input, &mut pads, 0, Button::South, true);
    assert_eq!(scans(&mut input), [0x39]);
    input.focus(false);
    assert_eq!(scans(&mut input), [0xb9]);
    press(&mut input, &mut pads, 0, Button::West, true);
    assert!(scans(&mut input).is_empty());
    input.focused = true;
    apply(&mut input, &pads);
    assert_eq!(scans(&mut input), [0x1d, 0x39]);
    press(&mut input, &mut pads, 0, Button::West, false);
    press(&mut input, &mut pads, 0, Button::South, false);
    assert_eq!(scans(&mut input), [0x9d, 0xb9]);

    // A second pad holding the same key keeps it held when the first is unplugged.
    pads.push(Pad::default());
    press(&mut input, &mut pads, 0, Button::South, true);
    press(&mut input, &mut pads, 1, Button::South, true);
    assert_eq!(scans(&mut input), [0x39]);
    pads.remove(0);
    apply(&mut input, &pads);
    assert!(scans(&mut input).is_empty());
    pads.clear();
    apply(&mut input, &pads);
    assert_eq!(scans(&mut input), [0xb9]);
    assert!(!input.held.iter().any(|held| *held));
}
