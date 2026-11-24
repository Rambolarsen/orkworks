//! Best-effort native UI status from live terminal output (ADRs 0093 and 0120).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Working,
    Blocked,
    Idle,
    Failed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Profile {
    #[default]
    Codex,
    Antigravity,
}

impl Profile {
    pub(crate) fn harness(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Antigravity => "antigravity",
        }
    }
    pub(crate) fn origin(self) -> &'static str {
        match self {
            Self::Codex => "codex_terminal",
            Self::Antigravity => "antigravity_terminal",
        }
    }
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
    profile: Profile,
    mode: Mode,
    payload: Vec<u8>,
    escaped: bool,
}

impl Reader {
    pub(crate) fn new(profile: Profile) -> Self {
        Self {
            profile,
            ..Self::default()
        }
    }

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
                    if self.profile == Profile::Antigravity && matches!(byte, 0x18 | 0x1a) {
                        self.mode = Mode::Text;
                        self.escaped = false;
                        self.payload.clear();
                        continue;
                    }
                    if byte == 7 || (self.escaped && byte == b'\\') {
                        if matches!(self.mode, Mode::Osc)
                            && !(self.profile == Profile::Antigravity && byte == 7 && self.escaped)
                        {
                            if let Some(status) = classify(&self.payload, self.profile) {
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
                        if self.profile == Profile::Antigravity && (byte < 0x20 || byte == 0x7f) {
                            self.mode = Mode::Skip;
                            self.payload.clear();
                        } else if self.payload.len() == 256 {
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

fn classify(payload: &[u8], profile: Profile) -> Option<Status> {
    let payload = payload.strip_prefix(b"7501;")?;
    if profile == Profile::Codex && payload == b"state=clear" {
        return Some(Status::Unknown);
    }
    let (mut state, mut app, mut kind) = (None, None, None);
    let mut message_seen = false;
    for field in payload.split(|byte| *byte == b':') {
        let separator = field.iter().position(|byte| *byte == b'=')?;
        let key = std::str::from_utf8(&field[..separator]).ok()?;
        if key == "msg" && profile == Profile::Antigravity {
            if message_seen {
                return None;
            }
            message_seen = true;
            continue;
        }
        let value = std::str::from_utf8(&field[separator + 1..]).ok()?;
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
    if app != Some(profile.harness()) {
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
    // Literal frames reproduce the retained native shape; message bodies are
    // deliberately synthetic and must never become attention metadata.
    #[test]
    fn antigravity_status_reports_question_resolution_and_discards_message() {
        let trace = b"\x1b]7501;state=working:app=antigravity\x07\x1b]7501;state=blocked:app=antigravity:kind=question:msg=PRIVATE\x07\x1b]7501;state=working:app=antigravity\x07\x1b]7501;state=done:app=antigravity:msg=PRIVATE\x07";
        let expected = [
            Status::Working,
            Status::Blocked,
            Status::Working,
            Status::Idle,
        ];
        for boundary in 0..=trace.len() {
            let mut reader = Reader::new(Profile::Antigravity);
            let mut actual = reader.feed(&trace[..boundary]);
            actual.extend(reader.feed(&trace[boundary..]));
            assert_eq!(actual, expected, "boundary {boundary}");
        }
        assert!(Reader::default().feed(trace).is_empty());
    }

    #[test]
    fn antigravity_status_rejects_child_foreign_duplicate_and_unknown_fields() {
        for payload in [
            "state=blocked:app=codex:kind=question",
            "state=blocked:app=antigravity:kind=question:id=child",
            "state=blocked:app=antigravity:kind=question:msg=a:msg=b",
            "state=blocked:app=antigravity:kind=question:extra=yes",
            "state=blocked:app=antigravity:kind=question:title=a",
            "state=clear",
        ] {
            let frame = format!("\x1b]7501;{payload}\x07");
            assert!(
                Reader::new(Profile::Antigravity)
                    .feed(frame.as_bytes())
                    .is_empty(),
                "{payload}"
            );
        }
        let mut reader = Reader::new(Profile::Antigravity);
        assert_eq!(reader.feed(b"\x1b]7501;state=blocked:app=antigravity:kind=permission:msg=discard\x1b\\\x1b]7501;state=idle:app=antigravity\x07"), [Status::Blocked, Status::Idle]);
        let oversized = format!("\x1b]7501;state=working:app=antigravity:msg={}\x07\x1b]7501;state=done:app=antigravity\x07", "x".repeat(256));
        assert_eq!(reader.feed(oversized.as_bytes()), [Status::Idle]);
    }

    #[test]
    fn antigravity_status_rejects_cancelled_controls_and_preserves_opaque_message() {
        for control in [0, 0x18, 0x1a, 0x7f, 0x1b] {
            let mut frame = b"\x1b]7501;state=blocked:app=antigravity:kind=question:msg=x".to_vec();
            frame.push(control);
            frame.extend_from_slice(b"\x07\x1b]7501;state=working:app=antigravity\x07");
            for split in 0..=frame.len() {
                let mut reader = Reader::new(Profile::Antigravity);
                let mut states = reader.feed(&frame[..split]);
                states.extend(reader.feed(&frame[split..]));
                assert_eq!(
                    states,
                    [Status::Working],
                    "control {control}, split {split}"
                );
            }
        }
        assert_eq!(
            Reader::new(Profile::Antigravity)
                .feed(b"\x1b]7501;state=blocked:app=antigravity:kind=auth:msg=\xff\xfe\x07"),
            [Status::Blocked]
        );
    }

    #[test]
    fn antigravity_cancel_without_terminator_recovers_the_next_frame() {
        for control in [0x18, 0x1a] {
            for prefix in [
                b"\x1b]7501;state=blocked:app=antigravity".as_slice(),
                b"\x1bPignored".as_slice(),
            ] {
                let mut frame = prefix.to_vec();
                frame.push(control);
                frame.extend_from_slice(b"\x1b]7501;state=working:app=antigravity\x07");
                for split in 0..=frame.len() {
                    let mut reader = Reader::new(Profile::Antigravity);
                    let mut statuses = reader.feed(&frame[..split]);
                    statuses.extend(reader.feed(&frame[split..]));
                    assert_eq!(
                        statuses,
                        [Status::Working],
                        "cancel {control}, split {split}"
                    );
                }
            }
        }
    }

    const MANUAL: &[u8] = b"\x1b]7501;state=idle:app=codex\x1b\\\x1b]7501;state=working:app=codex\x1b\\\x1b]7501;state=blocked:app=codex:kind=permission\x1b\\\x1b]7501;state=working:app=codex\x1b\\\x1b]7501;state=done:app=codex\x1b\\";

    #[test]
    fn terminal_status_manual_trace_survives_every_chunk_boundary() {
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
    fn terminal_status_clear_unknown_and_all_prompt_kinds() {
        let mut reader = Reader::default();
        assert_eq!(reader.feed(b"\x1b]7501;state=blocked:app=codex:kind=question\x07\x1b]7501;state=blocked:app=codex:kind=auth\x07\x1b]7501;state=error:app=codex\x07\x1b]7501;state=clear\x07\x1b]7501;state=future:app=codex\x07\x1b]7501;state=blocked:app=codex:kind=future\x07"), [Status::Blocked, Status::Blocked, Status::Failed, Status::Unknown, Status::Unknown, Status::Unknown]);
    }

    #[test]
    fn terminal_status_ignores_foreign_malformed_embedded_and_oversized_frames() {
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
