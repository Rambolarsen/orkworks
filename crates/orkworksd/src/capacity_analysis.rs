//! Pure per-runtime capacity evidence. Its mutex is also fail-fast projection admission.
use crate::capacity_state::{CapacityObservation, CapacityState};
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(crate) struct CapacityInput {
    pub id: String,
    pub identity: RuntimeIdentity,
    pub sender: tokio::sync::mpsc::Sender<RuntimeCommand>,
    pub fingerprint: Option<Vec<u8>>,
    pub patterns: Option<Vec<String>>,
    pub state: CapacityState,
    pub lines: Vec<(u64, Arc<str>)>,
    pub raw: String,
    pub output_at: Option<String>,
}

pub(crate) struct PreparedCapacity {
    pub input: CapacityInput,
    pub observation: Option<CapacityObservation>,
}

#[derive(Default)]
struct LineCache {
    generation: u64,
    fingerprint: Option<Vec<u8>>,
    lines: HashMap<u64, (bool, Option<String>)>,
    raw: Option<(String, (bool, Option<String>))>,
}

#[derive(Default)]
pub(crate) struct CapacityAnalysisCache {
    sessions: HashMap<String, LineCache>,
}

impl CapacityAnalysisCache {
    pub(crate) fn analyze(
        &mut self,
        inputs: Vec<CapacityInput>,
    ) -> HashMap<String, PreparedCapacity> {
        self.analyze_with(inputs, crate::peon::capacity_line_evidence)
    }

    fn analyze_with(
        &mut self,
        inputs: Vec<CapacityInput>,
        mut scan: impl FnMut(&[String], &str) -> (bool, Option<String>),
    ) -> HashMap<String, PreparedCapacity> {
        let ids: HashSet<_> = inputs.iter().map(|input| input.id.as_str()).collect();
        self.sessions.retain(|id, _| ids.contains(id.as_str()));
        inputs
            .into_iter()
            .map(|input| {
                let cache = self.sessions.entry(input.id.clone()).or_default();
                if cache.generation != input.identity.run_generation
                    || cache.fingerprint != input.fingerprint
                {
                    *cache = LineCache {
                        generation: input.identity.run_generation,
                        fingerprint: input.fingerprint.clone(),
                        ..Default::default()
                    };
                }
                let retained: HashSet<_> =
                    input.lines.iter().map(|(revision, _)| *revision).collect();
                cache
                    .lines
                    .retain(|revision, _| retained.contains(revision));
                let observation = input.patterns.as_ref().map(|patterns| {
                    input
                        .state
                        .observe_with(input.lines.len(), &input.raw, |range, raw| {
                            if patterns.is_empty() {
                                return (false, None);
                            }
                            let mut detected = false;
                            let mut hint = None;
                            for (revision, text) in &input.lines[range] {
                                let evidence = cache
                                    .lines
                                    .entry(*revision)
                                    .or_insert_with(|| scan(patterns, text));
                                detected |= evidence.0;
                                if evidence.1.is_some() {
                                    hint = evidence.1.clone();
                                }
                            }
                            if raw.is_empty() {
                                return (detected, hint);
                            }
                            if cache.raw.as_ref().is_none_or(|(text, _)| text != raw) {
                                cache.raw = Some((raw.to_string(), scan(patterns, raw)));
                            }
                            let evidence = &cache.raw.as_ref().unwrap().1;
                            (detected || evidence.0, hint.or_else(|| evidence.1.clone()))
                        })
                });
                (input.id.clone(), PreparedCapacity { input, observation })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(generation: u64, lines: Vec<(u64, Arc<str>)>) -> CapacityInput {
        let (sender, _) = tokio::sync::mpsc::channel(1);
        CapacityInput {
            id: "cache".into(),
            identity: RuntimeIdentity {
                runtime_instance_id: "cache".into(),
                run_generation: generation,
            },
            sender,
            fingerprint: Some(vec![1]),
            patterns: Some(vec!["usage limit".into()]),
            state: CapacityState::default(),
            lines,
            raw: String::new(),
            output_at: None,
        }
    }

    #[test]
    fn unchanged_lines_are_not_normalized_again_during_continuous_output() {
        let mut cache = CapacityAnalysisCache::default();
        let retained: Arc<str> = Arc::from("ordinary output".repeat(9000));
        let mut calls = 0;
        let first = cache.analyze_with(
            vec![input(
                1,
                vec![
                    (1, retained.clone()),
                    (2, Arc::from("usage limit resets in 2h")),
                ],
            )],
            |patterns, text| {
                calls += 1;
                crate::peon::capacity_line_evidence(patterns, text)
            },
        );
        assert!(first["cache"].observation.as_ref().unwrap().at_usage_limit);
        assert_eq!(
            first["cache"]
                .observation
                .as_ref()
                .unwrap()
                .reset_hint
                .as_deref(),
            Some("resets in 2h")
        );
        let initial_calls = calls;
        cache.analyze_with(
            vec![input(
                1,
                vec![
                    (1, retained.clone()),
                    (2, Arc::from("usage limit resets in 2h")),
                    (3, Arc::from("new output")),
                ],
            )],
            |patterns, text| {
                calls += 1;
                crate::peon::capacity_line_evidence(patterns, text)
            },
        );
        assert_eq!(
            calls,
            initial_calls + 1,
            "only the new line may be normalized"
        );
        let previous = calls;
        let mut changed = input(1, vec![(1, retained.clone())]);
        changed.fingerprint = Some(vec![2]);
        changed.patterns = Some(vec!["another limit".into()]);
        let result = cache.analyze_with(vec![changed], |patterns, text| {
            calls += 1;
            crate::peon::capacity_line_evidence(patterns, text)
        });
        assert!(!result["cache"].observation.as_ref().unwrap().at_usage_limit);
        assert!(
            calls > previous,
            "capability changes must invalidate evidence"
        );
        let previous = calls;
        cache.analyze_with(vec![input(2, vec![(1, retained)])], |patterns, text| {
            calls += 1;
            crate::peon::capacity_line_evidence(patterns, text)
        });
        assert!(
            calls > previous,
            "a resumed generation cannot reuse old line identities"
        );
    }
}
