//! In-memory native effect authority for the source-only SQL application owner.
use super::*;
use crate::metadata::internal_mutations::InternalMutation;
use crate::session_store::AttentionProof;
use std::sync::atomic::{AtomicBool, Ordering};

/// Mint alongside reducer emission under the issuing tracker/attention guards,
/// before releasing them or reading SQL. Never reconstruct from a raw queued effect.
#[derive(Clone)]
pub(crate) struct IssuedEffect {
    pub(super) effect: Effect,
    instance: Arc<()>,
    hook_revision: u64,
    attention: AttentionWriteToken,
    observation_start: Option<Instant>,
}
impl NativeApprovalState {
    pub(crate) fn observe_sql(
        &mut self,
        owner: &super::super::observed_status::AttentionOwner,
        status: super::super::codex_approval::NativeStatus,
        complete_singleton_root: bool,
        started_at: Instant,
        now: Instant,
    ) -> Vec<IssuedEffect> {
        let effects = self
            .reducer
            .observe(status, complete_singleton_root, started_at, now);
        self.wrap_sql_effects(effects, owner)
    }
    pub(crate) fn hook_sql(
        &mut self,
        owner: &super::super::observed_status::AttentionOwner,
        event: HookEvent,
        tool_id: Option<&str>,
        now: Instant,
    ) -> Vec<IssuedEffect> {
        let effects = self.reducer.hook(event, tool_id, now);
        self.wrap_sql_effects(effects, owner)
    }
    pub(crate) fn tick_sql(
        &mut self,
        owner: &super::super::observed_status::AttentionOwner,
        now: Instant,
    ) -> Vec<IssuedEffect> {
        let effects = self.reducer.tick(now);
        self.wrap_sql_effects(effects, owner)
    }
    fn wrap_sql_effects(
        &self,
        effects: Vec<Effect>,
        owner: &super::super::observed_status::AttentionOwner,
    ) -> Vec<IssuedEffect> {
        effects
            .into_iter()
            .filter_map(|effect| self.wrap_sql_effect(effect, owner))
            .collect()
    }
    fn wrap_sql_effect(
        &self,
        effect: Effect,
        owner: &super::super::observed_status::AttentionOwner,
    ) -> Option<IssuedEffect> {
        if !self.reducer.accepts(&effect) {
            return None;
        }
        Some(IssuedEffect {
            effect,
            instance: self.instance.clone(),
            hook_revision: self.hook_revision?,
            attention: owner.snapshot()?,
            observation_start: match effect {
                Effect::ClearWait { .. } => Some(self.reducer.clear_observation_start()?),
                Effect::ShowWait { .. } => None,
            },
        })
    }
}

