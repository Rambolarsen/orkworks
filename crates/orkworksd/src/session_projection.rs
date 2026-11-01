use crate::capacity_analysis::{CapacityAnalysisCache, CapacityInput, PreparedCapacity};
use crate::git;
use crate::metadata;
#[cfg(test)]
use crate::plan_handoff::resolve_openable_plan_reference;
use crate::plan_handoff::PlanAvailabilityResolver;
use crate::session_types::SessionInfo;
use crate::session_view::{
    detect_conflicts, merge_live_session_info, resolve_harness_or_generic_shell,
    session_recommendation,
};
use crate::AppState;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const GIT_CONTEXT_TTL: Duration = Duration::from_secs(5);
const MAX_GIT_CONTEXT_ENTRIES: usize = 512;

/// A fresh workspace may have no session directory. Once observed, its
/// disappearance is an incomplete read, including across stream reconnects.
#[derive(Default)]
pub(crate) struct SessionDirectoryState {
    directory: Option<PathBuf>,
    established: bool,
    analysis: Arc<std::sync::Mutex<CapacityAnalysisCache>>,
}

impl SessionDirectoryState {
    fn observe(&mut self, directory: &Path) -> io::Result<()> {
        if self.directory.as_deref() != Some(directory) {
            self.directory = Some(directory.to_owned());
            self.established = false;
        }
        match std::fs::metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {
                self.established = true;
                Ok(())
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "session directory is not a directory",
            )),
            Err(error) if error.kind() == io::ErrorKind::NotFound && !self.established => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn read(
        &mut self,
        store: &metadata::MetadataStore,
    ) -> io::Result<Vec<metadata::SessionMetadata>> {
        let directory = store.sessions_dir();
        self.observe(&directory)?;
        let sessions = read_remembered_sessions(store)?;
        self.observe(&directory)?;
        Ok(sessions)
    }
}

struct CachedGitContext {
    scanned_at: Instant,
    context: git::GitContext,
}

/// Process-local results shared across projection requests. Callers resolve
/// worktree roots before lookup; workspace adoption clears all entries.
#[derive(Default)]
pub(crate) struct GitContextCache {
    entries: HashMap<PathBuf, CachedGitContext>,
    pub(crate) changes: Arc<std::sync::Mutex<crate::git_changes::CaptureStore>>,
    pub(crate) changes_busy: Arc<std::sync::atomic::AtomicBool>,
}

impl GitContextCache {
    fn get_or_detect(
        &mut self,
        cwd: &Path,
        now: Instant,
        detect: impl FnOnce(&Path) -> git::GitContext,
    ) -> git::GitContext {
        if let Some(entry) = self.entries.get(cwd) {
            if now.saturating_duration_since(entry.scanned_at) < GIT_CONTEXT_TTL {
                return entry.context.clone();
            }
        }
        // Expiry is measured from scan start, not last access: frequent polls
        // must never extend the freshness window. Prune even unused roots.
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.scanned_at) < GIT_CONTEXT_TTL);
        let context = detect(cwd);
        if self.entries.len() >= MAX_GIT_CONTEXT_ENTRIES {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.scanned_at)
                .map(|(path, _)| path.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            cwd.to_path_buf(),
            CachedGitContext {
                scanned_at: now,
                context: context.clone(),
            },
        );
        context
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.changes = Default::default();
        self.changes_busy = Default::default();
    }
}

/// Shared by the live and remembered session-info paths (issue #399): both
/// resolve the same openable-plan reference from `meta.plan_path` against
/// the current workspace, and previously reimplemented this expression
/// independently.
fn resolve_has_openable_plan(
    plan_path: Option<&metadata::PlanReference>,
    workspace: Option<&WorkspaceSnapshot>,
    resolve: &mut impl FnMut(&Path, &metadata::PlanReference) -> Result<PathBuf, String>,
) -> Option<bool> {
    plan_path.and_then(|reference| {
        workspace.map(|snapshot| resolve(&snapshot.workspace_path, reference).is_ok())
    })
}

/// Stateful coordinator for the session-listing projection.
///
/// This borrows the existing application state; it does not own a second
/// session registry or metadata store.
pub(crate) struct SessionProjection {
    state: Arc<AppState>,
}

#[derive(Clone)]
struct WorkspaceSnapshot {
    metadata_root: PathBuf,
    workspace_path: PathBuf,
}

struct ProjectionSnapshot {
    infos: Vec<SessionInfo>,
}

// A path alone cannot distinguish replacement ownership at the same workspace.
struct CapacityWorkspace {
    path: PathBuf,
    lease: Option<std::sync::Weak<crate::workspace_runtime::WorkspaceLease>>,
    instance: u64,
}

impl CapacityWorkspace {
    fn capture(workspace: &crate::WorkspaceState) -> Self {
        Self {
            path: workspace.path.clone(),
            lease: workspace.lease.as_ref().map(Arc::downgrade),
            instance: workspace.workflow_observations.instance_id(),
        }
    }

    fn matches(&self, workspace: &crate::WorkspaceState) -> bool {
        self.path == workspace.path
            && match (&self.lease, &workspace.lease) {
                (Some(expected), Some(current)) => expected
                    .upgrade()
                    .is_some_and(|expected| Arc::ptr_eq(&expected, current)),
                (None, None) => self.instance == workspace.workflow_observations.instance_id(),
                _ => false,
            }
    }
}

fn capacity_fingerprint(
    harness: Option<&crate::harness::registry::ResolvedHarness>,
) -> Option<Vec<u8>> {
    harness.map(|harness| {
        serde_json::to_vec(&(&harness.definition, &harness.effective_capabilities))
            .expect("resolved harness serializes")
    })
}

impl SessionProjection {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn snapshot(&self) -> io::Result<ProjectionSnapshot> {
        let mut plans = PlanAvailabilityResolver::default();
        self.snapshot_with_plan_resolver(&mut |workspace, reference| {
            plans.resolve(workspace, reference)
        })
    }

    fn snapshot_with_plan_resolver(
        &self,
        resolve: &mut impl FnMut(&Path, &metadata::PlanReference) -> Result<PathBuf, String>,
    ) -> io::Result<ProjectionSnapshot> {
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        let live_sessions: Vec<SessionInfo> = self
            .state
            .sessions
            .lock()
            .unwrap()
            .values()
            .map(|handle| handle.info.clone())
            .collect();
        let workspace = self.state.workspace.lock().unwrap().as_ref().map(|ws| {
            let metadata_root = ws.metadata.root_path();
            WorkspaceSnapshot {
                metadata_root,
                workspace_path: ws.path.clone(),
            }
        });

        // State locks are released before constructing the reader or reading
        // metadata from disk.
        let metadata = workspace
            .as_ref()
            .map(|snapshot| metadata::MetadataStore::new(&snapshot.metadata_root));
        let mut metadata_map = HashMap::new();
        if let Some(store) = metadata.as_ref() {
            for info in &live_sessions {
                if let Some(record) = store.try_read_revisioned_session(&info.id)? {
                    metadata_map.insert(info.id.clone(), record.metadata);
                }
            }
        }
        let remembered_sessions = metadata
            .as_ref()
            .map(|store| {
                self.state
                    .session_directory_state
                    .lock()
                    .map_err(|_| io::Error::other("session directory state lock poisoned"))?
                    .read(store)
            })
            .transpose()?
            .unwrap_or_default();
        let mut codex_native_session_ids = live_sessions
            .iter()
            .filter_map(|info| {
                let meta = metadata_map.get(&info.id);
                let harness_id = meta
                    .and_then(|meta| (!meta.harness.is_empty()).then_some(meta.harness.as_str()))
                    .or(info.harness_id.as_deref());
                (harness_id == Some("codex"))
                    .then(|| {
                        meta.and_then(|meta| meta.resume.as_ref())
                            .or(info.resume.as_ref())
                    })
                    .flatten()
                    .and_then(|resume| resume.harness_session_id.clone())
            })
            .collect::<Vec<_>>();
        codex_native_session_ids.extend(
            remembered_sessions
                .iter()
                .filter(|meta| meta.harness == "codex")
                .filter_map(|meta| {
                    meta.resume
                        .as_ref()
                        .and_then(|resume| resume.harness_session_id.clone())
                }),
        );
        codex_native_session_ids.sort_unstable();
        codex_native_session_ids.dedup();
        let saved_codex_session_ids =
            crate::codex_session_store::saved_sessions(&codex_native_session_ids);
        let live_ids: HashSet<String> = live_sessions.iter().map(|info| info.id.clone()).collect();
        // PTY output also writes this map. Capture only live-session values
        // and release its guard before any filesystem/Git plan validation.
        let peon_last_inference: HashMap<_, _> = {
            let inference = self.state.peon.last_inference.read().unwrap();
            live_ids
                .iter()
                .filter_map(|id| inference.get(id).map(|value| (id.clone(), value.clone())))
                .collect()
        };

        let mut infos = live_sessions
            .into_iter()
            .map(|info| {
                let id = info.id.clone();
                let meta = metadata_map.get(&id);
                let harness_id = meta
                    .and_then(|meta| (!meta.harness.is_empty()).then_some(meta.harness.as_str()))
                    .or(info.harness_id.as_deref());
                let resolved_harness = resolve_harness_or_generic_shell(&registry, harness_id);
                let mut info = merge_live_session_info(
                    info,
                    meta,
                    peon_last_inference.get(&id),
                    resolved_harness,
                    &saved_codex_session_ids,
                );
                info.has_openable_plan = resolve_has_openable_plan(
                    meta.and_then(|meta| meta.plan_path.as_ref()),
                    workspace.as_ref(),
                    resolve,
                );
                info
            })
            .collect::<Vec<_>>();

        for meta in remembered_sessions {
            if live_ids.contains(&meta.id) {
                continue;
            }
            infos.push(remembered_session_info(
                &meta,
                &registry,
                workspace.as_ref(),
                &saved_codex_session_ids,
                resolve,
            ));
        }

        Ok(ProjectionSnapshot { infos })
    }

