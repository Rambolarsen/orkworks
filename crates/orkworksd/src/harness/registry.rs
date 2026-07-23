use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use serde::Serialize;

use super::definition::{
    BuiltinDocument, DefinitionOrigin, HarnessDiagnostic, HarnessUserDocument,
};
use super::definition::{CapacityCapability, LaunchCapability};
use super::definition::{HarnessDefinition, ModelCapability, PeonCapability};
use crate::providers::ProviderDefinition;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityName {
    Launch,
    ResumeExact,
    ResumeLatestCwd,
    ResumeLatestRepo,
    Models,
    Peon,
    Capacity,
    NativeSessionId,
    Attention,
    #[allow(dead_code)]
    Lifecycle,
    Voice,
    WorkspaceIntegration,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedHarness {
    pub definition: HarnessDefinition,
    pub origin: DefinitionOrigin,
    pub effective_capabilities: BTreeSet<CapabilityName>,
}

impl ResolvedHarness {
    pub(crate) fn build_launch(
        &self,
        cwd: &str,
        model: Option<&str>,
    ) -> crate::harness::CommandSpec {
        match &self.definition.launch {
            LaunchCapability::PlatformShell { .. } => crate::harness::default_shell_command(cwd),
            LaunchCapability::CommandTemplate {
                command,
                args,
                model_prefix,
            } => {
                let model = model
                    .map(|model| format!("{}{}", model_prefix.as_deref().unwrap_or(""), model));
                let mut rendered = Vec::with_capacity(args.len());
                for arg in args {
                    if arg.contains("{model}") && model.is_none() {
                        let _ = rendered.pop();
                    } else {
                        rendered.push(arg.replace("{model}", model.as_deref().unwrap_or_default()));
                    }
                }
                crate::harness::CommandSpec {
                    program: command.clone(),
                    args: rendered,
                    cwd: cwd.into(),
                }
            }
        }
    }

    pub(crate) fn build_resume(
        &self,
        strategy: crate::harness::ResumeStrategy,
        cwd: &str,
        harness_session_id: Option<&str>,
        repo_root: Option<&str>,
        model: Option<&str>,
    ) -> Option<crate::harness::CommandSpec> {
        let resume = self.definition.resume.as_ref()?;
        let template = match strategy {
            crate::harness::ResumeStrategy::Exact => resume.exact.as_ref()?,
            crate::harness::ResumeStrategy::LatestCwd => resume.latest_cwd.as_ref()?,
            crate::harness::ResumeStrategy::LatestRepo => resume.latest_repo.as_ref()?,
            crate::harness::ResumeStrategy::None => return None,
        };
        Some(crate::harness::render_command_template(
            template,
            cwd,
            repo_root,
            harness_session_id,
            model,
        ))
    }

    pub(crate) fn capacity_patterns(&self) -> &[String] {
        match self.definition.capacity.as_ref() {
            Some(CapacityCapability::TerminalPatterns { limit_patterns }) => limit_patterns,
            None => &[],
        }
    }

    pub(crate) fn select_resume_strategy(
        &self,
        memory: &crate::harness::ResumeMemory,
    ) -> crate::harness::ResumeStrategy {
        if memory.state != crate::harness::ResumeState::Available {
            return crate::harness::ResumeStrategy::None;
        }
        let Some(resume) = self.definition.resume.as_ref() else {
            return crate::harness::ResumeStrategy::None;
        };
        if resume.exact.is_some() && memory.harness_session_id.is_some() {
            crate::harness::ResumeStrategy::Exact
        } else if resume.latest_cwd.is_some() && memory.latest_fallback {
            crate::harness::ResumeStrategy::LatestCwd
        } else if resume.latest_repo.is_some() && memory.latest_fallback {
            crate::harness::ResumeStrategy::LatestRepo
        } else {
            crate::harness::ResumeStrategy::None
        }
    }

    pub(crate) fn resume_flags(&self) -> (bool, bool, bool) {
        let Some(resume) = self.definition.resume.as_ref() else {
            return (false, false, false);
        };
        (
            resume.exact.is_some(),
            resume.latest_cwd.is_some(),
            resume.latest_repo.is_some(),
        )
    }
}

pub(crate) struct ResolvedHarnessRegistry {
    ordered: Vec<ResolvedHarness>,
    by_id: HashMap<String, usize>,
    aliases: HashMap<String, String>,
    diagnostics: Vec<HarnessDiagnostic>,
    providers: Vec<ProviderDefinition>,
}

impl ResolvedHarnessRegistry {
    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        self.ordered
            .iter()
            .map(|harness| harness.definition.id.as_str())
    }
    pub(crate) fn get(&self, id: &str) -> Option<&ResolvedHarness> {
        let canonical = self.aliases.get(id).map(String::as_str).unwrap_or(id);
        self.by_id.get(canonical).map(|index| &self.ordered[*index])
    }
    pub(crate) fn providers(&self) -> &[ProviderDefinition] {
        &self.providers
    }
    #[allow(dead_code)]
    pub(crate) fn diagnostics(&self) -> &[HarnessDiagnostic] {
        &self.diagnostics
    }
    pub(crate) fn with_diagnostics(mut self, mut diagnostics: Vec<HarnessDiagnostic>) -> Self {
        self.diagnostics.append(&mut diagnostics);
        self
    }
}

