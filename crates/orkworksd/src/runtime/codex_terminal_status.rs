//! Best-effort Codex UI status from live terminal output (ADR 0093).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Working,
    Blocked,
    Idle,
    Failed,
    Unknown,
}

#[derive(Default)]
enum Mode {
    #[default]
    Text,
    Escape,
    Osc,
    Skip,
}

#[derive(Default)]
pub(crate) struct Reader {
    mode: Mode,
    payload: Vec<u8>,
    escaped: bool,
}

impl Reader {
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<Status> {
        let mut statuses = Vec::new();
        for &byte in bytes {
            match self.mode {
                Mode::Text => {
                    if byte == 0x1b {
                        self.mode = Mode::Escape;
                    }
                }
                Mode::Escape => {
                    self.mode = match byte {
                        b']' => {
                            self.payload.clear();
                            self.escaped = false;
                            Mode::Osc
                        }
                        b'P' | b'X' | b'^' | b'_' => {
                            self.escaped = false;
                            Mode::Skip
                        }
                        0x1b => Mode::Escape,
                        _ => Mode::Text,
                    };
                }
                Mode::Osc | Mode::Skip => {
                    if byte == 7 || (self.escaped && byte == b'\\') {
                        if matches!(self.mode, Mode::Osc) {
                            if let Some(status) = classify(&self.payload) {
                                statuses.push(status);
                            }
                        }
                        self.mode = Mode::Text;
                        self.escaped = false;
                        continue;
                    }
                    if self.escaped {
                        self.mode = Mode::Skip;
                        self.escaped = false;
                    }
                    if byte == 0x1b {
                        self.escaped = true;
                    } else if matches!(self.mode, Mode::Osc) {
                        if self.payload.len() == 256 {
                            self.mode = Mode::Skip;
                            self.payload.clear();
                        } else {
                            self.payload.push(byte);
                        }
                    }
                }
            }
        }
        statuses
    }
}

fn classify(payload: &[u8]) -> Option<Status> {
    let payload = std::str::from_utf8(payload).ok()?.strip_prefix("7501;")?;
    if payload == "state=clear" {
        return Some(Status::Unknown);
    }
    let (mut state, mut app, mut kind) = (None, None, None);
    for field in payload.split(':') {
        let (key, value) = field.split_once('=')?;
        let slot = match key {
            "state" => &mut state,
            "app" => &mut app,
            "kind" => &mut kind,
            _ => return None,
        };
        if slot.replace(value).is_some() {
            return None;
        }
    }
    if app != Some("codex") {
        return None;
    }
    if state == Some("blocked") {
        return Some(
            if matches!(kind, Some("permission" | "question" | "auth")) {
                Status::Blocked
            } else {
                Status::Unknown
            },
        );
    }
    if kind.is_some() {
        return None;
    }
    Some(match state? {
        "working" => Status::Working,
        "idle" | "done" => Status::Idle,
        "error" => Status::Failed,
        _ => Status::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Literal fixtures from Codex 0.164.0 terminal_program_status.rs.
    const MANUAL: &[u8] = b"\x1b]7501;state=idle:app=codex\x1b\\\x1b]7501;state=working:app=codex\x1b\\\x1b]7501;state=blocked:app=codex:kind=permission\x1b\\\x1b]7501;state=working:app=codex\x1b\\\x1b]7501;state=done:app=codex\x1b\\";

    #[test]
    fn codex_terminal_status_manual_trace_survives_every_chunk_boundary() {
        let expected = [
            Status::Idle,
            Status::Working,
            Status::Blocked,
            Status::Working,
            Status::Idle,
        ];
        for split in 0..=MANUAL.len() {
            let mut reader = Reader::default();
            let mut statuses = reader.feed(&MANUAL[..split]);
            statuses.extend(reader.feed(&MANUAL[split..]));
            assert_eq!(statuses, expected, "split {split}");
        }
        let mut reader = Reader::default();
        let statuses: Vec<_> = MANUAL
            .iter()
            .flat_map(|byte| reader.feed(&[*byte]))
            .collect();
        assert_eq!(statuses, expected);
    }

    #[test]
    fn codex_terminal_status_clear_unknown_and_all_prompt_kinds() {
        let mut reader = Reader::default();
        assert_eq!(reader.feed(b"\x1b]7501;state=blocked:app=codex:kind=question\x07\x1b]7501;state=blocked:app=codex:kind=auth\x07\x1b]7501;state=error:app=codex\x07\x1b]7501;state=clear\x07\x1b]7501;state=future:app=codex\x07\x1b]7501;state=blocked:app=codex:kind=future\x07"), [Status::Blocked, Status::Blocked, Status::Failed, Status::Unknown, Status::Unknown, Status::Unknown]);
    }

    #[test]
    fn codex_terminal_status_ignores_foreign_malformed_embedded_and_oversized_frames() {
        let mut reader = Reader::default();
        for frame in [
            "PermissionRequest Reviewing approval requests state=blocked:app=codex",
            "\x1b]7501;state=working:app=other\x07",
            "\x1b]7501;state=working:app=codex:state=blocked\x07",
            "\x1b]7501;state=working:app=codex:extra=yes\x07",
            "\x1b]7501;state=working:app=codex:kind=permission\x07",
            "\x1b]0;title\x1b]7501;state=working:app=codex\x1b\\",
            "\x1bP\x1b]7501;state=working:app=codex\x1b\\",
        ] {
            assert!(reader.feed(frame.as_bytes()).is_empty(), "{frame:?}");
        }
        let oversized = format!("\x1b]7501;{}\x07", "x".repeat(10000));
        assert!(reader.feed(oversized.as_bytes()).is_empty());
        assert_eq!(reader.feed(MANUAL).len(), 5);
    }
}
