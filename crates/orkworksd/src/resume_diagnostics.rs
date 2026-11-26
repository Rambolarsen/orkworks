//! Unified, authoritative resume availability and launch-time validation.
//!
//! Both session-list projection (chooser options and unavailable reasons) and
//! `SessionApplication::resume_session` (launch-time revalidation) derive from
//! `evaluate_strategy` so the UI never promises a strategy the backend rejects
//! and the backend never silently substitutes another strategy.

use crate::harness::registry::ResolvedHarness;
use crate::harness::{ResumeMemory, ResumeState, ResumeStrategy};
use crate::metadata::ResumeOption;
use crate::session_application::SessionError;
use crate::session_types::MemoryState;
use std::collections::HashSet;

pub(crate) const REASON_SESSION_RUNNING: &str = "Session is currently running";
pub(crate) const REASON_SESSION_ARCHIVED: &str =
    "Archived sessions must be restored before resuming";
pub(crate) const REASON_UNSUPPORTED_EXACT: &str = "Harness does not support exact resume";
pub(crate) const REASON_UNSUPPORTED_LATEST_CWD: &str =
    "Harness does not support folder-scoped resume";
pub(crate) const REASON_UNSUPPORTED_LATEST_REPO: &str =
    "Harness does not support repo-scoped resume";
pub(crate) const REASON_NO_REMEMBERED_SESSION: &str = "No compatible remembered session exists";
pub(crate) const REASON_NO_CAPTURED_ID: &str = "No harness session id was captured";
pub(crate) const REASON_CODEX_NO_CAPTURED_ID: &str = "Codex session has no captured native ID";
pub(crate) const REASON_CODEX_NO_SAVED_SESSION: &str =
    "Codex has no saved local session for this ID";

const ORDERED_STRATEGIES: [ResumeStrategy; 3] = [
    ResumeStrategy::Exact,
    ResumeStrategy::LatestCwd,
    ResumeStrategy::LatestRepo,
];

