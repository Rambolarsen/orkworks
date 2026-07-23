use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use serde::Serialize;

use super::definition::{
    BuiltinDocument, DefinitionOrigin, HarnessDiagnostic, HarnessUserDocument,
};
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
    pub(crate) fn diagnostics(&self) -> &[HarnessDiagnostic] {
        &self.diagnostics
    }
    pub(crate) fn providers(&self) -> &[ProviderDefinition] {
        &self.providers
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
    if definition.voice.is_some() {
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
}
