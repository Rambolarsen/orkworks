//! Interaction-mode checkpoint at the start of the bounded PTY replay window.
//!
//! This is not a screen emulator: retained output still owns screen contents.
//! Only evicted bytes advance this checkpoint, so replayed mode changes remain
//! in order. Parsing control strings prevents title/payload text becoming modes.

#[derive(Debug, Default)]
enum ParserState {
    #[default]
    Ground,
    Escape(Vec<u8>),
    Csi(Vec<u8>),
    IgnoreCsi,
    String(u8),
}

const MODES: [u16; 5] = [1, 7, 25, 1004, 2004];
const DEFAULT_MODES: [bool; 5] = [false, true, true, false, false];
const MAX_SEQUENCE_BYTES: usize = 128;

#[derive(Debug)]
pub(super) struct TerminalReplayModes {
    parser: ParserState,
    utf8: Vec<u8>,
    modes: [bool; 5],
    alternate: bool,
    mouse: Option<u16>,
    encoding: Option<u16>,
    application_keypad: bool,
}

impl Default for TerminalReplayModes {
    fn default() -> Self {
        Self {
            parser: ParserState::Ground,
            utf8: Vec::new(),
            modes: DEFAULT_MODES,
            alternate: false,
            mouse: None,
            encoding: None,
            application_keypad: false,
        }
    }
}