/// Returns `Some(id)` only when `id` is non-empty, has no leading/trailing
/// whitespace, and contains no control characters.
pub(crate) fn valid_native_session_id(id: Option<&str>) -> Option<&str> {
    id.filter(|value| {
        !value.is_empty() && value.trim() == *value && !value.chars().any(char::is_control)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnavailableKind {
    LifecycleConflict,
    BadRequest,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResumeUnavailable {
    pub(crate) kind: UnavailableKind,
    pub(crate) reason: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) struct ResumeContext<'a> {
    pub(crate) is_live: bool,
    pub(crate) is_archived: bool,
    pub(crate) harness: Option<&'a ResolvedHarness>,
    pub(crate) harness_id: Option<&'a str>,
    pub(crate) resume: Option<&'a ResumeMemory>,
    pub(crate) codex_saved_session_exists: Option<bool>,
}

impl<'a> ResumeContext<'a> {
    pub(crate) fn for_projection(
        is_live: bool,
        is_archived: bool,
        harness: Option<&'a ResolvedHarness>,
        harness_id: Option<&'a str>,
        resume: Option<&'a ResumeMemory>,
        saved_codex_session_ids: &HashSet<String>,
    ) -> Self {
        let is_codex =
            harness.is_some_and(|h| h.definition.id == "codex") || harness_id == Some("codex");
        let codex_saved_session_exists = if is_codex {
            valid_native_session_id(resume.and_then(|m| m.harness_session_id.as_deref()))
                .map(|native_id| saved_codex_session_ids.contains(native_id))
        } else {
            None
        };
        Self {
            is_live,
            is_archived,
            harness,
            harness_id,
            resume,
            codex_saved_session_exists,
        }
    }

    pub(crate) fn for_launch(
        is_live: bool,
        is_archived: bool,
        harness: Option<&'a ResolvedHarness>,
        harness_id: Option<&'a str>,
        resume: Option<&'a ResumeMemory>,
        codex_saved_session_exists: Option<bool>,
    ) -> Self {
        Self {
            is_live,
            is_archived,
            harness,
            harness_id,
            resume,
            codex_saved_session_exists,
        }
    }

    fn is_codex(&self) -> bool {
        self.harness.is_some_and(|h| h.definition.id == "codex") || self.harness_id == Some("codex")
    }

    fn resume_flags(&self) -> (bool, bool, bool) {
        self.harness
            .map(ResolvedHarness::resume_flags)
            .unwrap_or((false, false, false))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ResumeAssessment {
    pub(crate) memory_state: MemoryState,
    pub(crate) preferred_strategy: ResumeStrategy,
    pub(crate) options: Vec<ResumeOption>,
}

/// Evaluates whether a single concrete `strategy` is currently available under `ctx`.
pub(crate) fn evaluate_strategy(
    ctx: &ResumeContext<'_>,
    strategy: &ResumeStrategy,
) -> Result<(), ResumeUnavailable> {
    if ctx.is_live {
        return Err(ResumeUnavailable {
            kind: UnavailableKind::LifecycleConflict,
            reason: REASON_SESSION_RUNNING,
        });
    }
    if ctx.is_archived {
        return Err(ResumeUnavailable {
            kind: UnavailableKind::LifecycleConflict,
            reason: REASON_SESSION_ARCHIVED,
        });
    }

    let (supports_exact, supports_latest_cwd, supports_latest_repo) = ctx.resume_flags();
    let resume_available = ctx
        .resume
        .is_some_and(|memory| memory.state == ResumeState::Available);
    let valid_id = valid_native_session_id(
        ctx.resume
            .and_then(|memory| memory.harness_session_id.as_deref()),
    );

    match strategy {
        ResumeStrategy::Exact => {
            if !supports_exact {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_UNSUPPORTED_EXACT,
                });
            }
            if ctx.resume.is_none() {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_NO_REMEMBERED_SESSION,
                });
            }
            if ctx.is_codex() {
                if valid_id.is_none() {
                    return Err(ResumeUnavailable {
                        kind: UnavailableKind::BadRequest,
                        reason: REASON_CODEX_NO_CAPTURED_ID,
                    });
                }
                if ctx.codex_saved_session_exists != Some(true) {
                    return Err(ResumeUnavailable {
                        kind: UnavailableKind::BadRequest,
                        reason: REASON_CODEX_NO_SAVED_SESSION,
                    });
                }
            }
            if !resume_available {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_NO_REMEMBERED_SESSION,
                });
            }
            if valid_id.is_none() {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_NO_CAPTURED_ID,
                });
            }
            Ok(())
        }
        ResumeStrategy::LatestCwd => {
            if !supports_latest_cwd {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_UNSUPPORTED_LATEST_CWD,
                });
            }
            if !resume_available || !ctx.resume.is_some_and(|memory| memory.latest_fallback) {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_NO_REMEMBERED_SESSION,
                });
            }
            Ok(())
        }
        ResumeStrategy::LatestRepo => {
            if !supports_latest_repo {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_UNSUPPORTED_LATEST_REPO,
                });
            }
            if !resume_available || !ctx.resume.is_some_and(|memory| memory.latest_fallback) {
                return Err(ResumeUnavailable {
                    kind: UnavailableKind::Unavailable,
                    reason: REASON_NO_REMEMBERED_SESSION,
                });
            }
            Ok(())
        }
        ResumeStrategy::None => Err(ResumeUnavailable {
            kind: UnavailableKind::Unavailable,
            reason: REASON_NO_REMEMBERED_SESSION,
        }),
    }
}

fn strategy_label(strategy: &ResumeStrategy) -> &'static str {
    match strategy {
        ResumeStrategy::Exact => "Resume exact session",
        ResumeStrategy::LatestCwd => "Resume latest in folder",
        ResumeStrategy::LatestRepo => "Resume latest in repo",
        ResumeStrategy::None => "Resume unavailable",
    }
}

/// Derives the complete projection assessment (memory state, preferred strategy,
/// and chooser options with unavailable reasons) from `evaluate_strategy`.
pub(crate) fn derive_assessment(ctx: &ResumeContext<'_>) -> ResumeAssessment {
    let mut preferred_strategy = ResumeStrategy::None;
    let mut options = Vec::with_capacity(ORDERED_STRATEGIES.len());

    for strategy in &ORDERED_STRATEGIES {
        let outcome = evaluate_strategy(ctx, strategy);
        let available = outcome.is_ok();
        if available && preferred_strategy == ResumeStrategy::None {
            preferred_strategy = strategy.clone();
        }
        options.push(ResumeOption::new(
            strategy.clone(),
            strategy_label(strategy),
            available,
            outcome.err().map(|err| err.reason),
        ));
    }

    for option in &mut options {
        option.preferred = option.available && option.strategy == preferred_strategy;
    }

    let memory_state = if ctx.is_live {
        MemoryState::Live
    } else if ctx.resume.is_none() {
        MemoryState::Remembered
    } else if preferred_strategy == ResumeStrategy::None {
        MemoryState::Unsupported
    } else {
        MemoryState::Resumable
    };

    ResumeAssessment {
        memory_state,
        preferred_strategy,
        options,
    }
}

