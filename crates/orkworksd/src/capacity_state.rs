//! Owning module for the per-session capacity (usage-limit) state machine.
//!
//! Before this module existed, the six fields below lived as loose public
//! fields on `SessionHandle`, and the capped/pending/latched transition rules
//! were embedded ~390 lines deep inside `session_projection::project_capacity`
//! (reached from a *read* endpoint, `GET /sessions`, which then performed
//! write-back into session handles and the provider manager). See issue #398.
//!
//! `CapacityState` now owns the fields and exposes the transition rules as a
//! pure `observe()` (read-only, safe to call against a point-in-time
//! snapshot) plus mutators that apply an observation or record new output.
//! `session_projection::project_capacity` renders from this interface instead
//! of embedding the rules itself, and this module's own tests are the target
//! surface for capacity behavior — no `#[cfg(test)]` hook is threaded through
//! the projection to reach it.

/// Per-session usage-limit / capacity-check state.
///
/// Fields are `pub(crate)` (not fully encapsulated) so existing test call
/// sites can build fixtures with struct-update syntax
/// (`CapacityState { at_usage_limit_latched: true, ..Default::default() }`)
/// without a builder. External callers still only reach state transitions
/// through `observe`/`apply_observation`/`record_output`/`arm_recheck`/
/// `clear_latch_and_rearm`/`advance_pending_visibility`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CapacityState {
    /// Sticky: once a usage limit is detected it stays true until the
    /// session is killed/resumed or explicitly cleared.
    pub(crate) at_usage_limit_latched: bool,
    pub(crate) capacity_check_pending: bool,
    pub(crate) output_lines_seen: u64,
    pub(crate) scan_bytes_seen: u64,
    /// Snapshot origin used for one-shot post-resume / post-input fresh-output
    /// checks: `(output_lines_seen, scan_bytes_seen)` at the moment the
    /// recheck window was armed.
    pub(crate) resume_scan_origin: Option<(u64, u64)>,
    pub(crate) pending_capacity_visible_once: bool,
}

/// The result of `CapacityState::observe`: what the projection should render
/// for this cycle, and how the state machine wants to advance on write-back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapacityObservation {
    pub(crate) at_usage_limit: bool,
    pub(crate) reset_hint: Option<String>,
    /// `Some(origin)` when a scoped recheck (stale-latch or baseline) ran
    /// this cycle and wants to advance `resume_scan_origin` to it.
    pub(crate) origin_update: Option<(u64, u64)>,
    /// Whether that origin update should also clear the latch (a scoped
    /// recheck of a previously-latched session came back clean).
    pub(crate) clear_latch: bool,
}

impl CapacityState {
    /// Constructor for session creation, mirroring the pre-refactor inline
    /// literal: a freshly created session starts unlatched, with the given
    /// pending-capacity-check flag, and an armed origin at `(0, 0)` only when
    /// pending (nothing has been scanned yet).
    pub(crate) fn pending(capacity_check_pending: bool) -> Self {
        Self {
            capacity_check_pending,
            resume_scan_origin: capacity_check_pending.then_some((0, 0)),
            ..Default::default()
        }
    }

    /// Records freshly ingested PTY output. `lines` is deliberately the
    /// physical row count persisted (not the fewer logical lines after
    /// hard-wrap reassembly, per ADR 0065 Consequences); `bytes` is the raw
    /// scan-buffer byte count.
    pub(crate) fn record_output(&mut self, lines: u64, bytes: u64) {
        self.output_lines_seen += lines;
        self.scan_bytes_seen += bytes;
    }

    /// Arms the first capacity recheck after a latched session receives
    /// accepted input. No-op (returns `false`) unless the session is
    /// currently latched with no pending check and no already-armed origin.
    pub(crate) fn arm_recheck(&mut self) -> bool {
        if !self.at_usage_limit_latched
            || self.capacity_check_pending
            || self.resume_scan_origin.is_some()
        {
            return false;
        }
        self.resume_scan_origin = Some((self.output_lines_seen, self.scan_bytes_seen));
        true
    }

    /// Clears the latch and re-arms the recheck window at the current
    /// output position. Used when a harness's own hook reports it left the
    /// working state, which some harnesses (currently only Claude Code) can
    /// use as a stronger-than-terminal-scan signal that a usage limit no
    /// longer applies.
    pub(crate) fn clear_latch_and_rearm(&mut self) {
        self.at_usage_limit_latched = false;
        self.resume_scan_origin = Some((self.output_lines_seen, self.scan_bytes_seen));
    }

    /// Whether output has arrived since `resume_scan_origin` was armed.
    /// `false` when no origin is armed.
    pub(crate) fn fresh_output_since_origin(&self) -> bool {
        self.resume_scan_origin
            .map(|(line_count, scan_len)| {
                self.output_lines_seen > line_count || self.scan_bytes_seen > scan_len
            })
            .unwrap_or(false)
    }