    pub(crate) fn list(&self) -> io::Result<Vec<SessionInfo>> {
        self.list_with_capacity_analysis(|cache, inputs| cache.analyze(inputs))
    }

    // The compute dependency is private and used by the real projection path;
    // tests can stall it without adding a runtime race-injection switch.
    fn list_with_capacity_analysis(
        &self,
        analyze: impl FnOnce(
            &mut CapacityAnalysisCache,
            Vec<CapacityInput>,
        ) -> HashMap<String, PreparedCapacity>,
    ) -> io::Result<Vec<SessionInfo>> {
        let analysis = self
            .state
            .session_directory_state
            .lock()
            .unwrap()
            .analysis
            .clone();
        let mut cache = analysis.try_lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "capacity analysis already active",
            )
        })?;
        let (workspace, inputs) = {
            let _fence = self.state.projection_lock.lock().unwrap();
            self.capture_capacity()?
        };
        let prepared = analyze(&mut cache, inputs);
        drop(cache);
        let _fence = self.state.projection_lock.lock().unwrap();
        {
            let current = self.state.workspace.lock().unwrap();
            let matches = match (&workspace, current.as_ref()) {
                (None, None) => true,
                (Some(expected), Some(current)) => expected.matches(current),
                _ => false,
            };
            if !matches {
                return Err(io::Error::other(
                    "workspace changed during capacity analysis",
                ));
            }
        }
        let mut infos = self.project_capacity(self.snapshot()?, &prepared)?;
        self.project_organization(&mut infos);
        Ok(self.enrich_workspace(infos))
    }

    fn capture_capacity(&self) -> io::Result<(Option<CapacityWorkspace>, Vec<CapacityInput>)> {
        let registry = self.state.harness_catalog.read().unwrap().clone();
        let (workspace, metadata) = {
            let workspace = self.state.workspace.lock().unwrap();
            (
                workspace.as_ref().map(CapacityWorkspace::capture),
                workspace
                    .as_ref()
                    .map(|w| metadata::MetadataStore::new(&w.metadata.root_path())),
            )
        };
        let mut inputs: Vec<_> = self
            .state
            .sessions
            .lock()
            .unwrap()
            .values()
            .map(|handle| {
                (
                    handle.info.harness_id.clone(),
                    CapacityInput {
                        id: handle.info.id.clone(),
                        identity: handle.runtime.identity(),
                        sender: handle.runtime.control_tx.clone(),
                        fingerprint: None,
                        patterns: None,
                        state: handle.capacity.clone(),
                        lines: handle.output_buffer.shared_snapshot(),
                        raw: handle.scan_buf.clone(),
                        output_at: handle.info.last_output_at.clone(),
                    },
                )
            })
            .collect();
        for (live_harness, input) in &mut inputs {
            let persisted = metadata
                .as_ref()
                .map(|store| store.try_read_revisioned_session(&input.id))
                .transpose()?
                .flatten();
            let harness_id = persisted
                .as_ref()
                .and_then(|record| {
                    (!record.metadata.harness.is_empty())
                        .then_some(record.metadata.harness.as_str())
                })
                .or(live_harness.as_deref());
            let resolved = resolve_harness_or_generic_shell(&registry, harness_id);
            input.fingerprint = capacity_fingerprint(resolved);
            input.patterns = resolved.map(|harness| harness.capacity_patterns().to_vec());
        }
        Ok((
            workspace,
            inputs.into_iter().map(|(_, input)| input).collect(),
        ))
    }

    pub(crate) fn project_organization(&self, infos: &mut [SessionInfo]) {
        self.project_organization_with(infos, |store, info, closed, active, key| {
            store.project(&info.id, &info.created_at, closed, active, key)
        });
    }

    fn project_organization_with(
        &self,
        infos: &mut [SessionInfo],
        mut read: impl FnMut(
            &crate::session_archive::OrganizationStore,
            &SessionInfo,
            bool,
            bool,
            &str,
        ) -> crate::session_archive::OrganizationProjection,
    ) {
        // Snapshot runtime authority under the workspace, sessions and PID
        // mutexes; release all three before filesystem reads. List projection
        // retains its existing outer projection gate for write-back ordering.
        let inputs = {
            let workspace = self.state.workspace.lock().unwrap();
            let sessions = self.state.sessions.lock().unwrap();
            let pids = self.state.session_pids.lock().unwrap();
            workspace.as_ref().map(|ws| {
                let inputs = infos
                    .iter()
                    .map(|info| {
                        let handle = sessions.get(&info.id);
                        let closed = info.lifecycle_phase == "ended"
                            && info.lifecycle == "dead"
                            && crate::session_application::session_actions::runtime_closed(
                                handle,
                                pids.contains_key(&info.id),
                            );
                        let key = crate::session_application::session_actions::runtime_key(handle);
                        let active = !crate::session_application::session_actions::runtime_closed(
                            handle,
                            pids.contains_key(&info.id),
                        );
                        (closed, active, key)
                    })
                    .collect::<Vec<_>>();
                (ws.metadata.organization.clone(), inputs)
            })
        };
        if let Some((store, inputs)) = inputs {
            for (info, (closed, active, key)) in infos.iter_mut().zip(inputs) {
                info.organization = read(&store, info, closed, active, &key);
            }
        }
    }

    pub(crate) fn enrich_workspace(&self, mut infos: Vec<SessionInfo>) -> Vec<SessionInfo> {
        let session_pids = self.state.session_pids.lock().unwrap().clone();
        let reported_cwds = self.state.peon.reported_cwd.read().unwrap().clone();
        let locations = crate::session_view::resolve_effective_locations(
            &infos,
            &reported_cwds,
            &session_pids,
            crate::procfs::live_cwds,
        );
        let effective_cwds = locations
            .iter()
            .map(|(id, (path, _))| (id.clone(), path.clone()))
            .collect();
        for info in &mut infos {
            info.git_context_source = locations.get(&info.id).map(|(_, source)| *source);
        }
        {
            let mut cache = self
                .state
                .git_context_cache
                .lock()
                .expect("Git context cache lock poisoned");
            enrich_sessions_with_git_context(&mut infos, &effective_cwds, |cwd| {
                cache.get_or_detect(cwd, Instant::now(), git::detect)
            });
        }

        let conflict_warnings = detect_conflicts(&infos, &effective_cwds);
        for info in &mut infos {
            info.conflict_warning = conflict_warnings
                .iter()
                .find(|(id, _)| id == &info.id)
                .map(|(_, warning)| warning.clone());
        }
        infos
    }

    fn project_capacity(
        &self,
        snapshot: ProjectionSnapshot,
        prepared: &HashMap<String, PreparedCapacity>,
    ) -> io::Result<Vec<SessionInfo>> {
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .clone();
        let projected_infos = snapshot.infos;
        // Only immutable references were retained for compute. No text is
        // normalized or copied here, and the input gate is never awaited.
        let mut sessions = self.state.sessions.lock().unwrap();
        let mut input_guards = HashMap::new();
        for (id, handle) in sessions.iter() {
            let Some(candidate) = prepared.get(id) else {
                continue;
            };
            let harness_id = projected_infos
                .iter()
                .find(|info| &info.id == id)
                .and_then(|info| info.harness_id.as_deref())
                .or(handle.info.harness_id.as_deref());
            let fingerprint =
                capacity_fingerprint(resolve_harness_or_generic_shell(&registry, harness_id));
            if handle.runtime.matches_identity(&candidate.input.identity)
                && handle
                    .runtime
                    .control_tx
                    .same_channel(&candidate.input.sender)
                && handle.capacity.control_revision().is_some()
                && handle.capacity.control_revision() == candidate.input.state.control_revision()
                && fingerprint == candidate.input.fingerprint
            {
                if let Ok(guard) = handle.runtime.input_gate.clone().try_lock_owned() {
                    input_guards.insert(id.clone(), guard);
                }
            }
        }
        let live_sessions: Vec<_> = sessions
            .values()
            .map(|handle| (handle.info.clone(), handle.capacity.clone()))
            .collect();
        let durable_harnesses: HashMap<String, String> = projected_infos
            .iter()
            .filter_map(|info| {
                info.harness_id
                    .as_ref()
                    .map(|harness| (info.id.clone(), harness.clone()))
            })
            .collect();

        // Per-session scoped-recheck decisions from `CapacityState::observe`,
        // kept for the write-back stage: the *latch* write-back uses the
        // harness-aggregated `at_usage_limit` computed below (a shared
        // provider limit shows as capped across every session of that
        // harness), but the recheck-window `origin_update`/`clear_latch`
        // decision is always this session's own.
        let mut observations: HashMap<String, crate::capacity_state::CapacityObservation> =
            HashMap::new();
        let mut pending_transitions: Vec<(String, bool)> = Vec::new();
        let capacity_infos: Vec<SessionInfo> = live_sessions
            .into_iter()
            .map(|(info, capacity)| {
                let id = info.id.clone();
                let live_harness_id = info.harness_id.clone();
                let mut merged = projected_infos
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .cloned()
                    .unwrap_or(info);
                let harness_id = durable_harnesses
                    .get(&id)
                    .map(String::as_str)
                    .or(live_harness_id.as_deref());
                let resolved_harness = resolve_harness_or_generic_shell(&registry, harness_id);

                let observation = if input_guards.contains_key(&id) {
                    prepared
                        .get(&id)
                        .and_then(|candidate| candidate.observation.clone())
                } else {
                    resolved_harness.map(|_| crate::capacity_state::CapacityObservation {
                        at_usage_limit: capacity.at_usage_limit_latched,
                        reset_hint: merged.usage_limit_reset_hint.clone(),
                        origin_update: None,
                        clear_latch: false,
                    })
                };
                merged.at_usage_limit = observation.as_ref().map(|o| o.at_usage_limit);
                if merged.lifecycle == "alive" && merged.at_usage_limit == Some(true) {
                    merged.attention = Some("capped".into());
                }
                let preserve_debug_hint = merged.attention_source() == Some("debug")
                    && merged.lifecycle == "alive"
                    && merged.attention.as_deref() == Some("capped");
                let detected_reset_hint = observation.as_ref().and_then(|o| o.reset_hint.clone());
                if !preserve_debug_hint || detected_reset_hint.is_some() {
                    merged.usage_limit_reset_hint = detected_reset_hint;
                }
                merged.capacity_check_pending = capacity.rendered_check_pending();

                let scanned = prepared
                    .get(&id)
                    .map(|candidate| &candidate.input.state)
                    .unwrap_or(&capacity);
                let has_fresh_resume_output = scanned.capacity_check_pending
                    && !scanned.pending_capacity_visible_once
                    && scanned.fresh_output_since_origin();
                pending_transitions.push((id.clone(), has_fresh_resume_output));
                if let Some(observation) = observation {
                    observations.insert(id, observation);
                }
                merged
            })
            .collect();

        let mut infos = projected_infos;
        for info in &mut infos {
            let Some(capacity_info) = capacity_infos
                .iter()
                .find(|candidate| candidate.id == info.id)
            else {
                continue;
            };
            info.at_usage_limit = capacity_info.at_usage_limit;
            info.capacity_check_pending = capacity_info.capacity_check_pending;
            info.usage_limit_reset_hint = capacity_info.usage_limit_reset_hint.clone();
            if capacity_info.at_usage_limit == Some(true) && info.lifecycle == "alive" {
                info.attention = capacity_info.attention.clone();
            }
        }

        let mut harness_capped: HashMap<String, bool> = HashMap::new();
        let mut harness_reset_hint: HashMap<String, String> = HashMap::new();
        let mut provider_checking: HashSet<String> = HashSet::new();
        for info in &infos {
            if let (Some(hid), Some(capped)) = (&info.harness_id, info.at_usage_limit) {
                *harness_capped.entry(hid.clone()).or_insert(false) |= capped;
            }
            if let (Some(hid), Some(hint)) = (&info.harness_id, &info.usage_limit_reset_hint) {
                harness_reset_hint
                    .entry(hid.clone())
                    .or_insert_with(|| hint.clone());
            }
            if info.capacity_check_pending == Some(true) {
                if let Some(hid) = &info.harness_id {
                    provider_checking.insert(hid.clone());
                }
            }
        }
        for info in &mut infos {
            if info.memory_state != crate::session_types::MemoryState::Live {
                continue;
            }
            if let Some(hid) = &info.harness_id {
                if let Some(&capped) = harness_capped.get(hid) {
                    info.at_usage_limit = Some(capped);
                    if capped && info.lifecycle == "alive" {
                        info.attention = Some("capped".into());
                    }
                }
                if info.usage_limit_reset_hint.is_none() {
                    if let Some(hint) = harness_reset_hint.get(hid) {
                        info.usage_limit_reset_hint = Some(hint.clone());
                    }
                }
            }
        }
        let mut write_back_snapshot_ids = HashSet::new();
        for info in &infos {
            if let Some(handle) = sessions.get_mut(&info.id) {
                if !input_guards.contains_key(&info.id) {
                    continue;
                }
                write_back_snapshot_ids.insert(info.id.clone());
                let scoped_observation = observations.get(&info.id);
                let final_observation = crate::capacity_state::CapacityObservation {
                    at_usage_limit: info.at_usage_limit == Some(true),
                    reset_hint: None,
                    origin_update: scoped_observation.and_then(|o| o.origin_update),
                    clear_latch: scoped_observation.is_some_and(|o| o.clear_latch),
                };
                let newly_latched = handle.capacity.apply_observation(&final_observation);
                handle.info.at_usage_limit = info.at_usage_limit;
                handle.info.usage_limit_reset_hint = info.usage_limit_reset_hint.clone();
                if newly_latched {
                    handle.runtime.usage_limit_latched_at = prepared
                        .get(&info.id)
                        .and_then(|candidate| candidate.input.output_at.as_ref())
                        .as_deref()
                        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                        .map(|timestamp| timestamp.with_timezone(&chrono::Utc));
                }
            }
        }
        harness_capped.clear();
        harness_reset_hint.clear();
        provider_checking.clear();
        for info in &infos {
            if let (Some(hid), Some(capped)) = (&info.harness_id, info.at_usage_limit) {
                *harness_capped.entry(hid.clone()).or_insert(false) |= capped;
            }
            if let (Some(hid), Some(hint)) = (&info.harness_id, &info.usage_limit_reset_hint) {
                harness_reset_hint
                    .entry(hid.clone())
                    .or_insert_with(|| hint.clone());
            }
            if info.capacity_check_pending == Some(true) {
                if let Some(hid) = &info.harness_id {
                    provider_checking.insert(hid.clone());
                }
            }
        }
        self.state.providers.update_session_capping(
            harness_capped,
            harness_reset_hint,
            provider_checking,
        );
        for (id, has_fresh_resume_output) in &pending_transitions {
            if !write_back_snapshot_ids.contains(id) {
                continue;
            }
            let Some(handle) = sessions.get_mut(id) else {
                continue;
            };
            if let Some(new_value) = handle
                .capacity
                .advance_pending_visibility(*has_fresh_resume_output)
            {
                handle.info.capacity_check_pending = new_value;
            }
        }
        Ok(infos)
    }
}