/// Authoritative launch-time strategy validation sharing `evaluate_strategy`
/// with chooser projection. Never silently falls back when an explicit strategy
/// is requested.
pub(crate) fn validate_launch(
    ctx: &ResumeContext<'_>,
    requested_strategy: Option<ResumeStrategy>,
) -> Result<ResumeStrategy, SessionError> {
    if ctx.is_live {
        return Err(SessionError::ConflictWithMessage(REASON_SESSION_RUNNING));
    }
    if ctx.is_archived {
        return Err(SessionError::ConflictWithMessage(REASON_SESSION_ARCHIVED));
    }

    if let Some(strategy) = requested_strategy {
        if strategy == ResumeStrategy::None {
            return Err(SessionError::BadRequest(REASON_NO_REMEMBERED_SESSION));
        }
        return match evaluate_strategy(ctx, &strategy) {
            Ok(()) => Ok(strategy),
            Err(unavailable) => match unavailable.kind {
                UnavailableKind::BadRequest => Err(SessionError::BadRequest(unavailable.reason)),
                UnavailableKind::LifecycleConflict | UnavailableKind::Unavailable => {
                    Err(SessionError::ConflictWithMessage(unavailable.reason))
                }
            },
        };
    }

    // Legacy bodyless resume request: select the highest-priority available
    // strategy without violating Codex exact-only guarantees.
    if ctx.is_codex() && ctx.resume.is_some() {
        if let Err(unavailable) = evaluate_strategy(ctx, &ResumeStrategy::Exact) {
            return match unavailable.kind {
                UnavailableKind::BadRequest => Err(SessionError::BadRequest(unavailable.reason)),
                UnavailableKind::LifecycleConflict => {
                    Err(SessionError::ConflictWithMessage(unavailable.reason))
                }
                UnavailableKind::Unavailable => Err(SessionError::BadRequest(unavailable.reason)),
            };
        }
        return Ok(ResumeStrategy::Exact);
    }

    let assessment = derive_assessment(ctx);
    if assessment.preferred_strategy != ResumeStrategy::None {
        return Ok(assessment.preferred_strategy);
    }

    Err(SessionError::EmptyBadRequest)
}

