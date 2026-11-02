//! Translation of platform key codes into the DOM `KeyboardEvent.code` spelling.
//!
//! [`Keystroke::code`](crate::Keystroke::code) names the physical key that was
//! pressed (`"KeyS"`, `"Digit1"`, `"ArrowLeft"`), so it stays stable when the
//! user switches layout or holds a modifier. Every platform numbers physical
//! keys differently, and each table lives in exactly one place here — away from
//! the backends that report the numbers — so that any host can test all of them.

/// The physical key for an AppKit key code, from `<HIToolbox/Events.h>` (`kVK_*`).
pub fn from_macos_key_code(key_code: u16) -> Option<&'static str> {
    Some(match key_code {
        0x00 => "KeyA",
        0x01 => "KeyS",
        0x02 => "KeyD",
        0x03 => "KeyF",
        0x04 => "KeyH",
        0x05 => "KeyG",
        0x06 => "KeyZ",
        0x07 => "KeyX",
        0x08 => "KeyC",
        0x09 => "KeyV",
        0x0A => "IntlBackslash",
        0x0B => "KeyB",
        0x0C => "KeyQ",
        0x0D => "KeyW",
        0x0E => "KeyE",
        0x0F => "KeyR",
        0x10 => "KeyY",
        0x11 => "KeyT",
        0x12 => "Digit1",
        0x13 => "Digit2",
        0x14 => "Digit3",
        0x15 => "Digit4",
        0x16 => "Digit6",
        0x17 => "Digit5",
        0x18 => "Equal",
        0x19 => "Digit9",
        0x1A => "Digit7",
        0x1B => "Minus",
        0x1C => "Digit8",
        0x1D => "Digit0",
        0x1E => "BracketRight",
        0x1F => "KeyO",
        0x20 => "KeyU",
        0x21 => "BracketLeft",
        0x22 => "KeyI",
        0x23 => "KeyP",
        0x24 => "Enter",
        0x25 => "KeyL",
        0x26 => "KeyJ",
        0x27 => "Quote",
        0x28 => "KeyK",
        0x29 => "Semicolon",
        0x2A => "Backslash",
        0x2B => "Comma",
        0x2C => "Slash",
        0x2D => "KeyN",
        0x2E => "KeyM",
        0x2F => "Period",
        0x30 => "Tab",
        0x31 => "Space",
        0x32 => "Backquote",
        0x33 => "Backspace",
        0x35 => "Escape",
        0x36 => "MetaRight",
        0x37 => "MetaLeft",
        0x38 => "ShiftLeft",
        0x39 => "CapsLock",
        0x3A => "AltLeft",
        0x3B => "ControlLeft",
        0x3C => "ShiftRight",
        0x3D => "AltRight",
        0x3E => "ControlRight",
        0x3F => "Fn",
        0x40 => "F17",
        0x41 => "NumpadDecimal",
        0x43 => "NumpadMultiply",
        0x45 => "NumpadAdd",
        // The Clear key occupies the NumLock position on Apple keyboards.
        0x47 => "NumLock",
        0x48 => "AudioVolumeUp",
        0x49 => "AudioVolumeDown",
        0x4A => "AudioVolumeMute",
        0x4B => "NumpadDivide",
        0x4C => "NumpadEnter",
        0x4E => "NumpadSubtract",
        0x4F => "F18",
        0x50 => "F19",
        0x51 => "NumpadEqual",
        0x52 => "Numpad0",
        0x53 => "Numpad1",
        0x54 => "Numpad2",
        0x55 => "Numpad3",
        0x56 => "Numpad4",
        0x57 => "Numpad5",
        0x58 => "Numpad6",
        0x59 => "Numpad7",
        0x5A => "F20",
        0x5B => "Numpad8",
        0x5C => "Numpad9",
        0x5D => "IntlYen",
        0x5E => "IntlRo",
        0x5F => "NumpadComma",
        0x66 => "Lang2",
        0x68 => "Lang1",
        0x60 => "F5",
        0x61 => "F6",
        0x62 => "F7",
        0x63 => "F3",
        0x64 => "F8",
        0x65 => "F9",
        0x67 => "F11",
        0x69 => "F13",
        0x6A => "F16",
        0x6B => "F14",
        0x6D => "F10",
        0x6E => "ContextMenu",
        0x6F => "F12",
        0x71 => "F15",
        // kVK_Help occupies the Insert position; browsers report it as Insert.
        0x72 => "Insert",
        0x73 => "Home",
        0x74 => "PageUp",
        0x75 => "Delete",
        0x76 => "F4",
        0x77 => "End",
        0x78 => "F2",
        0x79 => "PageDown",
        0x7A => "F1",
        0x7B => "ArrowLeft",
        0x7C => "ArrowRight",
        0x7D => "ArrowDown",
        0x7E => "ArrowUp",
        _ => return None,
    })
}

