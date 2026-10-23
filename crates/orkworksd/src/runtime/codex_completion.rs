//! Serial completion correlation for independently qualified Codex launches.
//! Callers must supply a complete ordered hook stream, never receive-order
//! sequence numbers invented from the legacy UUID mailbox.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hook {
    Submitted,
    Pre,
    Permission,
    Post,
    Stop,
}

pub(super) struct Event<'a> {
    pub generation: u64,
    pub root: &'a str,
    pub turn: &'a str,
    pub sequence: u64,
    pub observed_micros: i64,
    pub hook: Hook,
    pub tool: Option<&'a str>,
    pub invocation: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub(super) struct Candidate {
    owner: std::sync::Arc<()>,
    epoch: u64,
    invocation: usize,
}
impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, &other.owner)
            && self.epoch == other.epoch
            && self.invocation == other.invocation
    }
}
impl Eq for Candidate {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    BindWait(Candidate),
    ClearWait(Candidate),
}

struct Invocation {
    id: String,
    tool: String,
    completed: bool,
}

pub(super) struct Tracker {
    generation: u64,
    identity: std::sync::Arc<()>,
    epoch: Option<u64>,
    root: String,
    turn: String,
    cursor: Option<(u64, i64)>,
    active: bool,
    records: Vec<Invocation>,
    pending: Option<Candidate>,
    bound: bool,
    issued: Option<Effect>,
}

impl Tracker {
    pub fn new(generation: u64) -> Self {
        Self {
            generation,
            identity: std::sync::Arc::new(()),
            epoch: Some(0),
            root: String::new(),
            turn: String::new(),
            cursor: None,
            active: false,
            records: Vec::new(),
            pending: None,
            bound: false,
            issued: None,
        }
    }

    /// `clean_submission` comes from the caller's qualified launch, actual
    /// child-context bootstrap, accepted root and persisted working tuple.
    pub fn event(&mut self, event: Event<'_>, clean_submission: bool) -> Option<Effect> {
        if self.issued.is_some() {
            // An unacknowledged persistence outcome is not a usable baseline.
            self.invalidate();
        }
        if event.generation != self.generation || !bounded(event.root) || !bounded(event.turn) {
            self.invalidate();
            return None;
        }
        let ordered = self
            .cursor
            .is_none_or(|(seq, time)| event.sequence > seq && event.observed_micros > time);
        if !ordered {
            self.invalidate();
            return None;
        }
        let contiguous = self
            .cursor
            .is_some_and(|(seq, _)| seq.checked_add(1) == Some(event.sequence));
        self.cursor = Some((event.sequence, event.observed_micros));
        if event.hook == Hook::Submitted {
            self.invalidate();
            if clean_submission && self.epoch.is_some() {
                self.active = true;
                self.root = event.root.to_owned();
                self.turn = event.turn.to_owned();
                self.records.clear();
            }
            return None;
        }
        if event.hook == Hook::Stop
            || !contiguous
            || event.root != self.root
            || event.turn != self.turn
        {
            self.invalidate();
            return None;
        }
        if !self.active {
            return None;
        }
        let Some(tool) = event.tool.filter(|tool| bounded(tool)) else {
            self.invalidate();
            return None;
        };
        let invocation = event.invocation.filter(|id| bounded(id));
        match event.hook {
            Hook::Pre => {
                let Some(id) = invocation else {
                    self.invalidate();
                    return None;
                };
                if self.records.len() >= 64
                    || self.pending.is_some()
                    || self.records.iter().any(|r| !r.completed || r.id == id)
                {
                    self.invalidate();
                    return None;
                }
                self.records.push(Invocation {
                    id: id.to_owned(),
                    tool: tool.to_owned(),
                    completed: false,
                });
            }
            Hook::Permission => {
                let index = self.records.iter().position(|r| !r.completed);
                let Some(index) = index else {
                    self.invalidate();
                    return None;
                };
                let record = &self.records[index];
                if self.pending.is_some()
                    || record.tool != tool
                    || event.invocation.is_some() && invocation != Some(record.id.as_str())
                {
                    self.invalidate();
                    return None;
                }
                let candidate = Candidate {
                    owner: self.identity.clone(),
                    epoch: self.epoch?,
                    invocation: index,
                };
                self.pending = Some(candidate.clone());
                self.bound = false;
                self.issued = Some(Effect::BindWait(candidate));
            }
            Hook::Post => {
                let Some(id) = invocation else {
                    self.invalidate();
                    return None;
                };
                let Some(index) = self.records.iter().position(|r| r.id == id) else {
                    self.invalidate();
                    return None;
                };
                let record = &mut self.records[index];
                if record.completed || record.tool != tool {
                    self.invalidate();
                    return None;
                }
                record.completed = true;
                if let Some(candidate) = self.pending.as_ref() {
                    if candidate.invocation != index || !self.bound {
                        self.invalidate();
                        return None;
                    }
                    self.issued = Some(Effect::ClearWait(candidate.clone()));
                }
            }
            Hook::Submitted | Hook::Stop => unreachable!(),
        }
        self.issued.clone()
    }