pub(crate) type HarnessCatalog = Arc<RwLock<Arc<ResolvedHarnessRegistry>>>;

pub(crate) fn resolve_document(
    builtins: &BuiltinDocument,
    user: &HarnessUserDocument,
) -> Result<ResolvedHarnessRegistry, Vec<HarnessDiagnostic>> {
    if builtins.version != 2 || user.version != 2 {
        return Err(vec![HarnessDiagnostic {
            harness_id: None,
            code: "unsupported_document_version".into(),
            message: "Harness documents must use version 2.".into(),
        }]);
    }
    let mut diagnostics = Vec::new();
    let mut ordered = Vec::new();
    let mut ids = HashSet::new();
    for builtin in &builtins.builtins {
        if !ids.insert(builtin.id.clone()) {
            diagnostics.push(HarnessDiagnostic::for_id(
                &builtin.id,
                "duplicate_builtin",
                "Built-in ID is duplicated.",
            ));
            continue;
        }
        match builtin.validate(DefinitionOrigin::Builtin) {
            Ok(()) => ordered.push(ResolvedHarness {
                definition: builtin.clone(),
                origin: DefinitionOrigin::Builtin,
                effective_capabilities: capability_names(builtin),
            }),
            Err(errors) => diagnostics.extend(errors),
        }
    }
    for (id, patch) in &user.overrides {
        let Some(harness) = ordered
            .iter_mut()
            .find(|harness| harness.definition.id == *id)
        else {
            diagnostics.push(HarnessDiagnostic::for_id(
                id,
                "unknown_override",
                "Override does not match a built-in harness.",
            ));
            continue;
        };
        match harness.definition.apply_patch(patch) {
            Ok(definition) => {
                harness.effective_capabilities = capability_names(&definition);
                harness.definition = definition;
                harness.origin = DefinitionOrigin::Override;
            }
            Err(error) => diagnostics.push(error),
        }
    }
    for custom in &user.custom {
        if !ids.insert(custom.id.clone()) || custom.id == "gh-copilot" {
            diagnostics.push(HarnessDiagnostic::for_id(
                &custom.id,
                "custom_id_collision",
                "Custom ID collides with a built-in or compatibility alias.",
            ));
            continue;
        }
        match custom.validate(DefinitionOrigin::Custom) {
            Ok(()) => ordered.push(ResolvedHarness {
                definition: custom.clone(),
                origin: DefinitionOrigin::Custom,
                effective_capabilities: capability_names(custom),
            }),
            Err(errors) => diagnostics.extend(errors),
        }
    }
    let by_id = ordered
        .iter()
        .enumerate()
        .map(|(index, harness)| (harness.definition.id.clone(), index))
        .collect();
    let aliases = HashMap::from([("gh-copilot".to_owned(), "copilot".to_owned())]);
    let providers = ordered.iter().filter_map(provider_from_harness).collect();
    Ok(ResolvedHarnessRegistry {
        ordered,
        by_id,
        aliases,
        diagnostics,
        providers,
    })
}