/// The physical key for a Set 1 scan code from `WM_KEYDOWN`.
///
/// `extended` marks the `E0`-prefixed codes that share a byte with an unrelated
/// key (the right-hand modifiers, the navigation cluster, media keys).
pub fn from_windows_scancode(scan_code: u8, extended: bool) -> Option<&'static str> {
    if extended {
        return Some(match scan_code {
            0x10 => "MediaTrackPrevious",
            0x19 => "MediaTrackNext",
            0x1C => "NumpadEnter",
            0x1D => "ControlRight",
            0x20 => "AudioVolumeMute",
            0x22 => "MediaPlayPause",
            0x24 => "MediaStop",
            0x2E => "AudioVolumeDown",
            0x30 => "AudioVolumeUp",
            0x32 => "BrowserHome",
            0x35 => "NumpadDivide",
            0x37 => "PrintScreen",
            0x38 => "AltRight",
            0x45 => "NumLock",
            0x47 => "Home",
            0x48 => "ArrowUp",
            0x49 => "PageUp",
            0x4B => "ArrowLeft",
            0x4D => "ArrowRight",
            0x4F => "End",
            0x50 => "ArrowDown",
            0x51 => "PageDown",
            0x52 => "Insert",
            0x53 => "Delete",
            0x5B => "MetaLeft",
            0x5C => "MetaRight",
            0x5D => "ContextMenu",
            0x5E => "Power",
            0x65 => "BrowserSearch",
            0x67 => "BrowserRefresh",
            0x68 => "BrowserStop",
            0x69 => "BrowserForward",
            0x6A => "BrowserBack",
            _ => return None,
        });
    }

    Some(match scan_code {
        0x01 => "Escape",
        0x02 => "Digit1",
        0x03 => "Digit2",
        0x04 => "Digit3",
        0x05 => "Digit4",
        0x06 => "Digit5",
        0x07 => "Digit6",
        0x08 => "Digit7",
        0x09 => "Digit8",
        0x0A => "Digit9",
        0x0B => "Digit0",
        0x0C => "Minus",
        0x0D => "Equal",
        0x0E => "Backspace",
        0x0F => "Tab",
        0x10 => "KeyQ",
        0x11 => "KeyW",
        0x12 => "KeyE",
        0x13 => "KeyR",
        0x14 => "KeyT",
        0x15 => "KeyY",
        0x16 => "KeyU",
        0x17 => "KeyI",
        0x18 => "KeyO",
        0x19 => "KeyP",
        0x1A => "BracketLeft",
        0x1B => "BracketRight",
        0x1C => "Enter",
        0x1D => "ControlLeft",
        0x1E => "KeyA",
        0x1F => "KeyS",
        0x20 => "KeyD",
        0x21 => "KeyF",
        0x22 => "KeyG",
        0x23 => "KeyH",
        0x24 => "KeyJ",
        0x25 => "KeyK",
        0x26 => "KeyL",
        0x27 => "Semicolon",
        0x28 => "Quote",
        0x29 => "Backquote",
        0x2A => "ShiftLeft",
        0x2B => "Backslash",
        0x2C => "KeyZ",
        0x2D => "KeyX",
        0x2E => "KeyC",
        0x2F => "KeyV",
        0x30 => "KeyB",
        0x31 => "KeyN",
        0x32 => "KeyM",
        0x33 => "Comma",
        0x34 => "Period",
        0x35 => "Slash",
        0x36 => "ShiftRight",
        0x37 => "NumpadMultiply",
        0x38 => "AltLeft",
        0x39 => "Space",
        0x3A => "CapsLock",
        0x3B => "F1",
        0x3C => "F2",
        0x3D => "F3",
        0x3E => "F4",
        0x3F => "F5",
        0x40 => "F6",
        0x41 => "F7",
        0x42 => "F8",
        0x43 => "F9",
        0x44 => "F10",
        // Pause arrives as E1 1D 45; Windows reports it as plain 0x45, and
        // NumLock as the extended 0x45.
        0x45 => "Pause",
        0x46 => "ScrollLock",
        0x47 => "Numpad7",
        0x48 => "Numpad8",
        0x49 => "Numpad9",
        0x4A => "NumpadSubtract",
        0x4B => "Numpad4",
        0x4C => "Numpad5",
        0x4D => "Numpad6",
        0x4E => "NumpadAdd",
        0x4F => "Numpad1",
        0x50 => "Numpad2",
        0x51 => "Numpad3",
        0x52 => "Numpad0",
        0x53 => "NumpadDecimal",
        0x56 => "IntlBackslash",
        0x57 => "F11",
        0x58 => "F12",
        0x59 => "NumpadEqual",
        0x64 => "F13",
        0x65 => "F14",
        0x66 => "F15",
        0x67 => "F16",
        0x68 => "F17",
        0x69 => "F18",
        0x6A => "F19",
        0x6B => "F20",
        0x6C => "F21",
        0x6D => "F22",
        0x6E => "F23",
        // ABNT, JIS and Korean keyboards.
        0x71 => "Lang2",
        0x72 => "Lang1",
        0x73 => "IntlRo",
        0x76 => "F24",
        0x7D => "IntlYen",
        0x7E => "NumpadComma",
        _ => return None,
    })
}

