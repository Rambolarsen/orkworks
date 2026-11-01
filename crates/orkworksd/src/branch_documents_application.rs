//! Application-owned branch document reads, pin reset and approved handoff.
use super::SessionApplication;
use crate::{
    branch_documents::{self, Document, DocumentKind},
    AppState,
};
use std::{path::PathBuf, sync::Arc};

#[cfg(test)]
#[path = "branch_documents_application_tests.rs"]
mod tests;
pub(crate) struct Subject {
    pub root: PathBuf,
    pub store: Arc<crate::branch_documents_store::Store>,
    pub legacy: Option<crate::metadata::PlanReference>,
    pub launch: PathBuf,
    branch: Option<branch_documents::Identity>,
    head: Option<git2::Oid>,
    bound: bool,
}
pub(crate) fn subject(state: &AppState, id: &str) -> Result<Subject, String> {
    let (store, meta) = {
        let ws = state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable.")?;
        let ws = ws.as_ref().ok_or("Workspace unavailable.")?;
        (
            ws.metadata.branch_documents.clone(),
            ws.metadata.read_session(id).ok_or("Session unavailable.")?,
        )
    };
    let (root, _) = crate::session_location::subject(state, id)?;
    if meta.worktree_binding.is_none() {
        crate::session_location::WorktreeBinding::resolve(
            &meta.cwd,
            root.to_str().ok_or("Unsupported worktree path.")?,
        )
        .map_err(str::to_owned)?;
    }
    let branch = branch_documents::identity(&root).ok();
    let head = git2::Repository::open(&root)
        .ok()
        .and_then(|repo| repo.head().ok().and_then(|h| h.target()));
    Ok(Subject {
        root,
        store,
        legacy: meta.plan_path,
        launch: meta.cwd.into(),
        branch,
        head,
        bound: meta.worktree_binding.is_some(),
    })
}
fn current(state: &AppState, id: &str, old: &Subject) -> bool {
    subject(state, id).is_ok_and(|now| {
        now.root == old.root
            && now.branch == old.branch
            && now.head == old.head
            && now.bound == old.bound
            && Arc::ptr_eq(&now.store, &old.store)
    })
}
pub(crate) fn selected(
    state: &AppState,
    id: &str,
    kind: Option<DocumentKind>,
    selection_id: Option<&str>,
) -> Result<(Subject, Document), String> {
    let context = subject(state, id)?;
    if let (Some(kind), Some(expected)) = (kind, selection_id) {
        if let Some(doc) = explicit_selection(&context, kind, expected) {
            if !current(state, id, &context) {
                return Err("Session workspace changed; refresh.".into());
            }
            return Ok((context, doc));
        }
    }
    let scan = context.store.scan(&context.root, true);
    if !current(state, id, &context) {
        return Err("Session workspace changed; refresh.".into());
    }
    let resolved = {
        let workspace = state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable.")?;
        if !workspace
            .as_ref()
            .is_some_and(|ws| Arc::ptr_eq(&context.store, &ws.metadata.branch_documents))
        {
            return Err("Session workspace changed; refresh.".into());
        }
        branch_documents::resolve_from_scan(&context.store, &context.root, scan)
    };
    let mut result = None;
    if let Ok((identity, documents, _)) = &resolved {
        for wanted in kind
            .map(|k| vec![k])
            .unwrap_or_else(|| vec![DocumentKind::Implementation, DocumentKind::Design])
        {
            if let Some(doc) = documents.iter().find(|d| d.kind == wanted) {
                // A missing pinned selection is authoritative; never substitute
                // another document or another slot for an explicit request.
                if let Err(error) = branch_documents::validate_document(identity, doc) {
                    if kind.is_some() {
                        return Err(error);
                    }
                    continue;
                }
                result = Some(doc.clone());
                break;
            }
        }
    }
    if result.is_none() && (kind.is_none() || context.branch.is_none()) {
        if let Some(reference) = &context.legacy {
            let legacy_kind = branch_documents::kind_for_path(&reference.relative_path);
            if legacy_kind.is_some() && (kind.is_none() || kind == legacy_kind) {
                let path = crate::plan_handoff::resolve_openable_plan_reference(
                    &context.launch,
                    reference,
                )?;
                let root = reference
                    .worktree_root
                    .as_deref()
                    .map(PathBuf::from)
                    .unwrap_or(context.launch.clone());
                if context.bound
                    && root.canonicalize().map_err(|_| "Document unavailable.")? != context.root
                {
                    return Err("Document is outside the bound worktree.".into());
                }
                // Legacy session-local fallback is readable, never a shared write.
                if path.starts_with(&root.canonicalize().map_err(|_| "Document unavailable.")?) {
                    result = Some(branch_documents::document(
                        &root,
                        &reference.relative_path,
                        reference.source,
                        reference.source == crate::metadata::PlanSource::UserSelected,
                    )?);
                }
            }
        }
    }
    if !current(state, id, &context) {
        return Err("Session workspace changed; refresh.".into());
    }
    if let Ok((before, _, _)) = resolved {
        if branch_documents::identity(&context.root)? != before {
            return Err("Branch changed; refresh.".into());
        }
    }
    Ok((
        context,
        result.ok_or("No available document for this step.")?,
    ))
}