fn capability_names(definition: &HarnessDefinition) -> BTreeSet<CapabilityName> {
    let mut names = BTreeSet::from([CapabilityName::Launch]);
    if let Some(resume) = &definition.resume {
        if resume.exact.is_some() {
            names.insert(CapabilityName::ResumeExact);
        }
        if resume.latest_cwd.is_some() {
            names.insert(CapabilityName::ResumeLatestCwd);
        }
        if resume.latest_repo.is_some() {
            names.insert(CapabilityName::ResumeLatestRepo);
        }
    }
    if definition.models.is_some() {
        names.insert(CapabilityName::Models);
    }
    if definition.peon.is_some() {
        names.insert(CapabilityName::Peon);
    }
    if definition.capacity.is_some() {
        names.insert(CapabilityName::Capacity);
    }
    if definition.voice.as_ref().is_some_and(|voice| {
        voice.native_voice
            || voice.requires_microphone_permission
            || voice.orkworks_dictation
            || voice.orkworks_voice_commands
    }) {
        names.insert(CapabilityName::Voice);
    }
    if let Some(binding) = &definition.session_signals {
        // Keep this aligned with the evidence register. Task 7 may only add a
        // signal after its exact payload fixture is implemented.
        match binding {
            super::definition::SessionSignalBinding::Claude
            | super::definition::SessionSignalBinding::Gemini
            | super::definition::SessionSignalBinding::Copilot => {
                names.insert(CapabilityName::NativeSessionId);
            }
            super::definition::SessionSignalBinding::Aider => {
                names.insert(CapabilityName::Attention);
            }
            super::definition::SessionSignalBinding::Codex
            | super::definition::SessionSignalBinding::OpenCode => {}
        }
    }
    if definition.integration.is_some() {
        names.insert(CapabilityName::WorkspaceIntegration);
    }
    names
}