pub(crate) struct NativeCapture {
    pub(crate) created: String,
    effect: Effect,
    instance: Arc<()>,
    runtime: super::super::session_runtime::RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<super::super::session_runtime::RuntimeCommand>,
    fence: Fence,
    hook_revision: u64,
    root: String,
    attention: AttentionWriteToken,
    committed_input_sequence: u64,
    observation_start: Option<Instant>,
    cancelled: Arc<AtomicBool>,
    pub(crate) proof: Option<AttentionProof>,
}
impl NativeCapture {
    pub(crate) fn capture(
        h: &SessionHandle,
        meta: &crate::metadata::SessionMetadata,
        issued: IssuedEffect,
        cancelled: Arc<AtomicBool>,
        coordinated: bool,
    ) -> Option<Self> {
        let effect = issued.effect;
        let tracker = h.runtime.native_approval.as_ref()?;
        if !Arc::ptr_eq(&issued.instance, &tracker.instance)
            || tracker.hook_revision != Some(issued.hook_revision)
            || !h.runtime.attention_owner.owns(&issued.attention)
            || (matches!(effect, Effect::ClearWait { .. })
                && tracker.reducer.clear_observation_start() != issued.observation_start)
        {
            return None;
        }
        let root = tracker.root.clone()?;
        if meta.created_at != h.info.created_at || accepted_root(meta) != Some(root.as_str()) {
            return None;
        }
        let clear = matches!(effect, Effect::ClearWait { .. });
        if clear && !coordinated {
            return None;
        }
        let captured = Self {
            created: h.info.created_at.clone(),
            effect,
            instance: issued.instance,
            runtime: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            fence: tracker.fence,
            hook_revision: tracker.hook_revision?,
            root,
            attention: issued.attention,
            committed_input_sequence: h.runtime.committed_input_sequence,
            observation_start: issued.observation_start,
            cancelled,
            proof: if clear {
                Some(tracker.sql_ownership.as_ref()?.1.clone())
            } else {
                None
            },
        };
        captured.is_current(h).then_some(captured)
    }
    pub(crate) fn intent(&self) -> InternalMutation {
        match self.observation_start {
            Some(observation_started_at) => InternalMutation::NativeClearWait {
                timestamp: crate::workspace_runtime::iso_now(),
                observation_started_at,
            },
            None => InternalMutation::NativeShowWait {
                timestamp: crate::workspace_runtime::iso_now(),
            },
        }
    }
    /// Called under short application guards; never waits or performs I/O.
    pub(crate) fn is_current(&self, h: &SessionHandle) -> bool {
        let Some(tracker) = h.runtime.native_approval.as_ref() else {
            return false;
        };
        let common = Arc::ptr_eq(&self.instance, &tracker.instance)
            && h.runtime.matches_identity(&self.runtime)
            && h.runtime.run_generation() == self.fence.runtime_generation
            && h.runtime.control_tx.same_channel(&self.control)
            && h.info.created_at == self.created
            && h.info.lifecycle == "alive"
            && h.info.lifecycle_phase == "active"
            && h.active_work_hook
            && h.runtime.input_generation == self.fence.input_revision
            && h.runtime.committed_input_sequence == self.committed_input_sequence
            && tracker.committed_input_sequence == self.committed_input_sequence
            && tracker.fence == self.fence
            && tracker.hook_revision == Some(self.hook_revision)
            && tracker.root.as_deref() == Some(self.root.as_str())
            && h.info.harness.as_deref() == Some("codex")
            && h.info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref())
                == Some(self.root.as_str())
            && h.runtime.attention_owner.owns(&self.attention)
            && tracker.reducer.accepts(&self.effect)
            && tracker
                .live_revision
                .as_ref()
                .is_none_or(|live| h.runtime.attention_owner.owns(live))
            && h.info
                .provenance
                .attention_source
                .as_deref()
                .or(h.info.metadata_source.as_deref())
                != Some("user")
            && h.info.needs_user_input != Some(true)
            && h.info.detected_question.is_none()
            && h.info.suggested_options.is_none();
        let tuple = match self.effect {
            Effect::ShowWait { .. } => h.info.attention.as_deref() != Some("needs_you"),
            Effect::ClearWait { .. } => {
                tracker.reducer.clear_is_fresh(Instant::now())
                    && tracker.reducer.clear_observation_start() == self.observation_start
                    && h.info.needs_user_input.is_none()
                    && h.info.observed_status.as_deref() == Some("waiting_for_input")
                    && h.info.attention.as_deref() == Some("needs_you")
                    && h.info.provenance.attention_source.as_deref() == Some("agent")
                    && h.info.provenance.attention_origin.as_deref() == Some("native_codex")
                    && h.info.provenance.attention_confidence == Some(1.0)
                    && tracker
                        .sql_ownership
                        .as_ref()
                        .is_some_and(|(live, _)| h.runtime.attention_owner.owns(live))
            }
        };
        common && tuple && !self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn publish(&self, h: &mut SessionHandle, proof: AttentionProof) {
        let status = match self.effect {
            Effect::ShowWait { .. } => "waiting_for_input",
            Effect::ClearWait { .. } => "working",
        };
        apply_live_attention_fields(
            &mut h.info,
            &mut h.runtime.attention_owner,
            status,
            None,
            "agent",
            1.0,
        );
        h.info.provenance.attention_origin = Some("native_codex".into());
        let tracker = h
            .runtime
            .native_approval
            .as_mut()
            .expect("validated native tracker");
        tracker.ownership = None;
        match self.effect {
            Effect::ShowWait { .. } => {
                tracker.sql_ownership = h
                    .runtime
                    .attention_owner
                    .snapshot()
                    .map(|live| (live, proof));
                tracker.reducer.acknowledge_show(&self.effect);
            }
            Effect::ClearWait { .. } => {
                tracker.sql_ownership = None;
                tracker.reducer.acknowledge_clear(&self.effect);
            }
        }
        tracker.live_revision = h.runtime.attention_owner.snapshot();
    }
}

