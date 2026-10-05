//! Pure, runtime-local correlation of validated Codex hooks and native status.
use std::time::{Duration, Instant};

const GRACE: Duration = Duration::from_secs(2);
const FRESHNESS: Duration = Duration::from_millis(300);
const MAX_RECORDS: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fence {
    pub runtime_generation: u64,
    pub identity_revision: u64,
    pub turn_revision: u64,
    pub input_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CandidateId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HookEvent {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    Stop,
    UserPromptSubmit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeStatus {
    Active,
    ApprovalPending,
    UserInputPending,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    ShowWait {
        candidate: CandidateId,
        fence: Fence,
    },
    ClearWait {
        candidate: CandidateId,
        fence: Fence,
    },
}

#[derive(Clone, Copy)]
struct Observation {
    status: NativeStatus,
    started_at: Instant,
}

struct Candidate {
    id: CandidateId,
    eligible: bool,
    saw_pending_edge: bool,
    retired: bool,
}

struct Record {
    tool_id: Option<String>,
    has_pre: bool,
    completed: bool,
    candidate: Option<Candidate>,
}

#[derive(Clone, Copy)]
struct Wait {
    candidate: CandidateId,
    received_at: Instant,
    shown: bool,
}

/// This reducer grants correlation rights, never attention-write ownership.
/// The caller must apply effects under its attention lock, recheck observation
/// freshness at commit, and invalidate with the new fence after trusted input,
/// turn, identity, authority, or runtime transitions.
pub(crate) struct ApprovalReducer {
    fence: Fence,
    records: Vec<Record>,
    latest_pre: Option<usize>,
    observation: Option<Observation>,
    last_observation_start: Option<Instant>,
    clock: Option<Instant>,
    wait: Option<Wait>,
    locked: bool,
    next_candidate: u64,
    exhausted: bool,
    issued: Option<Effect>,
}

impl ApprovalReducer {
    pub(crate) fn new(fence: Fence) -> Self {
        Self {
            fence,
            records: Vec::new(),
            latest_pre: None,
            observation: None,
            last_observation_start: None,
            clock: None,
            wait: None,
            locked: false,
            next_candidate: 0,
            exhausted: false,
            issued: None,
        }
    }

    pub(crate) fn hook(
        &mut self,
        event: HookEvent,
        tool_id: Option<&str>,
        now: Instant,
    ) -> Vec<Effect> {
        self.issued = None;
        if matches!(event, HookEvent::Stop | HookEvent::UserPromptSubmit) {
            self.reset();
            return Vec::new();
        }
        self.advance_clock(now);
        match event {
            HookEvent::PreToolUse => self.pre(tool_id),
            HookEvent::PermissionRequest => self.permission(tool_id, now),
            HookEvent::PostToolUse => {
                if self.wait.is_some()
                    && !self
                        .observation
                        .is_some_and(|o| Self::fresh(o.started_at, now))
                {
                    self.lock();
                }
                if let Some(effect) = self.post(tool_id) {
                    return self.issue(effect);
                }
            }
            HookEvent::Stop | HookEvent::UserPromptSubmit => unreachable!(),
        }
        self.display_due(now)
    }

    /// `started_at` is the first RPC time, `now` is current receive time.
    /// Invalid, future, or reordered observations cannot provide a baseline,
    /// attribution edge, suppression, or clear authority.
    pub(crate) fn observe(
        &mut self,
        status: NativeStatus,
        complete_singleton_root: bool,
        started_at: Instant,
        now: Instant,
    ) -> Vec<Effect> {
        self.issued = None;
        let ordered_now = self.advance_clock(now);
        let ordered_start = self.last_observation_start.is_none_or(|last| {
            started_at > last
                || (started_at == last && self.observation.is_some_and(|o| o.status == status))
        });
        let fresh = Self::fresh(started_at, now);
        if !ordered_now || !ordered_start || !fresh || !complete_singleton_root {
            self.lock();
            self.observation = None;
            return self.display_due(now);
        }
        self.last_observation_start = Some(started_at);
        let previous = self.observation.map(|o| o.status);
        self.observation = Some(Observation { status, started_at });
        match status {
            NativeStatus::ApprovalPending => {
                if !self.locked {
                    let index = self.serial_candidate();
                    let can_attribute = self.wait.is_some_and(|w| started_at >= w.received_at)
                        && previous == Some(NativeStatus::Active);
                    match index {
                        Some(index) if can_attribute => {
                            if let Some(candidate) = &mut self.records[index].candidate {
                                candidate.saw_pending_edge = true;
                            }
                        }
                        Some(index)
                            if self.records[index]
                                .candidate
                                .as_ref()
                                .is_some_and(|c| c.saw_pending_edge) => {}
                        _ => self.lock(),
                    }
                }
            }
            NativeStatus::Active => {
                if !self.locked && previous == Some(NativeStatus::ApprovalPending) {
                    if let Some(index) = self.serial_candidate() {
                        if self.records[index]
                            .candidate
                            .as_ref()
                            .is_some_and(|c| c.saw_pending_edge)
                        {
                            if let Some(effect) = self.retire(index) {
                                return self.issue(effect);
                            }
                        }
                    }
                }
            }
            NativeStatus::UserInputPending => self.lock(),
            NativeStatus::Unavailable => {
                if self.wait.is_some() {
                    self.lock();
                }
                self.observation = None;
            }
        }
        self.display_due(now)
    }

    pub(crate) fn tick(&mut self, now: Instant) -> Vec<Effect> {
        if !self.advance_clock(now) {
            self.issued = None;
        }
        if self.wait.is_some()
            && !self
                .observation
                .is_some_and(|o| Self::fresh(o.started_at, now))
        {
            if !self.locked {
                self.issued = None;
            }
            self.lock();
        }
        self.display_due(now)
    }

    pub(crate) fn invalidate(&mut self, fence: Fence) {
        self.reset();
        self.fence = fence;
    }

    pub(crate) fn disconnected(&mut self) {
        self.issued = None;
        self.observation = None;
        self.last_observation_start = None;
        if self.wait.is_some() {
            self.lock();
        }
    }

    /// False after identity exhaustion: the caller must use ordinary
    /// conservative hook reporting rather than suppress a permission wait.
    pub(crate) fn assistance_available(&self) -> bool {
        !self.exhausted
    }

    /// A retired candidate's clear remains acceptable until the next hook,
    /// observation, disconnect, or trusted boundary. A no-op tick preserves it.
    /// This guard does not replace the caller's fences and ownership token.
    pub(crate) fn accepts(&self, effect: &Effect) -> bool {
        self.issued.as_ref() == Some(effect)
    }

    fn reset(&mut self) {
        self.records.clear();
        self.latest_pre = None;
        self.observation = None;
        self.last_observation_start = None;
        self.clock = None;
        self.wait = None;
        self.locked = self.exhausted;
        self.issued = None;
        // Candidate identity never repeats, including after an identity reset.
    }

    fn lock(&mut self) {
        self.locked = true;
        for record in &mut self.records {
            if let Some(candidate) = &mut record.candidate {
                candidate.eligible = false;
            }
        }
    }

    fn advance_clock(&mut self, now: Instant) -> bool {
        if self.clock.is_some_and(|last| now < last) {
            self.lock();
            self.observation = None;
            return false;
        }
        self.clock = Some(now);
        true
    }

    fn fresh(started_at: Instant, now: Instant) -> bool {
        now.checked_duration_since(started_at)
            .is_some_and(|age| age <= FRESHNESS)
    }

    fn find(&self, tool_id: &str) -> Option<usize> {
        self.records
            .iter()
            .position(|r| r.tool_id.as_deref() == Some(tool_id))
    }

    fn add_record(
        &mut self,
        tool_id: Option<&str>,
        has_pre: bool,
        completed: bool,
    ) -> Option<usize> {
        if self.records.len() >= MAX_RECORDS {
            self.lock();
            return None;
        }
        let index = self.records.len();
        self.records.push(Record {
            tool_id: tool_id.map(str::to_owned),
            has_pre,
            completed,
            candidate: None,
        });
        Some(index)
    }

    fn pre(&mut self, tool_id: Option<&str>) {
        let tool_id = tool_id.filter(|id| !id.is_empty());
        if let Some(id) = tool_id {
            if self.find(id).is_some() {
                return;
            }
        } else {
            self.lock();
        }
        if self.records.iter().any(|r| !r.completed) {
            self.lock();
        }
        self.latest_pre = self.add_record(tool_id, true, false);
    }

    fn permission(&mut self, tool_id: Option<&str>, now: Instant) {
        let index = match tool_id {
            Some(id) if !id.is_empty() => match self.find(id) {
                Some(index) => Some(index),
                None => {
                    self.lock();
                    self.add_record(Some(id), false, false)
                }
            },
            Some(_) => {
                self.lock();
                self.add_record(None, false, false)
            }
            None => self
                .latest_pre
                .filter(|&i| {
                    let record = &self.records[i];
                    !record.completed && !record.candidate.as_ref().is_some_and(|c| c.retired)
                })
                .or_else(|| {
                    self.records.iter().position(|r| {
                        r.tool_id.is_none()
                            && !r.completed
                            && r.candidate.as_ref().is_some_and(|c| !c.retired)
                    })
                })
                .or_else(|| {
                    self.lock();
                    self.add_record(None, false, false)
                }),
        };
        // Only an identified replay can inherit a tombstone. An anonymous
        // repeat may be a second independent permission: preserve the original
        // deadline, but revoke clearing rather than assume delivery deduplication.
        if index.is_some_and(|i| self.records[i].completed || self.records[i].candidate.is_some()) {
            if tool_id.is_none() {
                self.lock();
            }
            return;
        }
        let active = self.records.iter().filter(|r| !r.completed).count();
        let baseline = self
            .observation
            .is_some_and(|o| o.status == NativeStatus::Active && Self::fresh(o.started_at, now));
        let chain =
            index.is_some_and(|i| self.records[i].has_pre && self.records[i].tool_id.is_some());
        if !baseline || !chain || active != 1 || self.wait.is_some() {
            self.lock();
        }
        let Some(next) = self.next_candidate.checked_add(1) else {
            self.exhausted = true;
            self.lock();
            return;
        };
        self.next_candidate = next;
        let candidate = CandidateId(next);
        if let Some(index) = index {
            self.records[index].candidate = Some(Candidate {
                id: candidate,
                eligible: !self.locked,
                saw_pending_edge: false,
                retired: false,
            });
        }
        // A locked batch can outlive every correlated invocation: completing a
        // new tool does not prove an older anonymous approval/question ended.
        if self.wait.is_none() {
            self.wait = Some(Wait {
                candidate,
                received_at: now,
                shown: false,
            });
        }
    }

    fn post(&mut self, tool_id: Option<&str>) -> Option<Effect> {
        let Some(id) = tool_id.filter(|id| !id.is_empty()) else {
            self.lock();
            return None;
        };
        let Some(index) = self.find(id) else {
            self.lock();
            self.add_record(Some(id), false, true);
            return None;
        };
        if self.records[index].completed {
            return None;
        }
        self.records[index].completed = true;
        if self.records[index].candidate.is_none() {
            return None;
        }
        self.retire(index)
    }

    fn serial_candidate(&self) -> Option<usize> {
        if self.locked {
            return None;
        }
        self.records.iter().position(|r| {
            r.candidate
                .as_ref()
                .is_some_and(|c| c.eligible && !c.retired)
        })
    }

    fn retire(&mut self, index: usize) -> Option<Effect> {
        let candidate = self.records[index].candidate.as_mut()?;
        if candidate.retired {
            return None;
        }
        candidate.retired = true;
        if self.locked || !candidate.eligible {
            return None;
        }
        let wait = self.wait?;
        if wait.candidate != candidate.id {
            return None;
        }
        self.wait = None;
        wait.shown.then_some(Effect::ClearWait {
            candidate: wait.candidate,
            fence: self.fence,
        })
    }

    fn display_due(&mut self, now: Instant) -> Vec<Effect> {
        let Some(wait) = self.wait else {
            return Vec::new();
        };
        if wait.shown
            || !now
                .checked_duration_since(wait.received_at)
                .is_some_and(|age| age >= GRACE)
        {
            return Vec::new();
        }
        let active_without_flags = !self.locked
            && self.observation.is_some_and(|o| {
                o.status == NativeStatus::Active && Self::fresh(o.started_at, now)
            });
        if active_without_flags {
            return Vec::new();
        }
        if let Some(wait) = &mut self.wait {
            wait.shown = true;
        }
        self.issue(Effect::ShowWait {
            candidate: wait.candidate,
            fence: self.fence,
        })
    }

    fn issue(&mut self, effect: Effect) -> Vec<Effect> {
        self.issued = Some(effect);
        vec![effect]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serial(t: Instant) -> ApprovalReducer {
        let mut reducer = ApprovalReducer::new(Fence::default());
        assert!(reducer.observe(NativeStatus::Active, true, t, t).is_empty());
        assert!(reducer
            .hook(
                HookEvent::PreToolUse,
                Some("tool-1"),
                t + Duration::from_millis(1)
            )
            .is_empty());
        assert!(reducer
            .hook(
                HookEvent::PermissionRequest,
                None,
                t + Duration::from_millis(2)
            )
            .is_empty());
        reducer
    }
    fn show(reducer: &mut ApprovalReducer, t: Instant) -> Effect {
        let effects = reducer.observe(NativeStatus::ApprovalPending, true, t, t);
        assert!(
            matches!(effects.as_slice(), [Effect::ShowWait { .. }]),
            "{effects:?}"
        );
        effects[0]
    }
    fn assert_clear(effects: Vec<Effect>, shown: Effect) -> Effect {
        let Effect::ShowWait { candidate, fence } = shown else {
            panic!("expected ShowWait")
        };
        assert_eq!(effects, vec![Effect::ClearWait { candidate, fence }]);
        effects[0]
    }

    // Catch early display, deadline renewal, and repeated writes.
    #[test]
    fn fixed_deadline_and_duplicate_permission_show_once() {
        let t = Instant::now();
        let mut r = serial(t);
        assert!(r
            .hook(
                HookEvent::PermissionRequest,
                None,
                t + Duration::from_secs(1)
            )
            .is_empty());
        assert!(r.tick(t + Duration::from_millis(2001)).is_empty());
        show(&mut r, t + Duration::from_millis(2002));
        assert!(r.tick(t + Duration::from_secs(3)).is_empty());
        assert!(r
            .hook(
                HookEvent::PermissionRequest,
                Some("tool-1"),
                t + Duration::from_secs(4)
            )
            .is_empty());
    }

    #[test]
    fn attributable_pending_edge_clears_before_tool_finishes() {
        let t = Instant::now();
        let mut r = serial(t);
        let shown = show(&mut r, t + Duration::from_secs(3));
        let at = t + Duration::from_millis(3100);
        let cleared = assert_clear(r.observe(NativeStatus::Active, true, at, at), shown);
        assert!(r.accepts(&cleared));
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(!r.accepts(&cleared));
        assert!(r
            .hook(HookEvent::PermissionRequest, Some("tool-1"), at)
            .is_empty());
        assert!(r.tick(t + Duration::from_secs(10)).is_empty());
    }

    #[test]
    fn slow_active_tool_stays_hidden_but_late_pending_can_show() {
        let t = Instant::now();
        let mut r = serial(t);
        for seconds in [2, 4, 8, 20] {
            let at = t + Duration::from_secs(seconds);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
            assert!(r.tick(at + Duration::from_millis(300)).is_empty());
        }
        show(&mut r, t + Duration::from_secs(21));
    }

    #[test]
    fn auto_completion_retires_hidden_candidate_and_suppresses_late_duplicates() {
        let t = Instant::now();
        let mut r = serial(t);
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("tool-1"),
                t + Duration::from_millis(200)
            )
            .is_empty());
        assert!(r
            .hook(
                HookEvent::PermissionRequest,
                Some("tool-1"),
                t + Duration::from_secs(1)
            )
            .is_empty());
        assert!(r.tick(t + Duration::from_secs(3)).is_empty());
    }

    #[test]
    fn exact_completion_clears_only_serial_owned_wait() {
        let t = Instant::now();
        let mut r = serial(t);
        let shown = show(&mut r, t + Duration::from_secs(3));
        let clear = assert_clear(
            r.hook(
                HookEvent::PostToolUse,
                Some("tool-1"),
                t + Duration::from_millis(3100),
            ),
            shown,
        );
        assert!(r.accepts(&clear));
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("tool-1"),
                t + Duration::from_millis(3200)
            )
            .is_empty());
        assert!(!r.accepts(&clear));
    }

    #[test]
    fn preexisting_pending_survives_new_exact_completion_and_active() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        assert!(r
            .observe(NativeStatus::ApprovalPending, true, t, t)
            .is_empty());
        r.hook(HookEvent::PreToolUse, Some("new-tool"), t);
        r.hook(HookEvent::PermissionRequest, None, t);
        show(&mut r, t + Duration::from_secs(2));
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("new-tool"),
                t + Duration::from_secs(3)
            )
            .is_empty());
        let at = t + Duration::from_secs(4);
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r.tick(t + Duration::from_secs(10)).is_empty());
    }

    #[test]
    fn preexisting_pending_completed_before_grace_still_retains_conservative_batch() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        r.observe(NativeStatus::ApprovalPending, true, t, t);
        r.hook(HookEvent::PreToolUse, Some("new-tool"), t);
        r.hook(HookEvent::PermissionRequest, None, t);
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("new-tool"),
                t + Duration::from_millis(100)
            )
            .is_empty());
        let effects = r.tick(t + Duration::from_secs(2));
        assert!(matches!(effects.as_slice(), [Effect::ShowWait { .. }]));
    }

    #[test]
    fn disconnected_edge_is_never_reconstructed() {
        let t = Instant::now();
        let mut r = serial(t);
        let shown = show(&mut r, t + Duration::from_secs(3));
        assert!(r.accepts(&shown));
        r.disconnected();
        assert!(!r.accepts(&shown));
        let at = t + Duration::from_secs(4);
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r
            .hook(HookEvent::PostToolUse, Some("tool-1"), at)
            .is_empty());
    }

    #[test]
    fn reconnect_requires_new_baseline_before_new_candidate() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        r.observe(NativeStatus::Active, true, t, t);
        r.disconnected();
        r.hook(HookEvent::PreToolUse, Some("tool-1"), t);
        r.hook(HookEvent::PermissionRequest, None, t);
        let shown = show(&mut r, t + Duration::from_secs(2));
        let at = t + Duration::from_secs(3);
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(!r.accepts(&shown));
    }

    #[test]
    fn question_status_never_clears_or_regrants_batch_authority() {
        let t = Instant::now();
        let mut r = serial(t);
        show(&mut r, t + Duration::from_secs(3));
        let at = t + Duration::from_secs(4);
        assert!(r
            .observe(NativeStatus::UserInputPending, true, at, at)
            .is_empty());
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r
            .hook(HookEvent::PostToolUse, Some("tool-1"), at)
            .is_empty());
    }

    #[test]
    fn unavailable_idle_error_never_clear_pending_wait() {
        let t = Instant::now();
        let mut r = serial(t);
        show(&mut r, t + Duration::from_secs(3));
        let at = t + Duration::from_secs(4);
        assert!(r
            .observe(NativeStatus::Unavailable, true, at, at)
            .is_empty());
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
    }

    #[test]
    fn unavailable_and_stale_observations_display_after_deadline() {
        for status in [NativeStatus::Unavailable, NativeStatus::Active] {
            let t = Instant::now();
            let mut r = serial(t);
            let at = t + Duration::from_millis(2002);
            let started = if status == NativeStatus::Active {
                at - Duration::from_millis(301)
            } else {
                at
            };
            assert!(matches!(
                r.observe(status, true, started, at).as_slice(),
                [Effect::ShowWait { .. }]
            ));
            assert!(r
                .hook(HookEvent::PostToolUse, Some("tool-1"), at)
                .is_empty());
        }
    }

    #[test]
    fn baseline_must_be_fresh_at_candidate_receipt() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        r.observe(NativeStatus::Active, true, t, t);
        let at = t + Duration::from_millis(301);
        r.hook(HookEvent::PreToolUse, Some("tool-1"), at);
        r.hook(HookEvent::PermissionRequest, None, at);
        show(&mut r, at + Duration::from_secs(2));
        assert!(r
            .observe(
                NativeStatus::Active,
                true,
                at + Duration::from_secs(3),
                at + Duration::from_secs(3)
            )
            .is_empty());
    }

    #[test]
    fn stale_future_and_reordered_samples_never_clear() {
        for invalid in [0, 1, 2] {
            let t = Instant::now();
            let mut r = serial(t);
            show(&mut r, t + Duration::from_secs(3));
            let now = t + Duration::from_secs(4);
            let started = match invalid {
                0 => now - Duration::from_millis(301),
                1 => now + Duration::from_millis(1),
                _ => t + Duration::from_secs(2),
            };
            assert!(r
                .observe(NativeStatus::Active, true, started, now)
                .is_empty());
            assert!(r.observe(NativeStatus::Active, true, now, now).is_empty());
        }
    }

    #[test]
    fn freshness_boundary_keeps_active_hidden_then_expiry_shows() {
        let t = Instant::now();
        let mut r = serial(t);
        let at = t + Duration::from_secs(3);
        assert!(r
            .observe(
                NativeStatus::Active,
                true,
                at - Duration::from_millis(300),
                at
            )
            .is_empty());
        assert!(matches!(
            r.tick(at + Duration::from_millis(1)).as_slice(),
            [Effect::ShowWait { .. }]
        ));
    }

    #[test]
    fn missing_pre_or_completion_ids_never_clear() {
        for missing_pre in [true, false] {
            let t = Instant::now();
            let mut r = ApprovalReducer::new(Fence::default());
            r.observe(NativeStatus::Active, true, t, t);
            if !missing_pre {
                r.hook(HookEvent::PreToolUse, Some("tool-1"), t);
            }
            r.hook(HookEvent::PermissionRequest, None, t);
            show(&mut r, t + Duration::from_secs(2));
            assert!(r
                .hook(HookEvent::PostToolUse, None, t + Duration::from_secs(3))
                .is_empty());
            let at = t + Duration::from_secs(4);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        }
    }

    #[test]
    fn missing_or_empty_pre_scalar_is_conservative() {
        for scalar in [None, Some("")] {
            let t = Instant::now();
            let mut r = ApprovalReducer::new(Fence::default());
            r.observe(NativeStatus::Active, true, t, t);
            r.hook(HookEvent::PreToolUse, scalar, t);
            r.hook(HookEvent::PermissionRequest, None, t);
            show(&mut r, t + Duration::from_secs(2));
            let at = t + Duration::from_secs(3);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        }
    }

    #[test]
    fn overlapping_pre_chains_and_permissions_lock_even_after_one_completion() {
        for second_permission in [false, true] {
            let t = Instant::now();
            let mut r = serial(t);
            r.hook(
                HookEvent::PreToolUse,
                Some("tool-2"),
                t + Duration::from_millis(3),
            );
            if second_permission {
                r.hook(
                    HookEvent::PermissionRequest,
                    Some("tool-2"),
                    t + Duration::from_millis(4),
                );
            }
            show(&mut r, t + Duration::from_secs(3));
            assert!(r
                .hook(
                    HookEvent::PostToolUse,
                    Some("tool-1"),
                    t + Duration::from_secs(4)
                )
                .is_empty());
            assert!(r
                .hook(
                    HookEvent::PostToolUse,
                    Some("tool-2"),
                    t + Duration::from_secs(4)
                )
                .is_empty());
            let at = t + Duration::from_secs(5);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        }
    }

    #[test]
    fn mismatched_permission_scalar_and_reordered_completion_lock_batch() {
        for mismatch in [true, false] {
            let t = Instant::now();
            let mut r = serial(t);
            if mismatch {
                r.hook(
                    HookEvent::PermissionRequest,
                    Some("unknown"),
                    t + Duration::from_millis(3),
                );
            } else {
                r.hook(
                    HookEvent::PostToolUse,
                    Some("unknown"),
                    t + Duration::from_millis(3),
                );
            }
            show(&mut r, t + Duration::from_secs(3));
            assert!(r
                .hook(
                    HookEvent::PostToolUse,
                    Some("tool-1"),
                    t + Duration::from_secs(4)
                )
                .is_empty());
            let at = t + Duration::from_secs(5);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        }
    }

    #[test]
    fn multiple_loaded_roots_lock_until_trusted_boundary() {
        let t = Instant::now();
        let mut r = serial(t);
        let at = t + Duration::from_secs(3);
        assert!(matches!(
            r.observe(NativeStatus::ApprovalPending, false, at, at)
                .as_slice(),
            [Effect::ShowWait { .. }]
        ));
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r
            .hook(HookEvent::PostToolUse, Some("tool-1"), at)
            .is_empty());
        r.hook(HookEvent::Stop, None, at);
        r.observe(NativeStatus::Active, true, at, at);
        r.hook(HookEvent::PreToolUse, Some("tool-2"), at);
        r.hook(HookEvent::PermissionRequest, None, at);
        let shown = show(&mut r, at + Duration::from_secs(2));
        let later = at + Duration::from_secs(3);
        assert_clear(r.observe(NativeStatus::Active, true, later, later), shown);
    }

    #[test]
    fn retired_tombstones_do_not_reopen_and_bound_overflow_locks() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        for i in 0..64 {
            let id = format!("tool-{i}");
            let at = t + Duration::from_millis(i * 10);
            r.observe(NativeStatus::Active, true, at, at);
            r.hook(HookEvent::PreToolUse, Some(&id), at);
            r.hook(HookEvent::PermissionRequest, Some(&id), at);
            assert!(r.hook(HookEvent::PostToolUse, Some(&id), at).is_empty());
        }
        let at = t + Duration::from_secs(1);
        assert!(r
            .hook(HookEvent::PermissionRequest, Some("tool-0"), at)
            .is_empty());
        r.observe(NativeStatus::Active, true, at, at);
        r.hook(HookEvent::PreToolUse, Some("overflow"), at);
        r.hook(HookEvent::PermissionRequest, Some("overflow"), at);
        show(&mut r, at + Duration::from_secs(2));
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("overflow"),
                at + Duration::from_secs(3)
            )
            .is_empty());
        let later = at + Duration::from_secs(4);
        assert!(r
            .observe(NativeStatus::Active, true, later, later)
            .is_empty());
        assert!(r
            .hook(HookEvent::PermissionRequest, Some("tool-0"), later)
            .is_empty());
        assert!(r.tick(later).is_empty());
    }

    #[test]
    fn exact_completion_cannot_use_expired_singleton_evidence() {
        let t = Instant::now();
        let mut r = serial(t);
        show(&mut r, t + Duration::from_secs(3));
        assert!(r
            .hook(
                HookEvent::PostToolUse,
                Some("tool-1"),
                t + Duration::from_millis(3301)
            )
            .is_empty());
        let at = t + Duration::from_secs(4);
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
    }

    #[test]
    fn conflicting_status_with_same_rpc_start_cannot_manufacture_resolution() {
        let t = Instant::now();
        let mut r = serial(t);
        let at = t + Duration::from_secs(3);
        show(&mut r, at);
        assert!(r
            .observe(
                NativeStatus::Active,
                true,
                at,
                at + Duration::from_millis(100)
            )
            .is_empty());
        let later = t + Duration::from_secs(4);
        assert!(r
            .observe(NativeStatus::Active, true, later, later)
            .is_empty());
    }

    #[test]
    fn repeated_anonymous_permission_never_grants_serial_clear_rights() {
        let t = Instant::now();
        let mut r = serial(t);
        assert!(r
            .hook(
                HookEvent::PermissionRequest,
                None,
                t + Duration::from_millis(3)
            )
            .is_empty());
        show(&mut r, t + Duration::from_secs(3));
        let at = t + Duration::from_millis(3100);
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r
            .hook(HookEvent::PostToolUse, Some("tool-1"), at)
            .is_empty());
    }

    #[test]
    fn explicit_same_invocation_duplicate_does_not_revoke_serial_clear_rights() {
        let t = Instant::now();
        let mut r = serial(t);
        assert!(r
            .hook(
                HookEvent::PermissionRequest,
                Some("tool-1"),
                t + Duration::from_millis(3)
            )
            .is_empty());
        let shown = show(&mut r, t + Duration::from_secs(3));
        let at = t + Duration::from_millis(3100);
        assert_clear(r.observe(NativeStatus::Active, true, at, at), shown);
    }

    #[test]
    fn identity_exhaustion_disables_assistance_without_reusing_or_clearing_wait() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        r.next_candidate = u64::MAX - 1;
        assert!(r.assistance_available());
        r.observe(NativeStatus::Active, true, t, t);
        r.hook(HookEvent::PreToolUse, Some("last-id"), t);
        r.hook(HookEvent::PermissionRequest, Some("last-id"), t);
        show(&mut r, t + Duration::from_secs(2));
        let at = t + Duration::from_millis(2100);
        r.hook(HookEvent::PreToolUse, Some("exhausted"), at);
        assert!(r
            .hook(HookEvent::PermissionRequest, Some("exhausted"), at)
            .is_empty());
        assert!(!r.assistance_available());
        assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
        assert!(r
            .hook(HookEvent::PostToolUse, Some("last-id"), at)
            .is_empty());
        assert!(r.hook(HookEvent::Stop, None, at).is_empty());
        assert!(!r.assistance_available());
        r.invalidate(Fence {
            runtime_generation: 1,
            ..Fence::default()
        });
        assert!(!r.assistance_available());
    }

    #[test]
    fn anonymous_permission_after_retirement_starts_conservative_wait() {
        for native_resolution in [false, true] {
            let t = Instant::now();
            let mut r = serial(t);
            if native_resolution {
                let shown = show(&mut r, t + Duration::from_secs(3));
                let at = t + Duration::from_millis(3100);
                assert_clear(r.observe(NativeStatus::Active, true, at, at), shown);
            } else {
                assert!(r
                    .hook(
                        HookEvent::PostToolUse,
                        Some("tool-1"),
                        t + Duration::from_millis(100)
                    )
                    .is_empty());
            }
            let at = t + Duration::from_secs(4);
            assert!(r.hook(HookEvent::PermissionRequest, None, at).is_empty());
            assert!(r
                .hook(
                    HookEvent::PermissionRequest,
                    None,
                    at + Duration::from_secs(1)
                )
                .is_empty());
            assert!(matches!(
                r.tick(at + Duration::from_secs(2)).as_slice(),
                [Effect::ShowWait { .. }]
            ));
            let later = at + Duration::from_secs(3);
            assert!(r
                .observe(NativeStatus::Active, true, later, later)
                .is_empty());
            assert!(r
                .hook(HookEvent::PostToolUse, Some("tool-1"), later)
                .is_empty());
        }
    }

    #[test]
    fn native_status_without_validated_permission_never_creates_effects() {
        let t = Instant::now();
        let mut r = ApprovalReducer::new(Fence::default());
        for status in [
            NativeStatus::ApprovalPending,
            NativeStatus::UserInputPending,
            NativeStatus::Unavailable,
            NativeStatus::Active,
        ] {
            assert!(r.observe(status, true, t, t).is_empty());
            assert!(r.tick(t + Duration::from_secs(10)).is_empty());
        }
    }

    #[test]
    fn stop_prompt_and_fence_invalidation_discard_old_effects_without_native_clear() {
        for boundary in 0..6 {
            let t = Instant::now();
            let mut r = serial(t);
            let shown = show(&mut r, t + Duration::from_secs(3));
            assert!(r.accepts(&shown));
            let mut fence = Fence::default();
            match boundary {
                0 => assert!(r.hook(HookEvent::Stop, None, t).is_empty()),
                1 => assert!(r.hook(HookEvent::UserPromptSubmit, None, t).is_empty()),
                2 => {
                    fence.runtime_generation = 1;
                    r.invalidate(fence);
                }
                3 => {
                    fence.identity_revision = 1;
                    r.invalidate(fence);
                }
                4 => {
                    fence.turn_revision = 1;
                    r.invalidate(fence);
                }
                _ => {
                    fence.input_revision = 1;
                    r.invalidate(fence);
                }
            }
            assert!(!r.accepts(&shown));
            let at = t + Duration::from_secs(4);
            assert!(r.observe(NativeStatus::Active, true, at, at).is_empty());
            assert!(r.tick(at).is_empty());
        }
    }

    #[test]
    fn candidate_identity_is_not_reused_across_invalidation_and_fence_is_preserved() {
        let t = Instant::now();
        let mut r = serial(t);
        let first = show(&mut r, t + Duration::from_secs(3));
        let fence = Fence {
            runtime_generation: 2,
            identity_revision: 3,
            turn_revision: 4,
            input_revision: 5,
        };
        r.invalidate(fence);
        let at = t + Duration::from_secs(4);
        r.observe(NativeStatus::Active, true, at, at);
        r.hook(HookEvent::PreToolUse, Some("tool-1"), at);
        r.hook(HookEvent::PermissionRequest, None, at);
        let second = show(&mut r, at + Duration::from_secs(2));
        let (
            Effect::ShowWait { candidate: a, .. },
            Effect::ShowWait {
                candidate: b,
                fence: actual,
            },
        ) = (first, second)
        else {
            panic!("expected shows")
        };
        assert_ne!(a, b);
        assert_eq!(actual, fence);
        assert!(!r.accepts(&first));
        assert!(r.accepts(&second));
    }
}