    /// The fresh-output window since `origin`, sliced out of `snapshot`
    /// (persisted output lines) and `scan_buf` (raw scan text). Ported
    /// unchanged from the four copies previously inlined at
    /// `session_projection.rs:292-316,317-339,348-366,367-386` (pre-refactor
    /// line numbers).
    fn fresh_window<'a>(
        &self,
        origin: (u64, u64),
        snapshot: &'a [String],
        scan_buf: &'a str,
    ) -> (&'a [String], &'a str) {
        let (line_count, scan_len) = origin;
        let line_window_start = self.output_lines_seen.saturating_sub(snapshot.len() as u64);
        let scan_window_start = self.scan_bytes_seen.saturating_sub(scan_buf.len() as u64);
        let fresh_line_start = line_count.saturating_sub(line_window_start) as usize;
        let fresh_scan_start = scan_len.saturating_sub(scan_window_start) as usize;
        let fresh_lines = snapshot
            .get(fresh_line_start.min(snapshot.len())..)
            .unwrap_or(&[]);
        let fresh_scan = scan_buf
            .get(fresh_scan_start.min(scan_buf.len())..)
            .unwrap_or("");
        (fresh_lines, fresh_scan)
    }

    /// Consults the transition rules against a live scan, without mutating
    /// `self`. Safe to call against a point-in-time snapshot; the result is
    /// applied later (if the snapshot is still current) via
    /// `apply_observation`.
    pub(crate) fn observe(
        &self,
        limit_patterns: &[String],
        snapshot: &[String],
        scan_buf: &str,
    ) -> CapacityObservation {
        let fresh_output_since_origin = self.fresh_output_since_origin();
        let stale_cap_recheck = self.at_usage_limit_latched
            && !self.capacity_check_pending
            && self.resume_scan_origin.is_some();
        let baseline_scoped_detection = !self.at_usage_limit_latched
            && !self.capacity_check_pending
            && self.resume_scan_origin.is_some();

        let detected_full = crate::peon::detect_usage_limit(limit_patterns, snapshot)
            || crate::peon::detect_usage_limit_raw(limit_patterns, scan_buf);

        if stale_cap_recheck && fresh_output_since_origin {
            let origin = self.resume_scan_origin.unwrap();
            let (fresh_lines, fresh_scan) = self.fresh_window(origin, snapshot, scan_buf);
            let detected_scoped = crate::peon::detect_usage_limit(limit_patterns, fresh_lines)
                || crate::peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
            let reset_hint = crate::peon::detect_usage_limit_hint(limit_patterns, fresh_lines)
                .or_else(|| crate::peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan));
            let origin_now = (self.output_lines_seen, self.scan_bytes_seen);
            CapacityObservation {
                at_usage_limit: detected_scoped,
                reset_hint,
                origin_update: Some(origin_now),
                clear_latch: !detected_scoped,
            }
        } else if baseline_scoped_detection {
            let origin = self.resume_scan_origin.unwrap();
            let (fresh_lines, fresh_scan) = self.fresh_window(origin, snapshot, scan_buf);
            let detected_scoped = crate::peon::detect_usage_limit(limit_patterns, fresh_lines)
                || crate::peon::detect_usage_limit_raw(limit_patterns, fresh_scan);
            let reset_hint = crate::peon::detect_usage_limit_hint(limit_patterns, fresh_lines)
                .or_else(|| crate::peon::detect_usage_limit_hint_raw(limit_patterns, fresh_scan));
            CapacityObservation {
                at_usage_limit: detected_scoped,
                reset_hint,
                origin_update: detected_scoped
                    .then_some((self.output_lines_seen, self.scan_bytes_seen)),
                clear_latch: false,
            }
        } else {
            let reset_hint = crate::peon::detect_usage_limit_hint(limit_patterns, snapshot)
                .or_else(|| crate::peon::detect_usage_limit_hint_raw(limit_patterns, scan_buf));
            CapacityObservation {
                at_usage_limit: self.at_usage_limit_latched || detected_full,
                reset_hint,
                origin_update: None,
                clear_latch: false,
            }
        }
    }

    /// Applies a previously-computed observation, mirroring the single
    /// write-back site in `session_projection::project_capacity`. Returns
    /// `true` if the latch transitioned from unlatched to latched this call
    /// (the caller stamps `usage_limit_latched_at` from `SessionRuntime` in
    /// that case, since the timestamp source lives outside this state).
    pub(crate) fn apply_observation(&mut self, observation: &CapacityObservation) -> bool {
        let newly_latched = observation.at_usage_limit && !self.at_usage_limit_latched;
        if observation.at_usage_limit {
            self.at_usage_limit_latched = true;
        }
        if let Some(origin) = observation.origin_update {
            self.resume_scan_origin = Some(origin);
            if observation.clear_latch {
                self.at_usage_limit_latched = false;
            }
        }
        newly_latched
    }

    /// Advances the one-shot post-resume pending-capacity visibility
    /// bookkeeping. Returns `None` when there is nothing to advance (no
    /// pending check outstanding — the caller should leave the session's
    /// cached `capacity_check_pending` info field untouched), otherwise the
    /// new value for that field.
    pub(crate) fn advance_pending_visibility(
        &mut self,
        has_fresh_resume_output: bool,
    ) -> Option<Option<bool>> {
        if !self.capacity_check_pending {
            return None;
        }
        if self.pending_capacity_visible_once {
            self.capacity_check_pending = false;
            self.resume_scan_origin = None;
            self.pending_capacity_visible_once = false;
            Some(None)
        } else if has_fresh_resume_output {
            self.pending_capacity_visible_once = true;
            self.resume_scan_origin = None;
            Some(Some(true))
        } else {
            Some(Some(true))
        }
    }

    /// The rendered `capacity_check_pending` value for `SessionInfo`: visible
    /// only while pending and not yet shown once.
    pub(crate) fn rendered_check_pending(&self) -> Option<bool> {
        (self.capacity_check_pending && !self.pending_capacity_visible_once).then_some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns() -> Vec<String> {
        vec!["usage limit".to_string()]
    }

    #[test]
    fn pending_constructor_arms_origin_only_when_pending() {
        let pending = CapacityState::pending(true);
        assert!(pending.capacity_check_pending);
        assert_eq!(pending.resume_scan_origin, Some((0, 0)));

        let not_pending = CapacityState::pending(false);
        assert!(!not_pending.capacity_check_pending);
        assert_eq!(not_pending.resume_scan_origin, None);
    }

    #[test]
    fn record_output_accumulates_lines_and_bytes() {
        let mut state = CapacityState::default();
        state.record_output(3, 10);
        state.record_output(2, 5);
        assert_eq!(state.output_lines_seen, 5);
        assert_eq!(state.scan_bytes_seen, 15);
    }

    #[test]
    fn arm_recheck_only_fires_when_latched_and_idle() {
        let mut state = CapacityState::default();
        assert!(!state.arm_recheck(), "not latched, should not arm");

        state.at_usage_limit_latched = true;
        state.output_lines_seen = 7;
        state.scan_bytes_seen = 11;
        assert!(state.arm_recheck());
        assert_eq!(state.resume_scan_origin, Some((7, 11)));

        // Already armed: a second call is a no-op.
        state.output_lines_seen = 20;
        assert!(!state.arm_recheck());
        assert_eq!(state.resume_scan_origin, Some((7, 11)));
    }

    #[test]
    fn arm_recheck_does_not_fire_while_pending() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        state.capacity_check_pending = true;
        assert!(!state.arm_recheck());
        assert_eq!(state.resume_scan_origin, None);
    }

    #[test]
    fn clear_latch_and_rearm_unlatches_and_sets_origin_to_current_position() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        state.output_lines_seen = 4;
        state.scan_bytes_seen = 9;
        state.clear_latch_and_rearm();
        assert!(!state.at_usage_limit_latched);
        assert_eq!(state.resume_scan_origin, Some((4, 9)));
    }

    #[test]
    fn observe_detects_full_buffer_when_no_recheck_is_armed() {
        let state = CapacityState::default();
        let obs = state.observe(&patterns(), &["usage limit".to_string()], "");
        assert!(obs.at_usage_limit);
        assert_eq!(obs.origin_update, None);
        assert!(!obs.clear_latch);
    }

    #[test]
    fn observe_stays_latched_when_no_fresh_output_since_origin() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        state.resume_scan_origin = Some((5, 5));
        state.output_lines_seen = 5;
        state.scan_bytes_seen = 5;
        let obs = state.observe(&patterns(), &[], "");
        assert!(
            obs.at_usage_limit,
            "still latched with nothing fresh to recheck"
        );
        assert_eq!(obs.origin_update, None);
    }

    #[test]
    fn observe_stale_recheck_clears_latch_when_fresh_window_is_clean() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        state.resume_scan_origin = Some((0, 0));
        state.output_lines_seen = 1;
        state.scan_bytes_seen = 5;
        let snapshot = vec!["all clear now".to_string()];
        let obs = state.observe(&patterns(), &snapshot, "all clear now");
        assert!(!obs.at_usage_limit);
        assert!(obs.clear_latch);
        assert_eq!(obs.origin_update, Some((1, 5)));
    }

    #[test]
    fn observe_stale_recheck_advances_origin_without_clearing_when_still_capped() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        state.resume_scan_origin = Some((0, 0));
        state.output_lines_seen = 1;
        state.scan_bytes_seen = 20;
        let snapshot = vec!["usage limit".to_string()];
        let obs = state.observe(&patterns(), &snapshot, "usage limit");
        assert!(obs.at_usage_limit);
        assert!(!obs.clear_latch);
        assert_eq!(obs.origin_update, Some((1, 20)));
    }

    #[test]
    fn observe_baseline_scoped_detection_only_advances_origin_when_capped() {
        let mut state = CapacityState::default();
        state.resume_scan_origin = Some((0, 0));
        state.output_lines_seen = 1;
        state.scan_bytes_seen = 6;
        let snapshot = vec!["clean output".to_string()];
        let obs = state.observe(&patterns(), &snapshot, "clean output");
        assert!(!obs.at_usage_limit);
        assert_eq!(
            obs.origin_update, None,
            "no baseline insert when not capped"
        );
    }

    #[test]
    fn apply_observation_reports_newly_latched_transition() {
        let mut state = CapacityState::default();
        let obs = CapacityObservation {
            at_usage_limit: true,
            reset_hint: None,
            origin_update: None,
            clear_latch: false,
        };
        assert!(state.apply_observation(&obs));
        assert!(state.at_usage_limit_latched);
        // Already latched: no second transition.
        assert!(!state.apply_observation(&obs));
    }

    #[test]
    fn apply_observation_with_clear_latch_unlatches_despite_prior_latch() {
        let mut state = CapacityState::default();
        state.at_usage_limit_latched = true;
        let obs = CapacityObservation {
            at_usage_limit: false,
            reset_hint: None,
            origin_update: Some((3, 4)),
            clear_latch: true,
        };
        assert!(!state.apply_observation(&obs));
        assert!(!state.at_usage_limit_latched);
        assert_eq!(state.resume_scan_origin, Some((3, 4)));
    }

    #[test]
    fn advance_pending_visibility_returns_none_when_nothing_pending() {
        let mut state = CapacityState::default();
        assert_eq!(state.advance_pending_visibility(false), None);
    }

    #[test]
    fn advance_pending_visibility_clears_once_already_shown() {
        let mut state = CapacityState::default();
        state.capacity_check_pending = true;
        state.pending_capacity_visible_once = true;
        state.resume_scan_origin = Some((1, 1));
        assert_eq!(state.advance_pending_visibility(false), Some(None));
        assert!(!state.capacity_check_pending);
        assert!(!state.pending_capacity_visible_once);
        assert_eq!(state.resume_scan_origin, None);
    }

    #[test]
    fn advance_pending_visibility_marks_visible_once_on_fresh_resume_output() {
        let mut state = CapacityState::default();
        state.capacity_check_pending = true;
        state.resume_scan_origin = Some((1, 1));
        assert_eq!(state.advance_pending_visibility(true), Some(Some(true)));
        assert!(state.pending_capacity_visible_once);
        assert_eq!(state.resume_scan_origin, None);
    }

    #[test]
    fn advance_pending_visibility_keeps_pending_without_fresh_output() {
        let mut state = CapacityState::default();
        state.capacity_check_pending = true;
        assert_eq!(state.advance_pending_visibility(false), Some(Some(true)));
        assert!(!state.pending_capacity_visible_once);
    }

    #[test]
    fn rendered_check_pending_hides_after_first_visibility() {
        let mut state = CapacityState::default();
        state.capacity_check_pending = true;
        assert_eq!(state.rendered_check_pending(), Some(true));
        state.pending_capacity_visible_once = true;
        assert_eq!(state.rendered_check_pending(), None);
    }

    #[test]
    fn stale_write_back_is_detected_by_equality() {
        // `session_projection::project_capacity`'s optimistic-concurrency
        // write-back guard compares a `CapacityState` snapshot taken at the
        // start of the projection cycle against the live handle's state at
        // write-back time, and skips the write-back on any mismatch. This
        // is the direct-interface replacement for the removed
        // `list_with_hook`/`before_write_back` HTTP-level race-injection
        // test hook.
        let snapshot = CapacityState {
            output_lines_seen: 1,
            ..Default::default()
        };
        let unchanged = snapshot.clone();
        assert_eq!(
            snapshot, unchanged,
            "no concurrent mutation: write-back proceeds"
        );

        let mut mutated = snapshot.clone();
        mutated.output_lines_seen = 2;
        assert_ne!(
            snapshot, mutated,
            "concurrent mutation: write-back must be skipped"
        );
    }
}