/// Scan remembered records without the legacy read path's quarantine and
/// skip-on-error behavior. A missing directory is a fresh workspace; a file
/// listed by the directory must remain readable and valid for this response.
fn read_remembered_sessions(
    store: &metadata::MetadataStore,
) -> io::Result<Vec<metadata::SessionMetadata>> {
    let entries = match std::fs::read_dir(store.sessions_dir()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut sessions = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid session filename")
            })?;
        let record = store.try_read_revisioned_session(id)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "listed session record disappeared")
        })?;
        sessions.push(record.metadata);
    }
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(sessions)
}

pub(crate) fn enrich_sessions_with_git_context<F>(
    infos: &mut [SessionInfo],
    effective_cwds: &HashMap<String, String>,
    detect_git: F,
) where
    F: FnMut(&std::path::Path) -> git::GitContext,
{
    enrich_sessions_with_git_context_using(infos, effective_cwds, git::worktree_root, detect_git);
}

fn enrich_sessions_with_git_context_using(
    infos: &mut [SessionInfo],
    effective_cwds: &HashMap<String, String>,
    mut worktree_root: impl FnMut(&Path) -> Option<PathBuf>,
    mut detect_git: impl FnMut(&Path) -> git::GitContext,
) {
    let cwd_for = |info: &SessionInfo| {
        effective_cwds
            .get(&info.id)
            .cloned()
            .unwrap_or_else(|| info.cwd.clone())
    };
    // A workspace can retain hundreds of sessions in the same directory.
    // Root discovery opens/parses Git configuration too, so deduplicate it
    // before consulting the root-keyed status cache. Keep this request-local
    // so repository creation/removal is observed on the next projection.
    let mut roots: HashMap<String, String> = HashMap::new();
    let mut contexts: HashMap<String, git::GitContext> = HashMap::new();
    for info in infos.iter_mut() {
        if info
            .worktree_binding
            .as_ref()
            .is_some_and(|binding| binding.validate().is_err())
        {
            info.repo_root = None;
            info.branch = None;
            info.dirty = None;
            info.changed_files = None;
            info.line_changes = None;
            info.is_worktree = None;
            info.recommendation = Some("Reported task worktree unavailable.".into());
            continue;
        }
        let cwd = cwd_for(info);
        let git_cwd = roots.entry(cwd.clone()).or_insert_with(|| {
            worktree_root(Path::new(&cwd))
                .map(|root| root.display().to_string())
                .unwrap_or_else(|| cwd.clone())
        });
        let ctx = contexts
            .entry(git_cwd.clone())
            .or_insert_with(|| detect_git(std::path::Path::new(&git_cwd)));
        if let Some(reason) = &ctx.unavailable_reason {
            info.repo_root = None;
            info.branch = None;
            info.dirty = None;
            info.changed_files = None;
            info.line_changes = None;
            info.is_worktree = None;
            info.recommendation = Some(reason.clone());
            continue;
        }
        info.repo_root = ctx.repo_root.clone();
        info.branch = ctx.branch.clone();
        info.dirty = Some(ctx.dirty);
        info.changed_files = Some(ctx.changed_files);
        info.line_changes = ctx.line_changes;
        info.is_worktree = Some(ctx.is_worktree);
    }

    // Count only after enrichment: persisted repoRoot values can be stale, and
    // unavailable bindings/status scans must not count as known participants.
    let mut worktree_counts: HashMap<String, usize> = HashMap::new();
    for info in infos.iter() {
        if info.organization.session_state != Some(crate::session_archive::SessionState::Archived)
            && (info.status == "running" || info.status == "creating")
            && info.dirty.is_some()
        {
            let key = crate::session_view::git_context_group_key(info, effective_cwds);
            *worktree_counts.entry(key.to_owned()).or_default() += 1;
        }
    }
    for info in infos.iter_mut() {
        if info.dirty.is_none() {
            continue;
        }
        let cwd = cwd_for(info);
        if let Some(ctx) = roots.get(&cwd).and_then(|root| contexts.get(root)) {
            let key = crate::session_view::git_context_group_key(info, effective_cwds);
            let count = worktree_counts.get(key).copied().unwrap_or(1);
            info.recommendation = if info.organization.session_state
                == Some(crate::session_archive::SessionState::Archived)
            {
                None
            } else {
                session_recommendation(ctx, count)
            };
        }
    }
}