/// Captured before a SQL read or RPC, so neither can adopt a replacement owner.
pub(crate) struct ObservationContext {
    captured_at: Instant,
    id: String,
    created: String,
    instance: Arc<()>,
    runtime: super::super::session_runtime::RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<super::super::session_runtime::RuntimeCommand>,
    fence: Fence,
    hook_revision: u64,
    root: Option<String>,
    live_root: Option<String>,
    attention: AttentionWriteToken,
    input_generation: u64,
    committed_input_sequence: u64,
    active_work_hook: bool,
    authority: Option<super::super::prompt_authority::AuthoritySnapshot>,
    cancelled: Arc<AtomicBool>,
}
impl ObservationContext {
    pub(crate) fn capture(
        h: &SessionHandle,
        generation: u64,
        cancelled: Arc<AtomicBool>,
        authority: Option<super::super::prompt_authority::AuthoritySnapshot>,
    ) -> Option<Self> {
        let tracker = h.runtime.native_approval.as_ref()?;
        if h.runtime.run_generation() != generation
            || tracker.fence.runtime_generation != generation
            || h.info.lifecycle != "alive"
            || h.info.lifecycle_phase != "active"
            || h.info.harness_id.as_deref().or(h.info.harness.as_deref()) != Some("codex")
            || cancelled.load(Ordering::Acquire)
        {
            return None;
        }
        Some(Self {
            captured_at: Instant::now(),
            id: h.info.id.clone(),
            created: h.info.created_at.clone(),
            instance: tracker.instance.clone(),
            runtime: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            fence: tracker.fence,
            hook_revision: tracker.hook_revision?,
            root: tracker.root.clone(),
            live_root: h
                .info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.clone()),
            attention: h.runtime.attention_owner.snapshot()?,
            input_generation: h.runtime.input_generation,
            committed_input_sequence: h.runtime.committed_input_sequence,
            active_work_hook: h.active_work_hook,
            authority,
            cancelled,
        })
    }
    pub(crate) fn matches(
        &self,
        h: &SessionHandle,
        authority: &Option<super::super::prompt_authority::AuthoritySnapshot>,
    ) -> bool {
        let Some(tracker) = h.runtime.native_approval.as_ref() else {
            return false;
        };
        h.info.id == self.id
            && h.info.created_at == self.created
            && h.info.lifecycle == "alive"
            && h.info.lifecycle_phase == "active"
            && h.info.harness_id.as_deref().or(h.info.harness.as_deref()) == Some("codex")
            && h.runtime.matches_identity(&self.runtime)
            && h.runtime.control_tx.same_channel(&self.control)
            && h.runtime.input_generation == self.input_generation
            && h.runtime.committed_input_sequence == self.committed_input_sequence
            && h.active_work_hook == self.active_work_hook
            && Arc::ptr_eq(&tracker.instance, &self.instance)
            && tracker.fence == self.fence
            && tracker.hook_revision == Some(self.hook_revision)
            && tracker.root == self.root
            && h.info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_ref())
                == self.live_root.as_ref()
            && h.runtime.attention_owner.owns(&self.attention)
            && &self.authority == authority
            && !self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn root(&self) -> Option<&str> {
        self.root.as_deref()
    }
    pub(crate) fn cancellation(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}