/// The physical key for an xkb key code, which on Linux is the evdev code plus 8.
pub fn from_xkb_keycode(keycode: u32) -> Option<&'static str> {
    let evdev = keycode.checked_sub(8)?;
    Some(match evdev {
        1 => "Escape",
        2 => "Digit1",
        3 => "Digit2",
        4 => "Digit3",
        5 => "Digit4",
        6 => "Digit5",
        7 => "Digit6",
        8 => "Digit7",
        9 => "Digit8",
        10 => "Digit9",
        11 => "Digit0",
        12 => "Minus",
        13 => "Equal",
        14 => "Backspace",
        15 => "Tab",
        16 => "KeyQ",
        17 => "KeyW",
        18 => "KeyE",
        19 => "KeyR",
        20 => "KeyT",
        21 => "KeyY",
        22 => "KeyU",
        23 => "KeyI",
        24 => "KeyO",
        25 => "KeyP",
        26 => "BracketLeft",
        27 => "BracketRight",
        28 => "Enter",
        29 => "ControlLeft",
        30 => "KeyA",
        31 => "KeyS",
        32 => "KeyD",
        33 => "KeyF",
        34 => "KeyG",
        35 => "KeyH",
        36 => "KeyJ",
        37 => "KeyK",
        38 => "KeyL",
        39 => "Semicolon",
        40 => "Quote",
        41 => "Backquote",
        42 => "ShiftLeft",
        43 => "Backslash",
        44 => "KeyZ",
        45 => "KeyX",
        46 => "KeyC",
        47 => "KeyV",
        48 => "KeyB",
        49 => "KeyN",
        50 => "KeyM",
        51 => "Comma",
        52 => "Period",
        53 => "Slash",
        54 => "ShiftRight",
        55 => "NumpadMultiply",
        56 => "AltLeft",
        57 => "Space",
        58 => "CapsLock",
        59 => "F1",
        60 => "F2",
        61 => "F3",
        62 => "F4",
        63 => "F5",
        64 => "F6",
        65 => "F7",
        66 => "F8",
        67 => "F9",
        68 => "F10",
        69 => "NumLock",
        70 => "ScrollLock",
        71 => "Numpad7",
        72 => "Numpad8",
        73 => "Numpad9",
        74 => "NumpadSubtract",
        75 => "Numpad4",
        76 => "Numpad5",
        77 => "Numpad6",
        78 => "NumpadAdd",
        79 => "Numpad1",
        80 => "Numpad2",
        81 => "Numpad3",
        82 => "Numpad0",
        83 => "NumpadDecimal",
        86 => "IntlBackslash",
        87 => "F11",
        88 => "F12",
        89 => "IntlRo",
        96 => "NumpadEnter",
        97 => "ControlRight",
        98 => "NumpadDivide",
        99 => "PrintScreen",
        100 => "AltRight",
        102 => "Home",
        103 => "ArrowUp",
        104 => "PageUp",
        105 => "ArrowLeft",
        106 => "ArrowRight",
        107 => "End",
        108 => "ArrowDown",
        109 => "PageDown",
        110 => "Insert",
        111 => "Delete",
        113 => "AudioVolumeMute",
        114 => "AudioVolumeDown",
        115 => "AudioVolumeUp",
        116 => "Power",
        117 => "NumpadEqual",
        119 => "Pause",
        121 => "NumpadComma",
        122 => "Lang1",
        123 => "Lang2",
        124 => "IntlYen",
        125 => "MetaLeft",
        126 => "MetaRight",
        127 => "ContextMenu",
        128 => "BrowserStop",
        158 => "BrowserBack",
        159 => "BrowserForward",
        163 => "MediaTrackNext",
        164 => "MediaPlayPause",
        165 => "MediaTrackPrevious",
        166 => "MediaStop",
        172 => "BrowserHome",
        173 => "BrowserRefresh",
        183 => "F13",
        184 => "F14",
        185 => "F15",
        186 => "F16",
        187 => "F17",
        188 => "F18",
        189 => "F19",
        190 => "F20",
        191 => "F21",
        192 => "F22",
        193 => "F23",
        194 => "F24",
        217 => "BrowserSearch",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_key_codes_name_physical_keys() {
        assert_eq!(from_macos_key_code(0x01), Some("KeyS"));
        assert_eq!(from_macos_key_code(0x12), Some("Digit1"));
        assert_eq!(from_macos_key_code(0x24), Some("Enter"));
        assert_eq!(from_macos_key_code(0x32), Some("Backquote"));
        assert_eq!(from_macos_key_code(0x37), Some("MetaLeft"));
        assert_eq!(from_macos_key_code(0x3B), Some("ControlLeft"));
        assert_eq!(from_macos_key_code(0x7B), Some("ArrowLeft"));
        assert_eq!(from_macos_key_code(0x7A), Some("F1"));
        assert_eq!(from_macos_key_code(0x5A), Some("F20"));
        // kVK_Delete is the backspace key; kVK_ForwardDelete is Delete.
        assert_eq!(from_macos_key_code(0x33), Some("Backspace"));
        assert_eq!(from_macos_key_code(0x75), Some("Delete"));
        assert_eq!(from_macos_key_code(0x72), Some("Insert"));
        assert_eq!(from_macos_key_code(0x7F), None);
    }

    #[test]
    fn windows_scancodes_name_physical_keys() {
        assert_eq!(from_windows_scancode(0x1F, false), Some("KeyS"));
        assert_eq!(from_windows_scancode(0x02, false), Some("Digit1"));
        assert_eq!(from_windows_scancode(0x1C, false), Some("Enter"));
        assert_eq!(from_windows_scancode(0x29, false), Some("Backquote"));
        assert_eq!(from_windows_scancode(0x2A, false), Some("ShiftLeft"));
        assert_eq!(from_windows_scancode(0x3B, false), Some("F1"));
        assert_eq!(from_windows_scancode(0x58, false), Some("F12"));
        // The extended flag disambiguates keys that share a scan code.
        assert_eq!(from_windows_scancode(0x1C, true), Some("NumpadEnter"));
        assert_eq!(from_windows_scancode(0x38, true), Some("AltRight"));
        assert_eq!(from_windows_scancode(0x4B, true), Some("ArrowLeft"));
        assert_eq!(from_windows_scancode(0x4B, false), Some("Numpad4"));
        assert_eq!(from_windows_scancode(0x5B, true), Some("MetaLeft"));
        assert_eq!(from_windows_scancode(0x1D, true), Some("ControlRight"));
        assert_eq!(from_windows_scancode(0x45, false), Some("Pause"));
        assert_eq!(from_windows_scancode(0x45, true), Some("NumLock"));
        assert_eq!(from_windows_scancode(0x37, true), Some("PrintScreen"));
        assert_eq!(from_windows_scancode(0x00, false), None);
        assert_eq!(
            from_windows_scancode(0x10, true),
            Some("MediaTrackPrevious")
        );
        assert_eq!(from_windows_scancode(0x99, true), None);
    }

    #[test]
    fn xkb_keycodes_name_physical_keys() {
        // xkb key codes are evdev codes plus 8: KEY_S is 31, KEY_1 is 2.
        assert_eq!(from_xkb_keycode(31 + 8), Some("KeyS"));
        assert_eq!(from_xkb_keycode(2 + 8), Some("Digit1"));
        assert_eq!(from_xkb_keycode(28 + 8), Some("Enter"));
        assert_eq!(from_xkb_keycode(41 + 8), Some("Backquote"));
        assert_eq!(from_xkb_keycode(42 + 8), Some("ShiftLeft"));
        assert_eq!(from_xkb_keycode(105 + 8), Some("ArrowLeft"));
        assert_eq!(from_xkb_keycode(59 + 8), Some("F1"));
        assert_eq!(from_xkb_keycode(88 + 8), Some("F12"));
        assert_eq!(from_xkb_keycode(83 + 8), Some("NumpadDecimal"));
        assert_eq!(from_xkb_keycode(128 + 8), Some("BrowserStop"));
        assert_eq!(from_xkb_keycode(166 + 8), Some("MediaStop"));
        // Codes below the evdev offset are not physical keys.
        assert_eq!(from_xkb_keycode(7), None);
        assert_eq!(from_xkb_keycode(0), None);
        assert_eq!(from_xkb_keycode(255 + 8), None);
    }

    /// A typo in any table shows up as a duplicate or a missing letter here.
    #[test]
    fn tables_expose_the_same_alphanumeric_keys() {
        let macos: Vec<_> = (0..=0x7E).filter_map(from_macos_key_code).collect();
        let windows: Vec<_> = (0..=0xFF)
            .flat_map(|scan_code| {
                [
                    from_windows_scancode(scan_code, false),
                    from_windows_scancode(scan_code, true),
                ]
            })
            .flatten()
            .collect();
        let linux: Vec<_> = (0..=0xFF)
            .filter_map(|keycode| from_xkb_keycode(keycode + 8))
            .collect();

        let alphanumeric = |codes: &[&'static str]| {
            let mut codes: Vec<_> = codes
                .iter()
                .copied()
                .filter(|code| code.starts_with("Key") || code.starts_with("Digit"))
                .collect();
            codes.sort_unstable();
            codes.dedup();
            codes
        };

        let macos_alphanumeric = alphanumeric(&macos);
        assert_eq!(macos_alphanumeric.len(), 36, "26 letters and 10 digits");
        assert_eq!(alphanumeric(&windows), macos_alphanumeric);
        assert_eq!(alphanumeric(&linux), macos_alphanumeric);

        // Every code any table produces is a bare DOM code word.
        for code in macos.iter().chain(&windows).chain(&linux) {
            assert!(
                code.chars().all(|c| c.is_ascii_alphanumeric()),
                "not a bare DOM code word: {code:?}"
            );
        }
    }
}