/// Projects a remembered (no live handle) session's persisted metadata into
/// a `SessionInfo`. Delegates all field precedence to
/// `session_view::project_session_info` — the same function the live path
/// uses — via a baseline built purely from `meta`
/// ([`SessionInfo::baseline_from_metadata`]) and `is_live: false` (a
/// remembered session's live handle is gone, regardless of what its stale
/// persisted `lifecycle`/`status` claim; issue #399).
fn remembered_session_info(
    meta: &metadata::SessionMetadata,
    registry: &crate::harness::registry::ResolvedHarnessRegistry,
    workspace: Option<&WorkspaceSnapshot>,
    saved_codex_session_ids: &std::collections::HashSet<String>,
    resolve: &mut impl FnMut(&Path, &metadata::PlanReference) -> Result<PathBuf, String>,
) -> SessionInfo {
    let harness_id = (!meta.harness.is_empty()).then_some(meta.harness.as_str());
    let resolved_harness = resolve_harness_or_generic_shell(registry, harness_id);
    let baseline = SessionInfo::baseline_from_metadata(meta);
    let mut info = crate::session_view::project_session_info(
        baseline,
        Some(meta),
        false,
        None,
        resolved_harness,
        saved_codex_session_ids,
    );
    info.has_openable_plan = resolve_has_openable_plan(meta.plan_path.as_ref(), workspace, resolve);
    info
}

#[cfg(test)]
mod tests {
    use super::{remembered_session_info, SessionProjection};
    use crate::harness::definition::{BuiltinDocument, HarnessUserDocument, EMBEDDED_BUILTINS};
    use crate::harness::registry::resolve_document;
    use crate::session_types::MemoryState;
    use crate::test_support::test_session_metadata;
    use crate::AppState;
    use std::sync::Arc;

    fn input_fixture() -> (
        tempfile::TempDir,
        Arc<AppState>,
        tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let (runtime, receiver) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
        let (kill_tx, _) = tokio::sync::watch::channel(false);
        let mut info = crate::test_support::test_session_info(
            "lag-test",
            "Known",
            dir.path().display().to_string(),
            "running",
            "now",
        );
        info.harness_id = Some("codex".into());
        info.harness = Some("codex".into());
        state.sessions.lock().unwrap().insert(
            "lag-test".into(),
            crate::SessionHandle {
                info,
                runtime,
                kill_tx,
                active_work_hook: false,
                output_buffer: crate::peon::RingBuffer::new(200),
                scan_buf: String::new(),
                pending_work_signal: None,
                terminal_attached: false,
                resume_in_progress: false,
                capacity: Default::default(),
            },
        );
        (dir, state, receiver)
    }