    pub fn acknowledge_wait(&mut self, effect: &Effect) -> bool {
        if !matches!(effect, Effect::BindWait(_)) || !self.accepts(&effect) {
            return false;
        }
        self.bound = true;
        self.issued = None;
        true
    }

    pub fn acknowledge_clear(&mut self, effect: &Effect) -> bool {
        if !matches!(effect, Effect::ClearWait(_)) || !self.accepts(&effect) {
            return false;
        }
        self.pending = None;
        self.bound = false;
        self.issued = None;
        true
    }

    pub fn has_baseline(&self) -> bool {
        self.active
    }

    pub fn accepts(&self, effect: &Effect) -> bool {
        self.active && self.issued.as_ref() == Some(effect)
    }

    pub fn invalidate(&mut self) {
        self.active = false;
        self.pending = None;
        self.bound = false;
        self.issued = None;
        self.epoch = self.epoch.and_then(|epoch| epoch.checked_add(1));
    }
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(hook: Hook, sequence: u64, invocation: Option<&str>) -> Event<'_> {
        Event {
            generation: 7,
            root: "root",
            turn: "turn",
            sequence,
            observed_micros: sequence as i64,
            hook,
            tool: Some("Bash"),
            invocation,
        }
    }

    fn baseline() -> Tracker {
        let mut tracker = Tracker::new(7);
        assert_eq!(tracker.event(event(Hook::Submitted, 1, None), true), None);
        tracker
    }

    fn wait(tracker: &mut Tracker) -> Effect {
        assert_eq!(
            tracker.event(event(Hook::Pre, 2, Some("tool-1")), false),
            None
        );
        let effect = tracker
            .event(event(Hook::Permission, 3, None), false)
            .expect("serial permission must offer an immediate owned wait");
        assert!(matches!(effect, Effect::BindWait(_)));
        assert!(tracker.acknowledge_wait(&effect));
        effect
    }

    #[test]
    fn exact_serial_completion_clears_only_acknowledged_wait_once() {
        let mut tracker = baseline();
        let bound = wait(&mut tracker);
        let clear = tracker
            .event(event(Hook::Post, 4, Some("tool-1")), false)
            .expect("exact serial completion must offer a clear");
        let Effect::BindWait(candidate) = bound else {
            panic!("expected binding")
        };
        assert_eq!(clear, Effect::ClearWait(candidate));
        assert!(tracker.accepts(&clear));
        assert!(tracker.acknowledge_clear(&clear));
        assert!(!tracker.acknowledge_clear(&clear));
        assert!(!tracker.accepts(&clear));
        assert_eq!(
            tracker.event(event(Hook::Post, 5, Some("tool-1")), false),
            None
        );
    }

    #[test]
    fn no_prompt_boundary_or_unqualified_submission_cannot_bind_a_wait() {
        for clean in [false, true] {
            let mut tracker = Tracker::new(7);
            if !clean {
                tracker.event(event(Hook::Submitted, 1, None), false);
            }
            assert_eq!(
                tracker.event(event(Hook::Pre, 2, Some("tool-1")), false),
                None
            );
            assert_eq!(tracker.event(event(Hook::Permission, 3, None), false), None);
            assert_eq!(
                tracker.event(event(Hook::Post, 4, Some("tool-1")), false),
                None
            );
        }
    }

    #[test]
    fn failed_wait_persistence_never_grants_completion_rights() {
        let mut tracker = baseline();
        tracker.event(event(Hook::Pre, 2, Some("tool-1")), false);
        let effect = tracker
            .event(event(Hook::Permission, 3, None), false)
            .unwrap();
        assert!(tracker.accepts(&effect));
        assert_eq!(
            tracker.event(event(Hook::Post, 4, Some("tool-1")), false),
            None
        );
        assert!(!tracker.acknowledge_wait(&effect));
    }
    #[test]
    fn proofs_cannot_be_transferred_between_trackers() {
        let mut a = baseline();
        let mut b = baseline();
        for tracker in [&mut a, &mut b] {
            tracker.event(event(Hook::Pre, 2, Some("tool-1")), false);
        }
        let first = a.event(event(Hook::Permission, 3, None), false).unwrap();
        let second = b.event(event(Hook::Permission, 3, None), false).unwrap();
        assert!(!b.acknowledge_wait(&first));
        assert!(b.acknowledge_wait(&second));
    }

    #[test]
    fn ambiguous_events_lock_clearing_until_a_new_prompt_boundary() {
        for variant in 0..12 {
            let mut tracker = baseline();
            wait(&mut tracker);
            let mut bad = event(Hook::Post, 4, Some("tool-1"));
            match variant {
                0 => bad.sequence = 5,        // missing event
                1 => bad.sequence = 3,        // duplicate/reverse delivery
                2 => bad.observed_micros = 3, // equal source timestamps
                3 => bad.generation = 8,
                4 => bad.root = "child",
                5 => bad.turn = "other-turn",
                6 => bad.invocation = None,
                7 => bad.invocation = Some("other-tool"),
                8 => bad.tool = Some("Other"),
                9 => bad.hook = Hook::Permission, // second pending request
                10 => {
                    bad.hook = Hook::Pre;
                    bad.invocation = Some("parallel-tool");
                }
                11 => bad.hook = Hook::Stop,
                _ => unreachable!(),
            }
            assert_eq!(tracker.event(bad, false), None, "variant {variant}");
            assert_eq!(
                tracker.event(event(Hook::Post, 6, Some("tool-1")), false),
                None
            );
            tracker.event(event(Hook::Submitted, 10, None), true);
            tracker.event(event(Hook::Pre, 11, Some("fresh-tool")), false);
            let bound = tracker
                .event(event(Hook::Permission, 12, None), false)
                .unwrap();
            assert!(tracker.acknowledge_wait(&bound));
            assert!(matches!(
                tracker.event(event(Hook::Post, 13, Some("fresh-tool")), false),
                Some(Effect::ClearWait(_))
            ));
        }
    }

    #[test]
    fn prior_completion_cannot_clear_a_later_same_tool_request() {
        let mut tracker = baseline();
        wait(&mut tracker);
        let first = tracker
            .event(event(Hook::Post, 4, Some("tool-1")), false)
            .unwrap();
        assert!(tracker.acknowledge_clear(&first));
        tracker.event(event(Hook::Pre, 5, Some("tool-2")), false);
        let second = tracker
            .event(event(Hook::Permission, 6, None), false)
            .unwrap();
        assert!(tracker.acknowledge_wait(&second));
        assert_eq!(
            tracker.event(event(Hook::Post, 7, Some("tool-1")), false),
            None
        );
        assert_eq!(
            tracker.event(event(Hook::Post, 8, Some("tool-2")), false),
            None
        );
    }

    #[test]
    fn completed_id_budget_and_epoch_exhaustion_fail_closed() {
        let mut tracker = baseline();
        for index in 0..63 {
            let id = format!("tool-{index}");
            tracker.event(event(Hook::Pre, 2 + index * 2, Some(&id)), false);
            tracker.event(event(Hook::Post, 3 + index * 2, Some(&id)), false);
        }
        tracker.event(event(Hook::Pre, 128, Some("last")), false);
        let bound = tracker
            .event(event(Hook::Permission, 129, None), false)
            .unwrap();
        assert!(tracker.acknowledge_wait(&bound));
        let clear = tracker
            .event(event(Hook::Post, 130, Some("last")), false)
            .unwrap();
        assert!(tracker.acknowledge_clear(&clear));
        tracker.event(event(Hook::Pre, 131, Some("overflow")), false);
        assert_eq!(
            tracker.event(event(Hook::Permission, 132, None), false),
            None
        );
        tracker.epoch = Some(u64::MAX);
        tracker.invalidate();
        tracker.event(event(Hook::Submitted, 133, None), true);
        tracker.event(event(Hook::Pre, 134, Some("fresh")), false);
        assert_eq!(
            tracker.event(event(Hook::Permission, 135, None), false),
            None
        );
    }
}