/// Retire only the still-current rejected tick; detached SQL proof grants no authority.
pub(crate) fn reject_sql_observation_effect(
    h: &mut SessionHandle,
    origin: &ObservationContext,
    issued: &IssuedEffect,
    authority: &Option<super::super::prompt_authority::AuthoritySnapshot>,
) -> bool {
    if !origin.matches(h, authority) {
        return false;
    }
    let tracker = h
        .runtime
        .native_approval
        .as_mut()
        .expect("validated tracker");
    if !Arc::ptr_eq(&issued.instance, &tracker.instance)
        || tracker.hook_revision != Some(issued.hook_revision)
        || !h.runtime.attention_owner.owns(&issued.attention)
        || !tracker.reducer.accepts(&issued.effect)
    {
        return false;
    }
    reject(tracker);
    true
}

pub(crate) fn begin_sql_observation(
    h: &mut SessionHandle,
    meta: &crate::metadata::SessionMetadata,
    origin: &ObservationContext,
    authority: Option<super::super::prompt_authority::AuthoritySnapshot>,
) -> Option<(ObservationContext, Vec<IssuedEffect>)> {
    if !origin.matches(h, &authority)
        || meta.id != origin.id
        || meta.created_at != origin.created
        || meta.lifecycle != "alive"
        || meta.lifecycle_phase != "active"
    {
        return None;
    }
    let mut tracker = h.runtime.native_approval.take()?;
    let effects = (|| {
        tracker.sync(h, meta);
        let root = tracker.root.as_deref()?;
        if h.info
            .resume
            .as_ref()
            .and_then(|r| r.harness_session_id.as_deref())
            != Some(root)
        {
            return None;
        }
        Some(tracker.tick_sql(&h.runtime.attention_owner, Instant::now()))
    })();
    h.runtime.native_approval = Some(tracker);
    let effects = effects?;
    let context = ObservationContext::capture(
        h,
        origin.runtime.run_generation,
        origin.cancelled.clone(),
        authority,
    )?;
    Some((context, effects))
}

pub(crate) fn finish_sql_observation(
    h: &mut SessionHandle,
    meta: &crate::metadata::SessionMetadata,
    origin: &ObservationContext,
    authority: Option<super::super::prompt_authority::AuthoritySnapshot>,
    result: Result<
        super::super::codex_native::NativeObservation,
        super::super::codex_native::NativeError,
    >,
) -> Option<Vec<IssuedEffect>> {
    if result
        .as_ref()
        .is_ok_and(|observation| observation.started_at < origin.captured_at)
    {
        return None;
    }
    if !origin.matches(h, &authority)
        || meta.id != origin.id
        || meta.created_at != origin.created
        || meta.lifecycle != "alive"
        || meta.lifecycle_phase != "active"
    {
        return None;
    }
    let mut tracker = h.runtime.native_approval.take()?;
    tracker.sync(h, meta);
    h.runtime.native_approval = Some(tracker);
    // Sync can discover a durable identity/input/source transition. The RPC
    // belongs to the old fence and cannot be fed into the updated reducer.
    if !origin.matches(h, &authority) {
        return None;
    }
    let tracker = h.runtime.native_approval.as_mut()?;
    #[cfg(test)]
    if let Some(sink) = &tracker.diagnostic {
        let mut metrics = sink.lock().unwrap();
        metrics.observations += 1;
        match &result {
            Ok(observation) => {
                metrics.last_status = Some(observation.status);
                metrics.last_error = None;
                metrics.singleton = observation.complete_singleton_root;
                metrics.observation_age_ms = observation.started_at.elapsed().as_millis();
            }
            Err(error) => {
                metrics.last_status = None;
                metrics.last_error = Some(*error);
                metrics.singleton = false;
            }
        }
    }
    let now = Instant::now();
    let effects = match result {
        Ok(observation) => tracker.observe_sql(
            &h.runtime.attention_owner,
            observation.status,
            observation.complete_singleton_root,
            observation.started_at,
            now,
        ),
        Err(_) => {
            tracker.reducer.disconnected();
            tracker.tick_sql(&h.runtime.attention_owner, now)
        }
    };
    tracker.live_revision = h.runtime.attention_owner.snapshot();
    Some(effects)
}