    #[test]
    fn acknowledged_input_excludes_capacity_commit_until_bookkeeping_finishes() {
        use std::time::Duration;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (_dir, state, mut receiver) = input_fixture();
            {
                let mut sessions = state.sessions.lock().unwrap();
                let handle = sessions.get_mut("lag-test").unwrap();
                handle.info.harness_id = Some("opencode".into());
                handle.info.harness = Some("opencode".into());
                handle.output_buffer.push("usage limit reached".into());
            }
            let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            let projection_state = state.clone();
            let projection = std::thread::spawn(move || {
                let result = SessionProjection::new(projection_state).list_with_capacity_analysis(
                    |cache, inputs| {
                        let prepared = cache.analyze(inputs);
                        let _ = entered_tx.send(());
                        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                        prepared
                    },
                );
                let _ = done_tx.send(result);
            });
            entered_rx.await.unwrap();
            let (identity, sender) = {
                let sessions = state.sessions.lock().unwrap();
                (
                    sessions["lag-test"].runtime.identity(),
                    sessions["lag-test"].runtime.control_tx.clone(),
                )
            };
            let input_state = state.clone();
            let owner = tokio::spawn(async move {
                crate::runtime::terminal_runtime::submit_approved_input_to_runtime(
                    &input_state,
                    "lag-test",
                    &identity,
                    &sender,
                    "x".into(),
                )
                .await
            });
            let command = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            let crate::runtime::session_runtime::RuntimeCommand::Input {
                accepted: Some(ack),
                ..
            } = command
            else {
                panic!("expected input");
            };
            // One blocking worker: preparation has completed before this barrier,
            // and acknowledged-input bookkeeping cannot run until we release it.
            let (blocked_tx, blocked_rx) = tokio::sync::oneshot::channel();
            let (unblock_tx, unblock_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                let _ = blocked_tx.send(());
                unblock_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            });
            blocked_rx.await.unwrap();
            ack.send(Ok(())).unwrap();
            release_tx.send(()).unwrap();
            let result = tokio::time::timeout(Duration::from_secs(3), done_rx).await;
            let before = state.sessions.lock().unwrap()["lag-test"].capacity.clone();
            unblock_tx.send(()).unwrap();
            blocker.await.unwrap();
            let delivered = owner.await.unwrap();
            projection.join().unwrap();
            assert!(delivered.is_ok());
            assert_eq!(
                result.unwrap().unwrap().unwrap()[0].at_usage_limit,
                Some(false)
            );
            assert!(!before.at_usage_limit_latched);
            assert_eq!(
                before.control_revision(),
                Some(0),
                "bookkeeping was held after delivery"
            );
            assert!(
                state.sessions.lock().unwrap()["lag-test"]
                    .capacity
                    .control_revision()
                    .unwrap()
                    > 0
            );
        });
    }

    #[test]
    fn output_churn_commits_the_scanned_prefix_without_rewinding_newer_counters() {
        let (_dir, state, _receiver) = input_fixture();
        {
            let mut sessions = state.sessions.lock().unwrap();
            let handle = sessions.get_mut("lag-test").unwrap();
            handle.info.harness_id = Some("opencode".into());
            handle.info.harness = Some("opencode".into());
            handle.output_buffer.push("usage limit reached".into());
            handle.capacity.record_output(1, 20);
            handle.capacity.resume_scan_origin = Some((0, 0));
            handle.info.last_output_at = Some("2026-10-31T20:00:00Z".into());
        }
        let mutation = state.clone();
        let infos = SessionProjection::new(state.clone())
            .list_with_capacity_analysis(|cache, inputs| {
                let prepared = cache.analyze(inputs);
                let mut sessions = mutation.sessions.lock().unwrap();
                let handle = sessions.get_mut("lag-test").unwrap();
                handle.capacity.record_output(2, 100);
                handle.output_buffer.push("newer unrelated output".into());
                handle.info.last_output_at = Some("2026-10-31T20:01:00Z".into());
                prepared
            })
            .unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].at_usage_limit, Some(true));
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions["lag-test"];
        assert_eq!(
            (
                handle.capacity.output_lines_seen,
                handle.capacity.scan_bytes_seen
            ),
            (3, 120)
        );
        assert_eq!(handle.capacity.resume_scan_origin, Some((1, 20)));
        assert!(handle.capacity.at_usage_limit_latched);
        assert_eq!(
            handle.runtime.usage_limit_latched_at.unwrap().to_rfc3339(),
            "2026-10-31T20:00:00+00:00"
        );
    }

    #[test]
    fn control_runtime_and_workspace_changes_reject_old_capacity_evidence() {
        for change in ["input", "clear", "runtime", "workspace"] {
            let (dir, state, _receiver) = input_fixture();
            {
                let mut sessions = state.sessions.lock().unwrap();
                let handle = sessions.get_mut("lag-test").unwrap();
                handle.info.harness_id = Some("opencode".into());
                handle.info.harness = Some("opencode".into());
                handle.output_buffer.push("usage limit reached".into());
            }
            let mutation = state.clone();
            let result =
                SessionProjection::new(state.clone()).list_with_capacity_analysis(
                    |cache, inputs| {
                        let prepared = cache.analyze(inputs);
                        match change {
                            "workspace" => {
                                *mutation.workspace.lock().unwrap() = Some(crate::WorkspaceState {
                                path: dir.path().to_owned(),
                                metadata: crate::metadata::MetadataStore::new(
                                    &dir.path().join(".orkworks"),
                                ),
                                workflow_observations:
                                    crate::workflow_observations::WorkflowObservationStore::open(
                                        dir.path().join(".replacement"),
                                    )
                                    .unwrap(),
                                recommendation_store:
                                    crate::taskmaster::store::RecommendationStore::open(
                                        dir.path().join(".replacement"),
                                    )
                                    .unwrap(),
                                lease: None,
                            });
                            }
                            _ => {
                                let mut sessions = mutation.sessions.lock().unwrap();
                                let handle = sessions.get_mut("lag-test").unwrap();
                                match change {
                                    "input" => handle.capacity.accepted_input(),
                                    "clear" => handle.capacity.clear_latch_and_rearm(),
                                    "runtime" => handle.runtime =
                                        crate::runtime::session_runtime::SessionRuntime::detached(
                                            24, 80,
                                        ),
                                    _ => unreachable!(),
                                }
                            }
                        }
                        prepared
                    },
                );
            if change == "workspace" {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap()[0].at_usage_limit, Some(false), "{change}");
            }
            assert!(
                !state.sessions.lock().unwrap()["lag-test"]
                    .capacity
                    .at_usage_limit_latched,
                "{change}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stalled_capacity_scan_does_not_block_real_input_delivery_or_bookkeeping() {
        use std::time::Duration;
        let (_dir, state, mut receiver) = input_fixture();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let projection_state = state.clone();
        let projection = std::thread::spawn(move || {
            SessionProjection::new(projection_state).list_with_capacity_analysis(|cache, inputs| {
                let _ = entered_tx.send(());
                release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                cache.analyze(inputs)
            })
        });
        entered_rx.await.unwrap();
        let (identity, sender) = {
            let sessions = state.sessions.lock().unwrap();
            let runtime = &sessions["lag-test"].runtime;
            (runtime.identity(), runtime.control_tx.clone())
        };
        let input_state = state.clone();
        let owner = tokio::spawn(async move {
            crate::runtime::terminal_runtime::submit_approved_input_to_runtime(
                &input_state,
                "lag-test",
                &identity,
                &sender,
                "x".into(),
            )
            .await
        });
        let command = tokio::time::timeout(Duration::from_secs(2), receiver.recv()).await;
        if let Ok(Some(crate::runtime::session_runtime::RuntimeCommand::Input {
            accepted: Some(ack),
            ..
        })) = command
        {
            ack.send(Ok(())).unwrap();
        } else {
            let _ = release_tx.send(());
            projection.join().unwrap().unwrap();
            panic!("input could not pass while pure capacity analysis was paused");
        }
        let result = tokio::time::timeout(Duration::from_secs(2), owner).await;
        release_tx.send(()).unwrap();
        projection.join().unwrap().unwrap();
        assert!(
            result.unwrap().unwrap().is_ok(),
            "bookkeeping must also finish before analysis resumes"
        );
        assert!(
            state.sessions.lock().unwrap()["lag-test"]
                .capacity
                .control_revision()
                .unwrap()
                > 0
        );
    }

    #[test]
    #[ignore = "manual shared-state plan projection benchmark"]
    fn benchmark_plan_projection_repository_reuse() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        std::fs::create_dir(dir.path().join("specs")).unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let store = &workspace.as_ref().unwrap().metadata;
            for i in 0..710 {
                let mut meta = test_session_metadata(
                    format!("history-{i}"),
                    "History",
                    dir.path().display().to_string(),
                    "ended",
                    "2026-01-01T00:00:00Z",
                    "2026-01-01T00:00:00Z",
                );
                if i < 32 {
                    let relative = format!("specs/plan-{i}.md");
                    std::fs::write(dir.path().join(&relative), "# Plan").unwrap();
                    meta.plan_path = Some(crate::metadata::PlanReference {
                        worktree_root: Some(dir.path().display().to_string()),
                        relative_path: relative,
                        source: crate::metadata::PlanSource::AgentReported,
                    });
                }
                store.write_session(&meta);
            }
        }
        let projection = SessionProjection::new(state.clone());
        let mut uncached = Vec::new();
        let mut request_local = Vec::new();
        for i in 0..7 {
            // Alternate order to reduce a systematic warm-filesystem advantage.
            for cached in if i % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let _gate = state.projection_lock.lock().unwrap();
                let start = std::time::Instant::now();
                let snapshot = if cached {
                    projection.snapshot().unwrap()
                } else {
                    projection
                        .snapshot_with_plan_resolver(&mut super::resolve_openable_plan_reference)
                        .unwrap()
                };
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(snapshot.infos.len(), 710);
                assert_eq!(
                    snapshot
                        .infos
                        .iter()
                        .filter(|row| row.has_openable_plan == Some(true))
                        .count(),
                    32
                );
                if cached {
                    request_local.push(elapsed);
                } else {
                    uncached.push(elapsed);
                }
            }
        }
        uncached.sort_by(f64::total_cmp);
        request_local.sort_by(f64::total_cmp);
        eprintln!("710-record shared-state snapshot, 32 anchored plans: uncached median {:.3} ms; request-local median {:.3} ms",
            uncached[3], request_local[3]);
    }

    // A plan validator stalled on filesystem/Git work must not stall the
    // same inference-map writer used by the PTY output driver.
    #[test]
    fn blocked_plan_validation_releases_inference_lock() {
        use std::sync::mpsc;
        use std::time::Duration;

        for live in [true, false] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("plan.md"), "# Plan\n").unwrap();
            let state = crate::test_support::test_app_state_with_workspace(dir.path());
            let mut meta = test_session_metadata(
                "session",
                "Session",
                dir.path().display().to_string(),
                "ended",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
            );
            meta.plan_path = Some(serde_json::from_str("\"plan.md\"").unwrap());
            state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .write_session(&meta);
            state
                .peon
                .last_inference
                .write()
                .unwrap()
                .insert("session".into(), "captured".into());
            if live {
                let (kill_tx, _) = tokio::sync::watch::channel(false);
                state.sessions.lock().unwrap().insert(
                    "session".into(),
                    crate::SessionHandle {
                        info: crate::test_support::test_session_info(
                            "session",
                            "Session",
                            dir.path().display().to_string(),
                            "running",
                            "now",
                        ),
                        kill_tx,
                        output_buffer: crate::peon::RingBuffer::new(200),
                        scan_buf: String::new(),
                        pending_work_signal: None,
                        runtime: crate::runtime::session_runtime::SessionRuntime::detached(24, 80),
                        terminal_attached: false,
                        resume_in_progress: false,
                        capacity: Default::default(),
                        active_work_hook: false,
                    },
                );
            }
            let (entered_tx, entered_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let projection_state = state.clone();
            let projection = std::thread::spawn(move || {
                SessionProjection::new(projection_state)
                    .snapshot_with_plan_resolver(&mut |root, reference| {
                        entered_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                        super::resolve_openable_plan_reference(root, reference)
                    })
                    .unwrap()
            });
            entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let (written_tx, written_rx) = mpsc::channel();
            let writer_state = state.clone();
            let writer = std::thread::spawn(move || {
                writer_state
                    .peon
                    .last_inference
                    .write()
                    .unwrap()
                    .insert("session".into(), "new output".into());
                written_tx.send(()).unwrap();
            });
            let written_while_blocked = written_rx.recv_timeout(Duration::from_secs(2)).is_ok();
            // Always unblock and join before asserting, including on regression.
            release_tx.send(()).unwrap();
            let snapshot = projection.join().unwrap();
            writer.join().unwrap();
            assert!(
                written_while_blocked,
                "plan validation retained inference lock (live={live})"
            );
            assert_eq!(snapshot.infos.len(), 1);
            assert_eq!(snapshot.infos[0].has_openable_plan, Some(true));
            assert_eq!(
                snapshot.infos[0].peon_last_inference.as_deref(),
                if live { Some("captured") } else { None }
            );
            std::fs::remove_file(dir.path().join("plan.md")).unwrap();
            let next = SessionProjection::new(state).snapshot().unwrap();
            assert_eq!(next.infos[0].has_openable_plan, Some(false));
            assert_eq!(
                next.infos[0].peon_last_inference.as_deref(),
                if live { Some("new output") } else { None }
            );
        }
    }

    #[test]
    fn legacy_closed_session_exposes_organization_and_action_precondition() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            workspace
                .as_ref()
                .unwrap()
                .metadata
                .write_session(&test_session_metadata(
                    "old",
                    "Old",
                    dir.path().display().to_string(),
                    "ended",
                    "2024-01-01T00:00:00Z",
                    "2024-01-01T00:00:00Z",
                ));
        }
        let sessions = SessionProjection::new(state).list().unwrap();
        let json = serde_json::to_value(&sessions[0]).unwrap();
        assert_eq!(json["sessionState"], "closed");
        assert!(json["sessionActionToken"]
            .as_str()
            .is_some_and(|token| !token.is_empty()));
    }

    #[test]
    fn organization_reads_release_application_state_locks() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let mut infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            dir.path().display().to_string(),
            "ended",
            "before",
        )];
        SessionProjection::new(state.clone()).project_organization_with(
            &mut infos,
            |store, info, closed, active, key| {
                assert!(
                    state.workspace.try_lock().is_ok(),
                    "workspace locked during archive read"
                );
                assert!(
                    state.sessions.try_lock().is_ok(),
                    "sessions locked during archive read"
                );
                assert!(
                    state.session_pids.try_lock().is_ok(),
                    "PIDs locked during archive read"
                );
                store.project(&info.id, &info.created_at, closed, active, key)
            },
        );
        assert!(infos[0].organization.session_action_token.is_some());
    }

    #[test]
    fn archived_history_does_not_receive_live_workspace_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let commit_id = repo
            .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        repo.branch("feature", &repo.find_commit(commit_id).unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        std::fs::write(dir.path().join("dirty.txt"), "uncommitted").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        {
            let workspace = state.workspace.lock().unwrap();
            let ws = workspace.as_ref().unwrap();
            for id in ["archived", "closed"] {
                let mut meta = test_session_metadata(
                    id,
                    id,
                    dir.path().display().to_string(),
                    "ended",
                    "before",
                    "before",
                );
                meta.cwd = dir.path().display().to_string();
                ws.metadata.write_session(&meta);
            }
            ws.metadata
                .organization
                .write(
                    "archived",
                    "before",
                    crate::session_archive::SessionState::Archived,
                )
                .unwrap();
        }
        {
            let workspace = state.workspace.lock().unwrap();
            let ws = workspace.as_ref().unwrap();
            let mut stale = ws.metadata.read_session("archived").unwrap();
            stale.status = "running".into();
            stale.lifecycle = "alive".into();
            stale.lifecycle_phase = "alive".into();
            ws.metadata.write_session(&stale);
            stale.id = "live".into();
            ws.metadata.write_session(&stale);
        }
        let infos = SessionProjection::new(state).list().unwrap();
        let live = infos.iter().find(|info| info.id == "live").unwrap();
        assert!(
            !live
                .recommendation
                .as_deref()
                .unwrap_or("")
                .contains("Multiple sessions"),
            "archived stale metadata must not count as a second current session"
        );
        assert!(live.conflict_warning.is_none());
        let archived = infos.iter().find(|info| info.id == "archived").unwrap();
        let closed = infos.iter().find(|info| info.id == "closed").unwrap();
        assert!(
            closed.recommendation.is_some(),
            "fixture must derive current guidance"
        );
        assert!(archived.recommendation.is_none());
        assert!(archived.conflict_warning.is_none());
        assert_eq!(
            archived.dirty,
            Some(true),
            "read-only Git context remains available"
        );
    }

    #[test]
    fn exposes_a_constructor_for_shared_app_state() {
        let _constructor: fn(Arc<AppState>) -> SessionProjection = SessionProjection::new;
    }

    fn warning_repo(path: &std::path::Path) -> git2::Repository {
        std::fs::create_dir_all(path).unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repo = git2::Repository::init_opts(path, &options).unwrap();
        let id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(id).unwrap();
            let sig = git2::Signature::now("test", "test@example.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn warning_session(
        id: &str,
        cwd: &std::path::Path,
        status: &str,
    ) -> crate::session_types::SessionInfo {
        std::fs::create_dir_all(cwd).unwrap();
        crate::test_support::test_session_info(id, id, cwd.display().to_string(), status, "now")
    }

    #[test]
    fn shared_worktree_subdirectories_warn_for_primary_and_linked_checkouts() {
        for linked in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let primary = temp.path().join("primary");
            let repo = warning_repo(&primary);
            let root = if linked {
                let path = temp.path().join("linked");
                repo.worktree("feature", &path, None).unwrap();
                path
            } else {
                primary
            };
            std::fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
            let mut archived = warning_session("archived", &root, "running");
            archived.organization.session_state =
                Some(crate::session_archive::SessionState::Archived);
            let infos = vec![
                warning_session("one", &root.join("src"), "running"),
                warning_session("two", &root.join("tests"), "creating"),
                warning_session("ended", &root, "ended"),
                archived,
            ];
            let state = crate::test_support::test_app_state_with_workspace(temp.path());
            let projected = SessionProjection::new(state).enrich_workspace(infos);
            for info in &projected[..2] {
                assert!(
                    info.conflict_warning
                        .as_deref()
                        .is_some_and(|s| s.contains("2 sessions")),
                    "linked={linked}: {}",
                    info.id
                );
                assert!(
                    info.recommendation
                        .as_deref()
                        .is_some_and(|s| s.starts_with("Multiple sessions")),
                    "linked={linked}: {}",
                    info.id
                );
            }
            assert!(projected[2].conflict_warning.is_none());
            assert!(projected[3].conflict_warning.is_none());
            assert!(projected[3].recommendation.is_none());
        }
    }

    #[test]
    fn shared_worktree_domains_separate_worktrees_clones_nested_repos_and_submodules() {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("primary");
        let repo = warning_repo(&primary);
        let linked = temp.path().join("linked");
        repo.worktree("feature", &linked, None).unwrap();
        let cloned = temp.path().join("clone");
        git2::Repository::clone(primary.to_str().unwrap(), &cloned).unwrap();
        let nested = primary.join("nested");
        warning_repo(&nested);
        let upstream = temp.path().join("upstream");
        warning_repo(&upstream);
        let output = std::process::Command::new("git")
            .args([
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "--quiet",
            ])
            .arg(&upstream)
            .arg("submodule")
            .current_dir(&primary)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let roots = [
            primary.clone(),
            linked,
            cloned,
            nested,
            primary.join("submodule"),
        ];
        let infos = roots
            .iter()
            .enumerate()
            .map(|(i, root)| {
                std::fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
                warning_session(&format!("session-{i}"), &root.join("child"), "running")
            })
            .collect();
        let state = crate::test_support::test_app_state_with_workspace(temp.path());
        let projected = SessionProjection::new(state).enrich_workspace(infos);
        assert!(projected.iter().all(|info| info.dirty == Some(true)));
        assert!(projected.iter().all(|info| info.conflict_warning.is_none()));
        assert!(projected.iter().all(|info| !info
            .recommendation
            .as_deref()
            .is_some_and(|s| s.starts_with("Multiple sessions"))));
        let roots = projected
            .iter()
            .map(|info| info.repo_root.as_ref().unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(roots.len(), 5);
    }

    #[test]
    fn shared_worktree_grouping_uses_reported_and_bound_locations() {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("primary");
        let repo = warning_repo(&primary);
        let linked = temp.path().join("linked");
        repo.worktree("feature", &linked, None).unwrap();
        for root in [&primary, &linked] {
            std::fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
        }
        let mut bound = warning_session("bound", &linked, "running");
        bound.worktree_binding = Some(
            crate::session_location::WorktreeBinding::resolve(
                linked.to_str().unwrap(),
                primary.to_str().unwrap(),
            )
            .unwrap(),
        );
        let infos = vec![
            warning_session("primary", &primary, "running"),
            warning_session("linked", &linked, "running"),
            warning_session("reported", &primary, "running"),
            bound,
        ];
        let state = crate::test_support::test_app_state_with_workspace(temp.path());
        state.peon.reported_cwd.write().unwrap().extend([
            (
                "reported".into(),
                linked.join("child").display().to_string(),
            ),
            ("bound".into(), linked.display().to_string()),
        ]);
        std::fs::create_dir(linked.join("child")).unwrap();
        let projected = SessionProjection::new(state).enrich_workspace(infos);
        for info in &projected {
            assert!(
                info.conflict_warning
                    .as_deref()
                    .is_some_and(|s| s.contains("2 sessions")),
                "{}",
                info.id
            );
            assert!(info
                .recommendation
                .as_deref()
                .is_some_and(|s| s.starts_with("Multiple sessions")));
        }
        assert_eq!(projected[0].repo_root, projected[3].repo_root);
        assert_eq!(projected[1].repo_root, projected[2].repo_root);
        assert_ne!(projected[0].repo_root, projected[1].repo_root);
        assert_eq!(
            projected[3].git_context_source,
            Some(crate::session_location::GitLocationSource::Agent)
        );
    }

    #[test]
    fn shared_worktree_counts_exclude_unavailable_bindings() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        warning_repo(&root);
        let foreign = warning_repo(&temp.path().join("foreign"));
        std::fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
        let mut invalid = warning_session("unavailable", &root, "running");
        invalid.worktree_binding = Some(crate::session_location::WorktreeBinding {
            root: root.canonicalize().unwrap().display().to_string(),
            common_dir: foreign
                .commondir()
                .canonicalize()
                .unwrap()
                .display()
                .to_string(),
        });
        let infos = vec![warning_session("healthy", &root, "running"), invalid];
        let state = crate::test_support::test_app_state_with_workspace(temp.path());
        let projected = SessionProjection::new(state).enrich_workspace(infos);
        assert!(projected.iter().all(|info| info.conflict_warning.is_none()));
        assert!(projected[0].recommendation.is_none());
        assert_eq!(projected[1].dirty, None);
        assert!(projected[1]
            .recommendation
            .as_deref()
            .unwrap()
            .contains("unavailable"));
    }

    #[test]
    fn git_root_discovery_scales_with_distinct_effective_directories() {
        let repo = tempfile::tempdir().unwrap();
        git2::Repository::init(repo.path()).unwrap();
        let child = repo.path().join("child");
        std::fs::create_dir(&child).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        git2::Repository::init(other.path()).unwrap();
        let paths = [repo.path(), child.as_path(), outside.path(), other.path()];
        let mut infos = (0..700)
            .map(|i| {
                crate::test_support::test_session_info(
                    &format!("session-{i}"),
                    "Session",
                    "/stale-launch-directory",
                    "running",
                    "now",
                )
            })
            .collect::<Vec<_>>();
        let effective_cwds = infos
            .iter()
            .enumerate()
            .map(|(i, info)| (info.id.clone(), paths[i % 4].display().to_string()))
            .collect();
        let mut root_reads = 0;
        let mut scans = 0;
        super::enrich_sessions_with_git_context_using(
            &mut infos,
            &effective_cwds,
            |cwd| {
                root_reads += 1;
                crate::git::worktree_root(cwd)
            },
            |cwd| {
                scans += 1;
                crate::git::GitContext {
                    repo_root: Some(cwd.display().to_string()),
                    ..Default::default()
                }
            },
        );
        assert_eq!(
            root_reads, 4,
            "history size must not multiply repository opens, including non-repositories"
        );
        assert_eq!(scans, 3, "subdirectories must still share their root scan");
        let repo_root = repo.path().canonicalize().unwrap();
        let other_root = other.path().canonicalize().unwrap();
        assert_eq!(
            infos[0].repo_root.as_deref().map(std::path::Path::new),
            Some(repo_root.as_path())
        );
        assert_eq!(infos[1].repo_root, infos[0].repo_root);
        assert_eq!(
            infos[3].repo_root.as_deref().map(std::path::Path::new),
            Some(other_root.as_path())
        );
    }

    #[test]
    fn git_root_discovery_refreshes_between_projections() {
        let parent = tempfile::tempdir().unwrap();
        let child = parent.path().join("child");
        std::fs::create_dir(&child).unwrap();
        let mut infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            child.display().to_string(),
            "running",
            "now",
        )];
        let mut detected_paths = Vec::new();
        for initialize in [false, true] {
            if initialize {
                git2::Repository::init(parent.path()).unwrap();
            }
            super::enrich_sessions_with_git_context(
                &mut infos,
                &std::collections::HashMap::new(),
                |cwd| {
                    detected_paths.push(cwd.to_path_buf());
                    crate::git::GitContext::default()
                },
            );
        }
        assert_eq!(
            detected_paths,
            vec![child, parent.path().canonicalize().unwrap()]
        );
    }

    #[test]
    fn git_context_cache_survives_separate_projection_requests() {
        let workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let file = repo_dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            repo_dir.path().display().to_string(),
            "running",
            "now",
        )];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert_eq!(first[0].line_changes.unwrap().additions, 1);

        std::fs::write(&file, "first\nsecond\n").unwrap();
        let second = SessionProjection::new(state).enrich_workspace(infos);
        assert_eq!(
            second[0].line_changes.unwrap().additions,
            1,
            "a separate poll must reuse the Git result within its TTL"
        );
    }

    #[test]
    fn git_context_cache_reuses_scans_but_refreshes_at_expiry() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let file = dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        let mut cache = super::GitContextCache::default();
        let now = std::time::Instant::now();
        let mut calls = 0;
        let mut detect = |cwd: &std::path::Path| {
            calls += 1;
            crate::git::detect(cwd)
        };
        let first = cache.get_or_detect(dir.path(), now, &mut detect);
        assert_eq!(first.line_changes.unwrap().additions, 1);
        std::fs::write(&file, "first\nsecond\n").unwrap();
        let reused = cache.get_or_detect(
            dir.path(),
            now + std::time::Duration::from_secs(4),
            &mut detect,
        );
        assert_eq!(reused.line_changes.unwrap().additions, 1);
        let refreshed = cache.get_or_detect(
            dir.path(),
            now + std::time::Duration::from_secs(5),
            &mut detect,
        );
        assert_eq!(refreshed.line_changes.unwrap().additions, 2);
        assert_eq!(refreshed.changed_files, 1);
        assert!(refreshed.dirty);
        assert_eq!(calls, 2, "a cache hit must skip the expensive detector");
    }

    #[test]
    fn git_context_cache_keeps_distinct_roots_and_evicts_oldest_when_full() {
        let mut cache = super::GitContextCache::default();
        let now = std::time::Instant::now();
        let context = crate::git::GitContext {
            repo_root: None,
            branch: None,
            dirty: false,
            changed_files: 0,
            line_changes: None,
            is_worktree: false,
            unavailable_reason: None,
        };
        for i in 0..513 {
            let path = std::path::PathBuf::from(format!("repo-{i}"));
            cache.get_or_detect(&path, now + std::time::Duration::from_millis(i), |_| {
                context.clone()
            });
        }
        assert_eq!(cache.entries.len(), 512);
        assert!(!cache.entries.contains_key(std::path::Path::new("repo-0")));
        assert!(cache.entries.contains_key(std::path::Path::new("repo-1")));
        assert!(cache.entries.contains_key(std::path::Path::new("repo-512")));
        cache.get_or_detect(
            std::path::Path::new("fresh"),
            now + std::time::Duration::from_secs(6),
            |_| context,
        );
        assert_eq!(
            cache.entries.len(),
            1,
            "unused expired roots must be removed"
        );
    }

    #[test]
    fn git_context_cache_shares_subdirectories_without_caching_session_recommendations() {
        let workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let child = repo_dir.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("new.txt"), "first\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let mut infos = vec![crate::test_support::test_session_info(
            "one",
            "One",
            child.display().to_string(),
            "running",
            "now",
        )];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert!(first[0].recommendation.is_some());
        infos.push(crate::test_support::test_session_info(
            "two",
            "Two",
            child.display().to_string(),
            "running",
            "now",
        ));
        infos.push(crate::test_support::test_session_info(
            "three",
            "Three",
            repo_dir.path().display().to_string(),
            "running",
            "now",
        ));
        let projected = SessionProjection::new(state.clone()).enrich_workspace(infos);
        assert_eq!(state.git_context_cache.lock().unwrap().entries.len(), 1);
        assert_eq!(projected[0].line_changes.unwrap().additions, 1);
        assert_eq!(projected[2].line_changes.unwrap().additions, 1);
        assert_ne!(projected[0].recommendation, first[0].recommendation);
        // The root session shares the same checkout as both child-directory sessions.
        for info in &projected {
            assert!(info
                .conflict_warning
                .as_deref()
                .is_some_and(|s| s.contains("3 sessions")));
            assert_eq!(info.recommendation, projected[0].recommendation);
        }
    }

    #[test]
    fn git_context_cache_is_cleared_when_a_workspace_is_adopted() {
        let home = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let next_workspace = tempfile::tempdir().unwrap();
        let repo_dir = tempfile::tempdir().unwrap();
        git2::Repository::init(repo_dir.path()).unwrap();
        let file = repo_dir.path().join("new.txt");
        std::fs::write(&file, "first\n").unwrap();
        crate::test_support::with_fake_home(home.path(), || {
            let state = crate::test_support::test_app_state_with_workspace(workspace.path());
            let infos = vec![crate::test_support::test_session_info(
                "one",
                "One",
                repo_dir.path().display().to_string(),
                "running",
                "now",
            )];
            SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
            std::fs::write(&file, "first\nsecond\n").unwrap();
            crate::session_application::SessionApplication::new(state.clone())
                .open_workspace(next_workspace.path().to_path_buf())
                .unwrap();
            let projected = SessionProjection::new(state).enrich_workspace(infos);
            assert_eq!(projected[0].line_changes.unwrap().additions, 2);
        });
    }

    #[test]
    fn git_context_cache_keeps_linked_worktrees_separate() {
        let workspace = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let main = root.path().join("main");
        let linked = root.path().join("linked");
        let repo = git2::Repository::init(&main).unwrap();
        let mut index = repo.index().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
            .unwrap();
        repo.worktree("linked", &linked, None).unwrap();
        std::fs::write(main.join("new.txt"), "one\n").unwrap();
        std::fs::write(linked.join("new.txt"), "one\ntwo\n").unwrap();
        let state = crate::test_support::test_app_state_with_workspace(workspace.path());
        let infos = vec![
            crate::test_support::test_session_info(
                "one",
                "One",
                main.display().to_string(),
                "running",
                "now",
            ),
            crate::test_support::test_session_info(
                "two",
                "Two",
                linked.display().to_string(),
                "running",
                "now",
            ),
        ];
        let first = SessionProjection::new(state.clone()).enrich_workspace(infos.clone());
        assert_eq!(first[0].line_changes.unwrap().additions, 1);
        assert_eq!(first[1].line_changes.unwrap().additions, 2);
        assert_eq!(first[0].is_worktree, Some(false));
        assert_eq!(first[1].is_worktree, Some(true));
        std::fs::write(main.join("new.txt"), "three\nfour\nfive\n").unwrap();
        let cached = SessionProjection::new(state.clone()).enrich_workspace(infos);
        assert_eq!(cached[0].line_changes.unwrap().additions, 1);
        assert_eq!(cached[1].line_changes.unwrap().additions, 2);
        assert_eq!(state.git_context_cache.lock().unwrap().entries.len(), 2);
    }

    fn registry() -> crate::harness::registry::ResolvedHarnessRegistry {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        resolve_document(&builtins, &HarnessUserDocument::default()).unwrap()
    }

    /// A remembered session's `meta.last_activity` is a required, always-set
    /// field, so `remembered_session_info` must surface it directly rather
    /// than falling back to `created_at` — pins the deliberate decision
    /// documented on `session_view::project_session_info` (issue #399).
    #[test]
    fn remembered_session_info_uses_metas_last_activity_without_falling_back_to_created_at() {
        let meta = test_session_metadata(
            "remembered-1",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );

        let info = remembered_session_info(
            &meta,
            &registry(),
            None,
            &Default::default(),
            &mut super::resolve_openable_plan_reference,
        );

        assert_eq!(info.created_at, "2026-06-28T09:00:00Z");
        assert_eq!(
            info.last_activity_at.as_deref(),
            Some("2026-06-28T09:05:00Z")
        );
    }

    /// A remembered session has no live handle to fall back to; a field
    /// `meta` doesn't carry must come out `None`, not silently substituted
    /// from a stray baseline value (issue #399).
    #[test]
    fn remembered_session_info_leaves_unset_meta_fields_absent() {
        let meta = test_session_metadata(
            "remembered-2",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );
        assert!(meta.summary.is_none());
        assert!(meta.repo_root.is_none());

        let info = remembered_session_info(
            &meta,
            &registry(),
            None,
            &Default::default(),
            &mut super::resolve_openable_plan_reference,
        );

        assert_eq!(info.summary, None);
        assert_eq!(info.repo_root, None);
        assert_eq!(info.memory_state, MemoryState::Remembered);
    }

    /// `meta.attention` is normalized to `None` whenever `lifecycle != "alive"`
    /// (`metadata::reconcile`), which every remembered session satisfies —
    /// so routing through the shared alive-only attention rule in
    /// `project_session_info` is a no-op for this path, not a behavior
    /// change (issue #399).
    #[test]
    fn remembered_session_info_never_reports_attention_for_a_dead_session() {
        let mut meta = test_session_metadata(
            "remembered-3",
            "Remembered",
            "/tmp/project",
            "ended",
            "2026-06-28T09:00:00Z",
            "2026-06-28T09:05:00Z",
        );
        meta.lifecycle = "dead".into();

        let info = remembered_session_info(
            &meta,
            &registry(),
            None,
            &Default::default(),
            &mut super::resolve_openable_plan_reference,
        );

        assert_eq!(info.attention, None);
    }
}