fn explicit_selection(context: &Subject, kind: DocumentKind, expected: &str) -> Option<Document> {
    let reference = context.legacy.as_ref()?;
    if reference.source != crate::metadata::PlanSource::UserSelected {
        return None;
    }
    let root = PathBuf::from(reference.worktree_root.as_deref()?);
    if context.bound && root.canonicalize().ok()? != context.root {
        return None;
    }
    crate::plan_handoff::resolve_openable_plan_reference(&context.root, reference).ok()?;
    let doc =
        branch_documents::document(&root, &reference.relative_path, reference.source, true).ok()?;
    let branch = branch_documents::identity(&root).ok();
    if doc.kind != kind || doc.selection_id(branch.as_ref()) != expected {
        return None;
    }
    if let Some(branch) = &branch {
        context
            .store
            .with_current_document(branch, &doc, &mut || Ok(()))
            .ok()?;
    }
    Some(doc)
}

impl SessionApplication {
    pub(crate) fn read_document_content(
        &self,
        id: &str,
        kind: Option<DocumentKind>,
        selection_id: Option<&str>,
    ) -> Result<String, String> {
        if kind.is_none() {
            if let Ok(content) = self.read_plan_content(id) {
                return Ok(content);
            }
        }
        let (context, mut doc) = selected(&self.state, id, kind, selection_id)?;
        let root = PathBuf::from(
            doc.reference
                .worktree_root
                .as_deref()
                .ok_or("Document anchor unavailable.")?,
        );
        let content = branch_documents::read_document(&root, &doc.reference.relative_path)?;
        doc.content_hash = branch_documents::content_hash(&content);
        let document_branch = branch_documents::identity(&root).ok();
        if selection_id
            .is_some_and(|expected| expected != doc.selection_id(document_branch.as_ref()))
        {
            return Err("Document selection changed; refresh.".into());
        }
        if !current(&self.state, id, &context) {
            return Err("Session workspace changed; refresh.".into());
        }
        #[cfg(test)]
        if let Some(hook) = context.store.before_content_return.lock().unwrap().take() {
            hook();
        }
        let workspace = self
            .state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable.")?;
        let ws = workspace.as_ref().ok_or("Workspace unavailable.")?;
        if !Arc::ptr_eq(&context.store, &ws.metadata.branch_documents) {
            return Err("Session workspace changed; refresh.".into());
        }
        let meta = ws.metadata.read_session(id).ok_or("Session unavailable.")?;
        if meta.cwd != context.launch.to_string_lossy()
            || meta.worktree_binding.is_some() != context.bound
            || meta
                .worktree_binding
                .as_ref()
                .is_some_and(|binding| binding.validate().ok().as_ref() != Some(&context.root))
        {
            return Err("Session workspace changed; refresh.".into());
        }
        if let Some(branch) = &document_branch {
            context
                .store
                .with_current_document(branch, &doc, &mut || Ok(()))
                .map_err(|_| "Document selection changed; refresh.")?;
        } else if meta.plan_path != context.legacy {
            return Err("Document selection changed; refresh.".into());
        }
        Ok(content)
    }
    pub(crate) fn reset_document_pin(
        &self,
        id: &str,
        kind: DocumentKind,
        selection_id: Option<&str>,
    ) -> Result<(), String> {
        let context = subject(&self.state, id)?;
        let key = branch_documents::identity(&context.root)?;
        let _fence = self
            .state
            .projection_lock
            .lock()
            .map_err(|_| "Workspace unavailable.")?;
        if !current(&self.state, id, &context) {
            return Err("Session workspace changed; refresh.".into());
        }
        let workspace = self
            .state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable.")?;
        if !workspace
            .as_ref()
            .is_some_and(|ws| Arc::ptr_eq(&context.store, &ws.metadata.branch_documents))
        {
            return Err("Session workspace changed; refresh.".into());
        }
        let meta = workspace
            .as_ref()
            .and_then(|ws| ws.metadata.read_session(id))
            .ok_or("Session unavailable.")?;
        if meta.cwd != context.launch.to_string_lossy()
            || meta.worktree_binding.is_some() != context.bound
            || meta
                .worktree_binding
                .as_ref()
                .is_some_and(|binding| binding.validate().ok().as_ref() != Some(&context.root))
        {
            return Err("Session workspace changed; refresh.".into());
        }
        context.store.use_latest(&key, kind, selection_id)
    }
    pub(crate) async fn request_document_review(
        &self,
        id: &str,
        kind: Option<DocumentKind>,
        selection_id: Option<&str>,
    ) -> Result<(), String> {
        let (identity, control_tx) = {
            let sessions = self.state.sessions.lock().unwrap();
            let handle = sessions.get(id).ok_or("Session runtime unavailable.")?;
            (handle.runtime.identity(), handle.runtime.control_tx.clone())
        };
        if kind.is_none() && self.read_plan_content(id).is_ok() {
            return self
                .request_plan_review_to_runtime(id, &identity, &control_tx)
                .await
                .map_err(|_| "Could not request plan review.".into());
        }
        let captured = self.state.clone();
        let target = id.to_owned();
        let expected = selection_id.map(str::to_owned);
        let (context, mut doc) = tokio::task::spawn_blocking(move || {
            selected(&captured, &target, kind, expected.as_deref())
        })
        .await
        .map_err(|_| "Document inspection failed.")??;
        let live = {
            let ws = self.state.workspace.lock().unwrap();
            ws.as_ref()
                .and_then(|ws| ws.metadata.read_session(id))
                .is_some_and(|meta| meta.lifecycle == "alive")
        };
        if !live || !current(&self.state, id, &context) {
            return Err("Session workspace changed; refresh.".into());
        }
        let root = PathBuf::from(
            doc.reference
                .worktree_root
                .as_deref()
                .ok_or("Document anchor unavailable.")?,
        );
        let path = root.join(&doc.reference.relative_path);
        let content = branch_documents::read_document(&root, &doc.reference.relative_path)?;
        doc.content_hash = branch_documents::content_hash(&content);
        let document_branch = branch_documents::identity(&root).ok();
        if selection_id
            .is_some_and(|expected| expected != doc.selection_id(document_branch.as_ref()))
        {
            return Err("Document selection changed; refresh.".into());
        }
        let prompt=format!("Please review the plan or specification at {}. Delegate this review to a subagent if you can; otherwise review it yourself. Check for missing requirements, risky assumptions, and unclear steps, then report the findings.\r",path.display());
        let checked_state = self.state.clone();
        let target = id.to_owned();
        let audit_store = context.store.clone();
        crate::runtime::terminal_runtime::submit_approved_input_to_runtime_with_admission(
            &self.state,
            id,
            &identity,
            &control_tx,
            prompt,
            move |send| {
                if !current(&checked_state, &target, &context) {
                    return Err(());
                }
                let workspace = checked_state.workspace.lock().map_err(|_| ())?;
                let ws = workspace.as_ref().ok_or(())?;
                if !Arc::ptr_eq(&context.store, &ws.metadata.branch_documents) {
                    return Err(());
                }
                let meta = ws.metadata.read_session(&target).ok_or(())?;
                if meta.lifecycle != "alive" || meta.cwd != context.launch.to_string_lossy() {
                    return Err(());
                }
                if let Some(binding) = &meta.worktree_binding {
                    if binding.validate().map_err(|_| ())? != context.root {
                        return Err(());
                    }
                }
                if let Some(branch) = &document_branch {
                    context.store.with_current_document(branch, &doc, send)
                } else {
                    // Detached/non-Git documents remain session-local. They have
                    // no shared record, but retain the same stale-content fence.
                    if meta.plan_path != context.legacy {
                        return Err(());
                    }
                    let content =
                        branch_documents::read_document(&root, &doc.reference.relative_path)
                            .map_err(|_| ())?;
                    if branch_documents::content_hash(&content) != doc.content_hash {
                        return Err(());
                    }
                    send()
                }
            },
        )
        .await
        .map_err(|_| "Could not deliver review request.")?;
        if let Some(ws) = self.state.workspace.lock().unwrap().as_ref() {
            if !Arc::ptr_eq(&audit_store, &ws.metadata.branch_documents) {
                return Ok(());
            }
            ws.metadata.append_event(
                id,
                &crate::metadata::Event {
                    event_type: "plan_review_requested".into(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    status: "working".into(),
                    observed_status: Some("working".into()),
                    confidence: None,
                    summary: Some("User requested document review.".into()),
                    source: Some("user".into()),
                    recommendation_id: None,
                },
            );
        }
        Ok(())
    }
}
