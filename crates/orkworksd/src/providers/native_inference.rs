//! Code-owned native Taskmaster profiles; no user-JSON deserialization.

use crate::harness::{definition::DefinitionOrigin, store::HarnessSnapshot};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum NativeProfile {
    Codex,
    Claude,
    Ollama,
}

impl NativeProfile {
    pub(crate) fn resolve(snapshot: &HarnessSnapshot, id: &str) -> Option<Self> {
        let profile = match id {
            "codex" => Self::Codex,
            "claude-code" => Self::Claude,
            "ollama" if snapshot.registry.get(id).is_none() => return Some(Self::Ollama),
            _ => return None,
        };
        let harness = snapshot.registry.get(id)?;
        let patch = snapshot.stored_patches.get(id);
        if harness.definition.retired
            || harness.definition.inference.is_some()
            || patch.is_some_and(|patch| patch.inference.is_some())
        {
            return None;
        }
        match harness.origin {
            DefinitionOrigin::Builtin => Some(profile),
            DefinitionOrigin::Override
                if patch.is_some_and(|patch| patch.launch.is_none() && patch.peon.is_none()) =>
            {
                Some(profile)
            }
            _ => None,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude-code",
            Self::Ollama => "ollama",
        }
    }

    /// Fixed inference inputs, never projected from mutable Peon metadata.
    pub(super) fn definition(self) -> super::ProviderDefinition {
        let (label, command) = match self {
            Self::Codex => ("Codex", "codex"),
            Self::Claude => ("Claude Code", "claude"),
            Self::Ollama => ("Ollama", ""),
        };
        super::ProviderDefinition {
            id: self.id().into(),
            label: label.into(),
            command: command.into(),
            default_args: vec![],
            model_arg_template: None,
            supports_model: false,
            timeout_secs: 30,
            prompt_transport: super::PromptTransport::Stdin,
            reasoning_effort_args: vec![],
            list_models_command: None,
            list_models_args: vec![],
            static_models: vec![],
            http_list_models: self == Self::Ollama,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{
        definition::{BuiltinDocument, EMBEDDED_BUILTINS},
        store::HarnessStore,
    };
    use crate::providers::{
        InvocationResult, PeonSelection, ProviderManager, ProviderRunner, ProviderSettingsPayload,
    };
    use serde_json::json;
    use std::{process::Command, sync::Arc};

    #[test]
    fn native_profiles_require_builtin_origin_and_compatible_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let store = HarnessStore::new(
            dir.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        for (patch, allowed) in [
            (json!({}), true),
            (json!({"name":"Renamed"}), true),
            (json!({"peon":{"timeoutSecs":1}}), false),
            (json!({"launch":{"args":["--changed"]}}), false),
            (json!({"inference":null}), false),
            (
                json!({"inference":{"kind":"command","command":"wrapper","args":["{model}"],"input":"stdin","output":"result-json-v1"}}),
                false,
            ),
        ] {
            std::fs::write(
                dir.path().join("harnesses.json"),
                serde_json::to_vec(&json!({
                "version":3,"overrides":{"codex":patch},"custom":[{"id":"lookalike","name":"Alias",
                "launch":{"kind":"command-template","command":"codex","args":[]}}]}))
                .unwrap(),
            )
            .unwrap();
            let snapshot = store.snapshot().unwrap();
            assert_eq!(
                NativeProfile::resolve(&snapshot, "codex"),
                allowed.then_some(NativeProfile::Codex)
            );
            assert_eq!(NativeProfile::resolve(&snapshot, "lookalike"), None);
            assert_eq!(
                NativeProfile::resolve(&snapshot, "ollama"),
                Some(NativeProfile::Ollama)
            );
        }
    }

    #[test]
    fn native_profiles_do_not_require_a_peon_capability() {
        let dir = tempfile::tempdir().unwrap();
        let mut builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        for definition in &mut builtins.builtins {
            definition.peon = None;
        }
        let store = HarnessStore::new(dir.path().join("harnesses.json"), Arc::new(builtins));
        assert_eq!(
            NativeProfile::resolve(&store.snapshot().unwrap(), "claude-code"),
            Some(NativeProfile::Claude)
        );
    }

    #[test]
    fn retired_native_profiles_have_no_execution_binding() {
        let dir = tempfile::tempdir().unwrap();
        let mut builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        for definition in &mut builtins.builtins {
            definition.retired = true;
        }
        let store = HarnessStore::new(dir.path().join("harnesses.json"), Arc::new(builtins));
        let snapshot = store.snapshot().unwrap();
        for id in ["codex", "claude-code"] {
            assert_eq!(NativeProfile::resolve(&snapshot, id), None);
            let catalog = crate::taskmaster::provider_catalog::inspect(&store, None).unwrap();
            let entry = catalog.iter().find(|entry| entry.id == id).unwrap();
            assert_eq!(
                entry.state,
                crate::taskmaster::provider_catalog::Availability::Unavailable
            );
            assert_eq!(
                entry.transport,
                crate::taskmaster::provider_catalog::Transport::Unsupported
            );
        }
    }

    fn conflicting_peon_settings() -> ProviderSettingsPayload {
        ProviderSettingsPayload {
            peon_selection: Some(PeonSelection {
                provider: "peon-sentinel".into(),
                model: "peon-model-sentinel".into(),
                reasoning_effort: Some("peon-effort-sentinel".into()),
                ollama_base_url: Some("http://127.0.0.1:11437".into()),
            }),
            ollama_base_url: "http://127.0.0.1:11438".into(),
            ..ProviderSettingsPayload::default()
        }
    }

    struct NativeRunner;
    impl ProviderRunner for NativeRunner {
        fn run_with_connection(
            &self,
            id: &str,
            command: &str,
            args: &[String],
            prompt: &str,
            timeout: u64,
            model: Option<&str>,
            connection: Option<&str>,
        ) -> InvocationResult {
            assert_eq!(id, "ollama");
            assert!(command.is_empty());
            assert!(args.is_empty());
            assert_eq!(prompt, "fixture context");
            assert_eq!(timeout, 30);
            assert_eq!(model, Some("chosen-model"));
            assert_eq!(connection, Some("http://127.0.0.1:11436"));
            InvocationResult {
                success: true,
                stdout: r#"{"proposals":[]}"#.into(),
                stderr: String::new(),
            }
        }
        fn run(
            &self,
            _: &str,
            _: &str,
            _: &[String],
            _: &str,
            _: u64,
            _: Option<&str>,
        ) -> InvocationResult {
            panic!("native CLI must use its isolated prepared command");
        }
        fn run_prepared(
            &self,
            id: &str,
            command: &mut Command,
            prompt: &str,
            timeout: u64,
            _: Option<&str>,
        ) -> InvocationResult {
            assert_eq!(id, "claude-code");
            assert_eq!(command.get_program(), "claude");
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            let stdout = if args == ["--version"] {
                assert!(prompt.is_empty());
                assert_eq!(timeout, 5);
                "2.1.236 (Claude Code)"
            } else {
                assert_eq!(timeout, 30);
                assert!(args.contains(&"--safe-mode"));
                assert!(args.contains(&"--model=chosen-model"));
                assert!(!args.iter().any(|arg| arg.contains("peon-sentinel")));
                assert_eq!(prompt, "fixture context");
                r#"{"type":"result","subtype":"success","is_error":false,"result":"{\"proposals\":[]}"}"#
            };
            InvocationResult {
                success: true,
                stdout: stdout.into(),
                stderr: String::new(),
            }
        }
    }

    #[test]
    fn native_dispatch_ignores_mutated_peon_command_arguments_and_timeout() {
        let mut manager = ProviderManager::for_tests(conflicting_peon_settings(), vec![]);
        let mut definition = manager.definition("claude-code").unwrap();
        definition.command = "must-not-run-peon-sentinel".into();
        definition.default_args = vec!["--peon-sentinel".into()];
        definition.timeout_secs = 1;
        manager.harness_catalog = None;
        manager.registry = vec![definition];
        manager.runner = Arc::new(NativeRunner);
        let before = serde_json::to_value(&*manager.settings.read().unwrap()).unwrap();
        assert_eq!(
            manager
                .invoke_native_taskmaster_prompt(
                    NativeProfile::Claude,
                    "chosen-model",
                    None,
                    None,
                    "fixture context".into()
                )
                .unwrap(),
            r#"{"proposals":[]}"#
        );
        assert_eq!(
            serde_json::to_value(&*manager.settings.read().unwrap()).unwrap(),
            before
        );
        assert!(manager.runtime.read().unwrap().is_empty());
    }

    #[test]
    fn native_ollama_dispatch_preserves_explicit_endpoint_without_peon_registry() {
        let mut manager = ProviderManager::for_tests(conflicting_peon_settings(), vec![]);
        manager.harness_catalog = None;
        manager.registry.clear();
        manager.runner = Arc::new(NativeRunner);
        let before = serde_json::to_value(&*manager.settings.read().unwrap()).unwrap();
        assert_eq!(
            manager
                .invoke_native_taskmaster_prompt(
                    NativeProfile::Ollama,
                    "chosen-model",
                    None,
                    Some("http://127.0.0.1:11436"),
                    "fixture context".into(),
                )
                .unwrap(),
            r#"{"proposals":[]}"#
        );
        assert_eq!(
            serde_json::to_value(&*manager.settings.read().unwrap()).unwrap(),
            before
        );
        assert!(manager.runtime.read().unwrap().is_empty());
    }
}