impl TerminalReplayModes {
    pub(super) fn advance(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            // PTY bytes reach xterm through its UTF-8 decoder. A bare 0x9b
            // is invalid UTF-8; only decoded U+009B is the C1 CSI control.
            if self.utf8.is_empty() {
                if byte < 0x80 {
                    self.control(byte);
                } else if (0xc2..=0xf4).contains(&byte) {
                    self.utf8.push(byte);
                } else {
                    self.control(0xff);
                }
                continue;
            }
            self.utf8.push(byte);
            match std::str::from_utf8(&self.utf8) {
                Ok(text) => {
                    let control = text
                        .chars()
                        .next()
                        .filter(|ch| ('\u{80}'..='\u{9f}').contains(ch));
                    self.utf8.clear();
                    self.control(control.map_or(0xff, |ch| ch as u8));
                }
                Err(error) if error.error_len().is_none() => {}
                Err(_) => {
                    self.utf8.clear();
                    self.control(0xff);
                    // An invalid prefix must not swallow a following ASCII
                    // escape or the beginning of a new UTF-8 character.
                    if byte < 0x80 {
                        self.control(byte);
                    } else if (0xc2..=0xf4).contains(&byte) {
                        self.utf8.push(byte);
                    }
                }
            }
        }
    }

    fn control(&mut self, byte: u8) {
        match byte {
            0x9b => {
                self.parser = ParserState::Csi(Vec::new());
                return;
            }
            0x90 | 0x98 | 0x9d | 0x9e | 0x9f => {
                let kind = match byte {
                    0x90 => b'P',
                    0x98 => b'X',
                    0x9d => b']',
                    0x9e => b'^',
                    _ => b'_',
                };
                self.parser = ParserState::String(kind);
                return;
            }
            0x9c => {
                self.parser = ParserState::Ground;
                return;
            }
            0x7f | 0x80..=0x9f => return,
            _ => {}
        }
        if matches!(byte, 0x18 | 0x1a) {
            self.parser = ParserState::Ground;
            return;
        }
        if byte == 0x1b {
            self.parser = ParserState::Escape(Vec::new());
            return;
        }
        let state = std::mem::take(&mut self.parser);
        self.parser = match state {
            ParserState::Ground => ParserState::Ground,
            ParserState::Escape(mut intermediates) => {
                if intermediates.is_empty() {
                    match byte {
                        b'[' => {
                            self.parser = ParserState::Csi(Vec::new());
                            return;
                        }
                        b']' | b'P' | b'X' | b'^' | b'_' => {
                            self.parser = ParserState::String(byte);
                            return;
                        }
                        b'c' => *self = Self::default(),
                        b'=' => self.application_keypad = true,
                        b'>' => self.application_keypad = false,
                        _ => {}
                    }
                }
                if (0x20..=0x2f).contains(&byte) && intermediates.len() < MAX_SEQUENCE_BYTES {
                    intermediates.push(byte);
                    ParserState::Escape(intermediates)
                } else if byte < 0x20 {
                    ParserState::Escape(intermediates)
                } else {
                    ParserState::Ground
                }
            }
            ParserState::Csi(mut params) => {
                if (0x40..=0x7e).contains(&byte) {
                    self.csi(&params, byte);
                    ParserState::Ground
                } else if byte < 0x20 {
                    ParserState::Csi(params)
                } else if params.len() < MAX_SEQUENCE_BYTES {
                    params.push(byte);
                    ParserState::Csi(params)
                } else {
                    ParserState::IgnoreCsi
                }
            }
            ParserState::IgnoreCsi => {
                if (0x40..=0x7e).contains(&byte) {
                    ParserState::Ground
                } else {
                    ParserState::IgnoreCsi
                }
            }
            ParserState::String(kind) => {
                if kind == b']' && byte == 0x07 {
                    ParserState::Ground
                } else {
                    ParserState::String(kind)
                }
            }
        };
    }

    fn csi(&mut self, params: &[u8], final_byte: u8) {
        if params == b"!" && final_byte == b'p' {
            // xterm's DECSTR resets core keyboard/paste/focus modes, but not
            // the mouse service or active buffer.
            self.modes = DEFAULT_MODES;
            self.application_keypad = false;
            return;
        }
        if !matches!(final_byte, b'h' | b'l') || params.first() != Some(&b'?') {
            return;
        }
        // Recognize plain numeric private modes; storage and arithmetic are bounded.
        if !params[1..].iter().all(|b| b.is_ascii_digit() || *b == b';') {
            return;
        }
        let enabled = final_byte == b'h';
        for param in params[1..].split(|b| *b == b';') {
            let mode = param.iter().try_fold(0u16, |n, digit| {
                n.checked_mul(10)?.checked_add(u16::from(*digit - b'0'))
            });
            let Some(mode) = mode else { continue };
            match mode {
                47 | 1047 | 1049 => self.alternate = enabled,
                9 | 1000 | 1002 | 1003 => {
                    self.mouse = enabled.then_some(mode);
                }
                1006 | 1016 => self.encoding = enabled.then_some(mode),
                66 => self.application_keypad = enabled,
                _ => {
                    if let Some(index) = MODES.iter().position(|candidate| *candidate == mode) {
                        self.modes[index] = enabled;
                    }
                }
            }
        }
    }

    pub(super) fn prefix(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        if self.alternate {
            bytes.extend_from_slice(b"\x1b[?1049h");
        }
        if self.application_keypad {
            bytes.extend_from_slice(b"\x1b=");
        }
        for mode in [self.mouse, self.encoding].into_iter().flatten() {
            bytes.extend_from_slice(format!("\x1b[?{mode}h").as_bytes());
        }
        for (index, mode) in MODES.iter().enumerate() {
            if self.modes[index] != DEFAULT_MODES[index] {
                let action = if self.modes[index] { 'h' } else { 'l' };
                bytes.extend_from_slice(format!("\x1b[?{mode}{action}").as_bytes());
            }
        }
        // Re-enter a sequence that began before the retained window. Control
        // string contents are deliberately discarded, but their parser state
        // must survive or their remaining payload would become screen output.
        match &self.parser {
            ParserState::Ground => {}
            ParserState::Escape(intermediates) => {
                bytes.push(0x1b);
                bytes.extend_from_slice(intermediates);
            }
            ParserState::Csi(params) => {
                bytes.extend_from_slice(b"\x1b[");
                bytes.extend_from_slice(params);
            }
            ParserState::IgnoreCsi => bytes.extend_from_slice(b"\x1b[0?"),
            ParserState::String(kind) => bytes.extend_from_slice(&[0x1b, *kind]),
        }
        bytes.extend_from_slice(&self.utf8);
        bytes
    }
}
