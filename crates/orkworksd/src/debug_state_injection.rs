use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionStateInjectionId {
    ActiveFakeEnding,
    EndedStaleLiveAttention,
    EndedMissingFinalSnapshot,
    RunningBlocked,
    RunningIdleTooEarly,
    RunningCapped,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SessionStateInjectionOption {
    pub id: &'static str,
    pub label: &'static str,
}

impl SessionStateInjectionId {
    pub(crate) fn parse(id: &str) -> Option<Self> {
        match id {
            "active_fake_ending" => Some(Self::ActiveFakeEnding),
            "ended_stale_live_attention" => Some(Self::EndedStaleLiveAttention),
            "ended_missing_final_snapshot" => Some(Self::EndedMissingFinalSnapshot),
            "running_blocked" => Some(Self::RunningBlocked),
            "running_idle_too_early" => Some(Self::RunningIdleTooEarly),
            "running_capped" => Some(Self::RunningCapped),
            _ => None,
        }
    }

    pub(crate) fn options() -> Vec<SessionStateInjectionOption> {
        vec![
            SessionStateInjectionOption {
                id: "active_fake_ending",
                label: "Active -> fake ending",
            },
            SessionStateInjectionOption {
                id: "ended_stale_live_attention",
                label: "Ended -> stale live attention",
            },
            SessionStateInjectionOption {
                id: "ended_missing_final_snapshot",
                label: "Ended -> missing final snapshot",
            },
            SessionStateInjectionOption {
                id: "running_blocked",
                label: "Running -> blocked",
            },
            SessionStateInjectionOption {
                id: "running_idle_too_early",
                label: "Running -> idle too early",
            },
            SessionStateInjectionOption {
                id: "running_capped",
                label: "Running -> capped",
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_lists_all_curated_injections() {
        let ids: Vec<&'static str> = SessionStateInjectionId::options()
            .into_iter()
            .map(|option| option.id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "active_fake_ending",
                "ended_stale_live_attention",
                "ended_missing_final_snapshot",
                "running_blocked",
                "running_idle_too_early",
                "running_capped",
            ]
        );
    }

    #[test]
    fn parse_rejects_unknown_ids() {
        assert!(SessionStateInjectionId::parse("definitely_not_real").is_none());
    }
}