fn provider_from_harness(harness: &ResolvedHarness) -> Option<ProviderDefinition> {
    let PeonCapability {
        command_override,
        args,
        model_arg_template,
        supports_model,
        timeout_secs,
    } = harness.definition.peon.as_ref()?.clone();
    let (list_models_command, list_models_args, static_models) = match &harness.definition.models {
        Some(ModelCapability::Static { models }) => (None, Vec::new(), models.clone()),
        Some(ModelCapability::Command { command, args }) => {
            (Some(command.clone()), args.clone(), Vec::new())
        }
        Some(ModelCapability::Http) => (None, Vec::new(), Vec::new()),
        None => (None, Vec::new(), Vec::new()),
    };
    let command = command_override.unwrap_or_else(|| match &harness.definition.launch {
        super::definition::LaunchCapability::CommandTemplate { command, .. } => command.clone(),
        super::definition::LaunchCapability::PlatformShell { .. } => String::new(),
    });
    Some(ProviderDefinition {
        id: harness.definition.id.clone(),
        label: harness.definition.name.clone(),
        command,
        default_args: args,
        model_arg_template,
        supports_model,
        timeout_secs,
        list_models_command,
        list_models_args,
        static_models,
        http_list_models: matches!(harness.definition.models, Some(ModelCapability::Http)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{BuiltinDocument, HarnessPatch, EMBEDDED_BUILTINS};

    #[test]
    fn invalid_override_keeps_the_builtin_and_reports_a_diagnostic() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let mut user = HarnessUserDocument::default();
        user.overrides.insert(
            "codex".into(),
            serde_json::from_str::<HarnessPatch>(r#"{"launch":{"kind":"platform-shell"}}"#)
                .unwrap(),
        );

        let resolved = resolve_document(&builtins, &user).unwrap();

        let super::super::definition::LaunchCapability::CommandTemplate { command, .. } =
            &resolved.get("codex").unwrap().definition.launch
        else {
            panic!("invalid override must retain codex built-in");
        };
        assert_eq!(command, "codex");
        assert_eq!(
            resolved.diagnostics()[0].code,
            "launch_kind_replace_required"
        );
    }

    #[test]
    fn custom_ids_cannot_collide_with_builtin_or_aliases() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let codex = builtins
            .builtins
            .iter()
            .find(|definition| definition.id == "codex")
            .unwrap();
        let mut user = HarnessUserDocument::default();
        let mut builtin_collision = codex.clone();
        builtin_collision.name = "Another Codex".into();
        let mut alias_collision = codex.clone();
        alias_collision.id = "gh-copilot".into();
        user.custom = vec![builtin_collision, alias_collision];

        let resolved = resolve_document(&builtins, &user).unwrap();

        assert_eq!(resolved.ids().filter(|id| *id == "codex").count(), 1);
        assert!(resolved
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code == "custom_id_collision"));
    }

    #[test]
    fn signal_capabilities_follow_the_conservative_contract_evidence() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
        let codex = &registry.get("codex").unwrap().effective_capabilities;
        assert!(!codex.contains(&CapabilityName::NativeSessionId));
        assert!(!codex.contains(&CapabilityName::Attention));
        assert!(!codex.contains(&CapabilityName::Lifecycle));
        let aider = &registry.get("aider").unwrap().effective_capabilities;
        assert!(aider.contains(&CapabilityName::Attention));
        assert!(!aider.contains(&CapabilityName::NativeSessionId));
        let claude = &registry.get("claude-code").unwrap().effective_capabilities;
        assert!(claude.contains(&CapabilityName::NativeSessionId));
        assert!(!claude.contains(&CapabilityName::Attention));
        assert!(!claude.contains(&CapabilityName::Lifecycle));
    }

    #[test]
    fn false_voice_flags_do_not_advertise_voice_support() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        assert!(builtins
            .builtins
            .iter()
            .find(|definition| definition.id == "claude-code")
            .expect("Claude Code builtin")
            .voice
            .is_none());

        let mut definition = builtins
            .builtins
            .iter()
            .find(|definition| definition.id == "codex")
            .expect("Codex builtin")
            .clone();
        definition.voice = Some(super::super::definition::VoiceCapability {
            native_voice: false,
            requires_microphone_permission: false,
            orkworks_dictation: false,
            orkworks_voice_commands: false,
        });

        assert!(!capability_names(&definition).contains(&CapabilityName::Voice));
    }

    #[test]
    fn opencode_launch_and_resume_share_one_definition() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
        let harness = registry.get("opencode").unwrap();

        let launch = harness.build_launch("/repo", Some("qwen3"));
        let resume = harness
            .build_resume(
                crate::harness::ResumeStrategy::Exact,
                "/repo",
                Some("ses_1"),
                None,
                None,
            )
            .unwrap();

        assert_eq!(launch.program, "opencode");
        assert_eq!(launch.args, ["--model", "ollama/qwen3"]);
        assert_eq!(resume.program, "opencode");
        assert_eq!(resume.args, ["--session", "ses_1"]);
    }

    #[test]
    fn builtin_launch_without_model_drops_model_flag_and_capacity_is_declarative() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
        let opencode = registry.get("opencode").unwrap();
        let claude = registry.get("claude-code").unwrap();

        assert_eq!(
            opencode.build_launch("/repo", None).args,
            Vec::<String>::new()
        );
        assert_eq!(
            claude.capacity_patterns(),
            ["you've hit your session limit"]
        );
    }
}