/// Applies the archived-session rule to already-projected resume fields once
/// organization state has been resolved.
pub(crate) fn mark_archived(
    memory_state: &mut MemoryState,
    resume_strategy: &mut ResumeStrategy,
    resume_options: &mut [ResumeOption],
) {
    if *memory_state == MemoryState::Resumable {
        *memory_state = MemoryState::Unsupported;
    }
    *resume_strategy = ResumeStrategy::None;
    for option in resume_options {
        option.available = false;
        option.preferred = false;
        option.reason = Some(REASON_SESSION_ARCHIVED.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{
        BuiltinDocument, HarnessUserDocument, ResumePatch, EMBEDDED_BUILTINS,
    };
    use crate::harness::registry::{resolve_document, ResolvedHarnessRegistry};
    use crate::harness::CommandTemplate;

    fn registry() -> ResolvedHarnessRegistry {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        resolve_document(&builtins, &HarnessUserDocument::default()).unwrap()
    }

    fn registry_with_opencode_latest_repo() -> ResolvedHarnessRegistry {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let mut user = HarnessUserDocument::default();
        user.overrides.entry("opencode".into()).or_default().resume = Some(Some(ResumePatch {
            latest_repo: Some(Some(CommandTemplate {
                command: "opencode".into(),
                args: vec!["--continue-repo".into()],
            })),
            ..Default::default()
        }));
        resolve_document(&builtins, &user).unwrap()
    }

    fn assert_projection_matches_launch(ctx: &ResumeContext<'_>) {
        let assessment = derive_assessment(ctx);
        for option in &assessment.options {
            let launch = validate_launch(ctx, Some(option.strategy.clone()));
            if option.available {
                assert_eq!(
                    launch,
                    Ok(option.strategy.clone()),
                    "chooser promised {:?} available, but launch rejected it",
                    option.strategy
                );
            } else {
                let err = launch.expect_err(&format!(
                    "chooser marked {:?} unavailable ({:?}), but launch accepted it",
                    option.strategy, option.reason
                ));
                let message = match err {
                    SessionError::BadRequest(msg) | SessionError::ConflictWithMessage(msg) => msg,
                    other => panic!(
                        "unexpected launch error variant for {:?}: {other:?}",
                        option.strategy
                    ),
                };
                assert_eq!(
                    option.reason.as_deref(),
                    Some(message),
                    "chooser reason and launch rejection diverged for {:?}",
                    option.strategy
                );
            }
        }
        let bodyless = validate_launch(ctx, None);
        if assessment.preferred_strategy == ResumeStrategy::None {
            assert!(
                bodyless.is_err(),
                "assessment had no preferred strategy, but bodyless launch succeeded"
            );
        } else {
            assert_eq!(
                bodyless,
                Ok(assessment.preferred_strategy.clone()),
                "bodyless launch must select the exact preferred strategy"
            );
        }
    }

    #[test]
    fn valid_native_session_id_rejects_blank_whitespace_and_control_chars() {
        assert_eq!(valid_native_session_id(None), None);
        assert_eq!(valid_native_session_id(Some("")), None);
        assert_eq!(valid_native_session_id(Some("   ")), None);
        assert_eq!(valid_native_session_id(Some(" ses_1")), None);
        assert_eq!(valid_native_session_id(Some("ses_1\n")), None);
        assert_eq!(valid_native_session_id(Some("ses\x001")), None);
        assert_eq!(
            valid_native_session_id(Some("ses_valid-123")),
            Some("ses_valid-123")
        );
    }

    #[test]
    fn chooser_and_launch_validation_agree_across_representative_contexts() {
        let reg = registry();
        let reg_repo = registry_with_opencode_latest_repo();
        let codex = reg.get("codex");
        let opencode = reg.get("opencode");
        let opencode_repo = reg_repo.get("opencode");
        let claude = reg.get("claude-code");
        let shell = reg.get("generic-shell");

        let with_id_and_fallback = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("ses_123".into()),
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        };
        let without_id_with_fallback = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        };
        let blank_id_with_fallback = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("   ".into()),
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        };
        let unavailable_memory = ResumeMemory {
            state: ResumeState::Unavailable,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("ses_123".into()),
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        };
        let no_fallback_no_id = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::None,
            harness_session_id: None,
            latest_fallback: false,
            last_seen_at: None,
        };

        let saved: HashSet<String> = ["ses_123".to_string()].into_iter().collect();
        let empty: HashSet<String> = HashSet::new();

        let cases = [
            ResumeContext::for_projection(
                false,
                false,
                codex,
                Some("codex"),
                Some(&with_id_and_fallback),
                &saved,
            ),
            ResumeContext::for_projection(
                false,
                false,
                codex,
                Some("codex"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                codex,
                Some("codex"),
                Some(&without_id_with_fallback),
                &saved,
            ),
            ResumeContext::for_projection(
                false,
                false,
                codex,
                Some("codex"),
                Some(&blank_id_with_fallback),
                &saved,
            ),
            ResumeContext::for_projection(
                false,
                false,
                opencode,
                Some("opencode"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                opencode,
                Some("opencode"),
                Some(&without_id_with_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                opencode,
                Some("opencode"),
                Some(&blank_id_with_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                opencode_repo,
                Some("opencode"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                claude,
                Some("claude-code"),
                Some(&no_fallback_no_id),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                claude,
                Some("claude-code"),
                Some(&unavailable_memory),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                shell,
                Some("generic-shell"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                false,
                None,
                Some("deleted-tool"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(false, false, opencode, Some("opencode"), None, &empty),
            ResumeContext::for_projection(
                true,
                false,
                opencode,
                Some("opencode"),
                Some(&with_id_and_fallback),
                &empty,
            ),
            ResumeContext::for_projection(
                false,
                true,
                opencode,
                Some("opencode"),
                Some(&with_id_and_fallback),
                &empty,
            ),
        ];

        for ctx in &cases {
            assert_projection_matches_launch(ctx);
        }
    }

    #[test]
    fn explicit_unavailable_strategy_never_silently_substitutes_available_fallback() {
        let reg = registry();
        let claude = reg.get("claude-code");
        let memory = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::LatestCwd,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: None,
        };
        let ctx = ResumeContext::for_launch(
            false,
            false,
            claude,
            Some("claude-code"),
            Some(&memory),
            None,
        );

        assert_eq!(
            validate_launch(&ctx, Some(ResumeStrategy::Exact)),
            Err(SessionError::ConflictWithMessage(REASON_NO_CAPTURED_ID))
        );
        assert_eq!(
            validate_launch(&ctx, Some(ResumeStrategy::LatestRepo)),
            Err(SessionError::ConflictWithMessage(
                REASON_UNSUPPORTED_LATEST_REPO
            ))
        );
        assert_eq!(
            validate_launch(&ctx, Some(ResumeStrategy::LatestCwd)),
            Ok(ResumeStrategy::LatestCwd)
        );
    }

    #[test]
    fn codex_unavailable_reasons_preserve_specific_identity_and_artifact_diagnostics() {
        let reg = registry();
        let codex = reg.get("codex");
        let with_id = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("thread-missing".into()),
            latest_fallback: true,
            last_seen_at: None,
        };
        let without_id = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: None,
            latest_fallback: true,
            last_seen_at: None,
        };
        let empty = HashSet::new();

        let missing_rollout = ResumeContext::for_projection(
            false,
            false,
            codex,
            Some("codex"),
            Some(&with_id),
            &empty,
        );
        let missing_rollout_assessment = derive_assessment(&missing_rollout);
        assert_eq!(
            missing_rollout_assessment.memory_state,
            MemoryState::Unsupported
        );
        assert_eq!(
            missing_rollout_assessment.options[0].reason.as_deref(),
            Some(REASON_CODEX_NO_SAVED_SESSION)
        );
        assert_eq!(
            validate_launch(&missing_rollout, Some(ResumeStrategy::Exact)),
            Err(SessionError::BadRequest(REASON_CODEX_NO_SAVED_SESSION))
        );

        let missing_id = ResumeContext::for_projection(
            false,
            false,
            codex,
            Some("codex"),
            Some(&without_id),
            &empty,
        );
        let missing_id_assessment = derive_assessment(&missing_id);
        assert_eq!(missing_id_assessment.memory_state, MemoryState::Unsupported);
        assert_eq!(
            missing_id_assessment.options[0].reason.as_deref(),
            Some(REASON_CODEX_NO_CAPTURED_ID)
        );
        assert_eq!(
            validate_launch(&missing_id, None),
            Err(SessionError::BadRequest(REASON_CODEX_NO_CAPTURED_ID))
        );

        let unverified_launch =
            ResumeContext::for_launch(false, false, codex, Some("codex"), Some(&with_id), None);
        assert_eq!(
            validate_launch(&unverified_launch, Some(ResumeStrategy::Exact)),
            Err(SessionError::BadRequest(REASON_CODEX_NO_SAVED_SESSION))
        );
    }

    #[test]
    fn mark_archived_matches_archived_assessment() {
        let reg = registry();
        let opencode = reg.get("opencode");
        let resume = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: Some("ses_123".into()),
            latest_fallback: true,
            last_seen_at: Some("now".into()),
        };
        let empty = HashSet::new();

        let mut two_phase = derive_assessment(&ResumeContext::for_projection(
            false,
            false,
            opencode,
            Some("opencode"),
            Some(&resume),
            &empty,
        ));
        mark_archived(
            &mut two_phase.memory_state,
            &mut two_phase.preferred_strategy,
            &mut two_phase.options,
        );
        let direct = derive_assessment(&ResumeContext::for_projection(
            false,
            true,
            opencode,
            Some("opencode"),
            Some(&resume),
            &empty,
        ));
        assert_eq!(two_phase, direct);
        assert_eq!(two_phase.memory_state, MemoryState::Unsupported);
    }

    #[test]
    fn disabled_entries_report_distinct_reasons_and_bodyless_returns_empty_bad_request() {
        let reg = registry();
        let claude = reg.get("claude-code");
        let resume = ResumeMemory {
            state: ResumeState::Available,
            preferred_strategy: ResumeStrategy::Exact,
            harness_session_id: None,
            latest_fallback: false,
            last_seen_at: None,
        };
        let empty = HashSet::new();
        let ctx = ResumeContext::for_projection(
            false,
            false,
            claude,
            Some("claude-code"),
            Some(&resume),
            &empty,
        );
        let assessment = derive_assessment(&ctx);
        assert_eq!(assessment.options.len(), 3);
        assert!(!assessment.options[0].available);
        assert_eq!(
            assessment.options[0].reason.as_deref(),
            Some(REASON_NO_CAPTURED_ID)
        );
        assert!(!assessment.options[1].available);
        assert_eq!(
            assessment.options[1].reason.as_deref(),
            Some(REASON_NO_REMEMBERED_SESSION)
        );
        assert!(!assessment.options[2].available);
        assert_eq!(
            assessment.options[2].reason.as_deref(),
            Some(REASON_UNSUPPORTED_LATEST_REPO)
        );
        assert_eq!(
            validate_launch(&ctx, None),
            Err(SessionError::EmptyBadRequest)
        );
    }
}
