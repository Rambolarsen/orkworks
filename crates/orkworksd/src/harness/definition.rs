use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::CommandTemplate;

pub(crate) const EMBEDDED_BUILTINS: &str = include_str!("../../resources/harnesses-v2.json");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDefinition {
    pub id: String,
    pub name: String,
    pub launch: LaunchCapability,
    pub default_model: Option<String>,
    pub resume: Option<ResumeCapability>,
    pub models: Option<ModelCapability>,
    pub peon: Option<PeonCapability>,
    pub capacity: Option<CapacityCapability>,
    pub session_signals: Option<SessionSignalBinding>,
    pub integration: Option<IntegrationBinding>,
    pub voice: Option<VoiceCapability>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum LaunchCapability {
    CommandTemplate {
        command: String,
        args: Vec<String>,
        model_prefix: Option<String>,
    },
    PlatformShell {
        login: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum SessionSignalBinding {
    Claude,
    Codex,
    OpenCode,
    Gemini,
    Copilot,
    Aider,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum IntegrationBinding {
    Claude,
    Codex,
    OpenCode,
    Gemini,
    Copilot,
    Aider,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumeCapability {
    pub exact: Option<CommandTemplate>,
    pub latest_cwd: Option<CommandTemplate>,
    pub latest_repo: Option<CommandTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ModelCapability {
    Static { models: Vec<String> },
    Command { command: String, args: Vec<String> },
    Http,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeonCapability {
    pub command_override: Option<String>,
    pub args: Vec<String>,
    pub model_arg_template: Option<String>,
    pub supports_model: bool,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CapacityCapability {
    TerminalPatterns { limit_patterns: Vec<String> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoiceCapability {
    pub native_voice: bool,
    pub requires_microphone_permission: bool,
    pub orkworks_dictation: bool,
    pub orkworks_voice_commands: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinDocument {
    pub version: u32,
    pub builtins: Vec<HarnessDefinition>,
    #[serde(default)]
    pub legacy_snapshots: Vec<LegacyBuiltinSnapshot>,
}

impl BuiltinDocument {
    pub(crate) fn parse(bytes: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LegacyBuiltinSnapshot {
    pub schema_version: u32,
    pub harness_id: String,
    pub source: String,
    #[serde(default)]
    pub definition: Option<serde_json::Value>,
    #[serde(default)]
    pub environment_dependent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HarnessUserDocument {
    pub version: u32,
    #[serde(default)]
    pub overrides: BTreeMap<String, HarnessPatch>,
    #[serde(default)]
    pub custom: Vec<HarnessDefinition>,
}

impl Default for HarnessUserDocument {
    fn default() -> Self {
        Self {
            version: 2,
            overrides: BTreeMap::new(),
            custom: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessPatch {
    pub name: Option<String>,
    pub launch: Option<LaunchPatch>,
    pub default_model: Option<Option<String>>,
    pub resume: Option<Option<ResumePatch>>,
    pub models: Option<Option<ModelCapability>>,
    pub peon: Option<Option<PeonPatch>>,
    pub capacity: Option<Option<CapacityCapability>>,
    pub session_signals: Option<Option<SessionSignalBinding>>,
    pub integration: Option<Option<IntegrationBinding>>,
    pub voice: Option<Option<VoicePatch>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchPatch {
    pub kind: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub model_prefix: Option<Option<String>>,
    pub login: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumePatch {
    pub exact: Option<Option<CommandTemplate>>,
    pub latest_cwd: Option<Option<CommandTemplate>>,
    pub latest_repo: Option<Option<CommandTemplate>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeonPatch {
    pub command_override: Option<Option<String>>,
    pub args: Option<Vec<String>>,
    pub model_arg_template: Option<Option<String>>,
    pub supports_model: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoicePatch {
    pub native_voice: Option<bool>,
    pub requires_microphone_permission: Option<bool>,
    pub orkworks_dictation: Option<bool>,
    pub orkworks_voice_commands: Option<bool>,
}

impl<'de> Deserialize<'de> for HarnessPatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        reject_unknown_fields(
            &fields,
            &[
                "name",
                "launch",
                "defaultModel",
                "resume",
                "models",
                "peon",
                "capacity",
                "sessionSignals",
                "integration",
                "voice",
            ],
        )?;
        Ok(Self {
            name: required_patch_field(&fields, "name")?,
            launch: required_patch_field(&fields, "launch")?,
            default_model: optional_boundary_field(&fields, "defaultModel")?,
            resume: optional_boundary_field(&fields, "resume")?,
            models: optional_boundary_field(&fields, "models")?,
            peon: optional_boundary_field(&fields, "peon")?,
            capacity: optional_boundary_field(&fields, "capacity")?,
            session_signals: optional_boundary_field(&fields, "sessionSignals")?,
            integration: optional_boundary_field(&fields, "integration")?,
            voice: optional_boundary_field(&fields, "voice")?,
        })
    }
}

impl<'de> Deserialize<'de> for LaunchPatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        reject_unknown_fields(
            &fields,
            &["kind", "command", "args", "modelPrefix", "login"],
        )?;
        Ok(Self {
            kind: required_patch_field(&fields, "kind")?,
            command: required_patch_field(&fields, "command")?,
            args: required_patch_field(&fields, "args")?,
            model_prefix: optional_boundary_field(&fields, "modelPrefix")?,
            login: required_patch_field(&fields, "login")?,
        })
    }
}

impl<'de> Deserialize<'de> for ResumePatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        reject_unknown_fields(&fields, &["exact", "latestCwd", "latestRepo"])?;
        Ok(Self {
            exact: optional_boundary_field(&fields, "exact")?,
            latest_cwd: optional_boundary_field(&fields, "latestCwd")?,
            latest_repo: optional_boundary_field(&fields, "latestRepo")?,
        })
    }
}

impl<'de> Deserialize<'de> for PeonPatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        reject_unknown_fields(
            &fields,
            &[
                "commandOverride",
                "args",
                "modelArgTemplate",
                "supportsModel",
                "timeoutSecs",
            ],
        )?;
        Ok(Self {
            command_override: optional_boundary_field(&fields, "commandOverride")?,
            args: required_patch_field(&fields, "args")?,
            model_arg_template: optional_boundary_field(&fields, "modelArgTemplate")?,
            supports_model: required_patch_field(&fields, "supportsModel")?,
            timeout_secs: required_patch_field(&fields, "timeoutSecs")?,
        })
    }
}

impl<'de> Deserialize<'de> for VoicePatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let fields = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        reject_unknown_fields(
            &fields,
            &[
                "nativeVoice",
                "requiresMicrophonePermission",
                "orkworksDictation",
                "orkworksVoiceCommands",
            ],
        )?;
        Ok(Self {
            native_voice: required_patch_field(&fields, "nativeVoice")?,
            requires_microphone_permission: required_patch_field(
                &fields,
                "requiresMicrophonePermission",
            )?,
            orkworks_dictation: required_patch_field(&fields, "orkworksDictation")?,
            orkworks_voice_commands: required_patch_field(&fields, "orkworksVoiceCommands")?,
        })
    }
}

fn required_patch_field<T, E>(
    fields: &BTreeMap<String, serde_json::Value>,
    name: &str,
) -> Result<Option<T>, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::Error,
{
    let Some(value) = fields.get(name) else {
        return Ok(None);
    };
    if value.is_null() {
        return Err(E::custom(format!("{name} cannot be null")));
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(E::custom)
}

fn optional_boundary_field<T, E>(
    fields: &BTreeMap<String, serde_json::Value>,
    name: &str,
) -> Result<Option<Option<T>>, E>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::Error,
{
    let Some(value) = fields.get(name) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    serde_json::from_value(value.clone())
        .map(|value| Some(Some(value)))
        .map_err(E::custom)
}

fn reject_unknown_fields<E>(
    fields: &BTreeMap<String, serde_json::Value>,
    allowed: &[&str],
) -> Result<(), E>
where
    E: serde::de::Error,
{
    if let Some(field) = fields
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(E::custom(format!("unknown patch field {field}")));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefinitionOrigin {
    Builtin,
    Override,
    Custom,
}

impl HarnessDefinition {
    pub(crate) fn apply_patch(&self, patch: &HarnessPatch) -> Result<Self, HarnessDiagnostic> {
        let mut result = self.clone();
        if let Some(name) = &patch.name {
            result.name = name.clone();
        }
        if let Some(launch) = &patch.launch {
            if let Some(kind) = &launch.kind {
                let expected = match result.launch {
                    LaunchCapability::CommandTemplate { .. } => "command-template",
                    LaunchCapability::PlatformShell { .. } => "platform-shell",
                };
                if kind != expected {
                    result.launch = match kind.as_str() {
                        "command-template" => LaunchCapability::CommandTemplate {
                            command: launch.command.clone().ok_or_else(|| {
                                HarnessDiagnostic::for_id(
                                    &self.id,
                                    "launch_kind_replace_required",
                                    "Changing launch kind requires command and args.",
                                )
                            })?,
                            args: launch.args.clone().ok_or_else(|| {
                                HarnessDiagnostic::for_id(
                                    &self.id,
                                    "launch_kind_replace_required",
                                    "Changing launch kind requires command and args.",
                                )
                            })?,
                            model_prefix: launch.model_prefix.clone().unwrap_or(None),
                        },
                        "platform-shell" => LaunchCapability::PlatformShell {
                            login: launch.login.ok_or_else(|| {
                                HarnessDiagnostic::for_id(
                                    &self.id,
                                    "launch_kind_replace_required",
                                    "Changing launch kind requires login.",
                                )
                            })?,
                        },
                        _ => {
                            return Err(HarnessDiagnostic::for_id(
                                &self.id,
                                "unknown_launch_kind",
                                "Unknown launch kind.",
                            ))
                        }
                    };
                    result
                        .validate(DefinitionOrigin::Override)
                        .map_err(|mut errors| errors.remove(0))?;
                    return Ok(result);
                }
            }
            match &mut result.launch {
                LaunchCapability::CommandTemplate {
                    command,
                    args,
                    model_prefix,
                } => {
                    if let Some(value) = &launch.command {
                        *command = value.clone();
                    }
                    if let Some(value) = &launch.args {
                        *args = value.clone();
                    }
                    if let Some(value) = &launch.model_prefix {
                        *model_prefix = value.clone();
                    }
                }
                LaunchCapability::PlatformShell { .. }
                    if launch.command.is_some()
                        || launch.args.is_some()
                        || launch.model_prefix.is_some() =>
                {
                    return Err(HarnessDiagnostic::for_id(
                        &self.id,
                        "invalid_launch_patch",
                        "Platform-shell launch accepts no command fields.",
                    ))
                }
                LaunchCapability::PlatformShell { login } => {
                    if let Some(value) = launch.login {
                        *login = value;
                    }
                }
            }
        }
        if let Some(value) = &patch.default_model {
            result.default_model = value.clone();
        }
        if let Some(value) = &patch.resume {
            result.resume = value
                .as_ref()
                .map(|patch| patch_resume(result.resume.as_ref(), patch));
        }
        if let Some(value) = &patch.models {
            result.models = value.clone();
        }
        if let Some(value) = &patch.peon {
            result.peon = value
                .as_ref()
                .map(|patch| patch_peon(result.peon.as_ref(), patch));
        }
        if let Some(value) = &patch.capacity {
            result.capacity = value.clone();
        }
        if let Some(value) = &patch.session_signals {
            result.session_signals = value.clone();
        }
        if let Some(value) = &patch.integration {
            result.integration = value.clone();
        }
        if let Some(value) = &patch.voice {
            result.voice = value
                .as_ref()
                .map(|patch| patch_voice(result.voice.as_ref(), patch));
        }
        result
            .validate(DefinitionOrigin::Override)
            .map_err(|mut errors| errors.remove(0))?;
        Ok(result)
    }

    pub(crate) fn validate(&self, origin: DefinitionOrigin) -> Result<(), Vec<HarnessDiagnostic>> {
        let mut errors = Vec::new();
        if self.id.trim().is_empty()
            || !self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            errors.push(HarnessDiagnostic::for_id(
                &self.id,
                "invalid_id",
                "Harness ID must be lowercase kebab-case.",
            ));
        }
        if self.name.trim().is_empty() {
            errors.push(HarnessDiagnostic::for_id(
                &self.id,
                "invalid_name",
                "Harness name is required.",
            ));
        }
        if let LaunchCapability::CommandTemplate { command, args, .. } = &self.launch {
            if command.trim().is_empty() {
                errors.push(HarnessDiagnostic::for_id(
                    &self.id,
                    "invalid_command",
                    "Launch command is required.",
                ));
            }
            validate_templates(&self.id, args, &mut errors);
        }
        if let Some(resume) = &self.resume {
            for template in [&resume.exact, &resume.latest_cwd, &resume.latest_repo]
                .into_iter()
                .flatten()
            {
                if template.command.trim().is_empty() {
                    errors.push(HarnessDiagnostic::for_id(
                        &self.id,
                        "invalid_resume_command",
                        "Resume command is required.",
                    ));
                }
                validate_templates(&self.id, &template.args, &mut errors);
            }
        }
        if let Some(ModelCapability::Command { command, args }) = &self.models {
            if command.trim().is_empty() {
                errors.push(HarnessDiagnostic::for_id(
                    &self.id,
                    "invalid_model_command",
                    "Model command is required.",
                ));
            }
            validate_templates(&self.id, args, &mut errors);
        }
        if matches!(origin, DefinitionOrigin::Custom)
            && (self.integration.is_some() || self.session_signals.is_some())
        {
            errors.push(HarnessDiagnostic::for_id(
                &self.id,
                "custom_authority_binding",
                "Custom definitions cannot select compiled signal or integration bindings.",
            ));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn patch_resume(existing: Option<&ResumeCapability>, patch: &ResumePatch) -> ResumeCapability {
    let existing = existing.cloned().unwrap_or(ResumeCapability {
        exact: None,
        latest_cwd: None,
        latest_repo: None,
    });
    ResumeCapability {
        exact: patch.exact.clone().unwrap_or(existing.exact),
        latest_cwd: patch.latest_cwd.clone().unwrap_or(existing.latest_cwd),
        latest_repo: patch.latest_repo.clone().unwrap_or(existing.latest_repo),
    }
}

fn patch_peon(existing: Option<&PeonCapability>, patch: &PeonPatch) -> PeonCapability {
    let existing = existing.cloned().unwrap_or(PeonCapability {
        command_override: None,
        args: Vec::new(),
        model_arg_template: None,
        supports_model: false,
        timeout_secs: 30,
    });
    PeonCapability {
        command_override: patch
            .command_override
            .clone()
            .unwrap_or(existing.command_override),
        args: patch.args.clone().unwrap_or(existing.args),
        model_arg_template: patch
            .model_arg_template
            .clone()
            .unwrap_or(existing.model_arg_template),
        supports_model: patch.supports_model.unwrap_or(existing.supports_model),
        timeout_secs: patch.timeout_secs.unwrap_or(existing.timeout_secs),
    }
}

fn patch_voice(existing: Option<&VoiceCapability>, patch: &VoicePatch) -> VoiceCapability {
    let existing = existing.cloned().unwrap_or(VoiceCapability {
        native_voice: false,
        requires_microphone_permission: false,
        orkworks_dictation: false,
        orkworks_voice_commands: false,
    });
    VoiceCapability {
        native_voice: patch.native_voice.unwrap_or(existing.native_voice),
        requires_microphone_permission: patch
            .requires_microphone_permission
            .unwrap_or(existing.requires_microphone_permission),
        orkworks_dictation: patch
            .orkworks_dictation
            .unwrap_or(existing.orkworks_dictation),
        orkworks_voice_commands: patch
            .orkworks_voice_commands
            .unwrap_or(existing.orkworks_voice_commands),
    }
}

fn validate_templates(id: &str, values: &[String], errors: &mut Vec<HarnessDiagnostic>) {
    for value in values {
        for token in value.match_indices('{').map(|(index, _)| &value[index..]) {
            if !["{model}", "{cwd}", "{repoRoot}", "{harnessSessionId}"]
                .iter()
                .any(|allowed| token.starts_with(allowed))
            {
                errors.push(HarnessDiagnostic::for_id(
                    id,
                    "invalid_placeholder",
                    "Command templates use only {model}, {cwd}, {repoRoot}, or {harnessSessionId}.",
                ));
                break;
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDiagnostic {
    pub harness_id: Option<String>,
    pub code: String,
    pub message: String,
}

impl HarnessDiagnostic {
    pub(crate) fn for_id(id: &str, code: &str, message: &str) -> Self {
        Self {
            harness_id: Some(id.to_owned()),
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }
}

pub(crate) use super::registry::resolve_document;

#[cfg(test)]
mod tests {
    use super::*;

    fn codex() -> HarnessDefinition {
        BuiltinDocument::parse(EMBEDDED_BUILTINS)
            .unwrap()
            .builtins
            .into_iter()
            .find(|definition| definition.id == "codex")
            .unwrap()
    }

    #[test]
    fn embedded_builtins_are_complete_and_valid() {
        let document = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let resolved = resolve_document(&document, &HarnessUserDocument::default()).unwrap();
        assert_eq!(
            resolved.ids().collect::<Vec<_>>(),
            vec![
                "claude-code",
                "opencode",
                "codex",
                "gemini",
                "aider",
                "copilot",
                "generic-shell"
            ]
        );
        assert!(matches!(
            resolved.get("codex").unwrap().definition.integration,
            Some(IntegrationBinding::Codex)
        ));
        assert!(resolved
            .get("generic-shell")
            .unwrap()
            .definition
            .integration
            .is_none());
    }

    #[test]
    fn sparse_json_patch_preserves_omitted_builtin_fields() {
        let patch: HarnessPatch = serde_json::from_str(r#"{"name":"Configured Codex"}"#).unwrap();

        let original = codex();
        let patched = original.apply_patch(&patch).unwrap();

        assert_eq!(patched.name, "Configured Codex");
        assert_eq!(patched.launch, original.launch);
        assert_eq!(patched.peon, original.peon);
        assert_eq!(patched.capacity, original.capacity);
    }

    #[test]
    fn patch_arrays_replace_instead_of_append() {
        let patch: HarnessPatch =
            serde_json::from_str(r#"{"launch":{"args":["--sandbox","workspace-write"]}}"#).unwrap();

        let patched = codex().apply_patch(&patch).unwrap();
        let LaunchCapability::CommandTemplate { args, .. } = patched.launch else {
            panic!("codex uses a command template");
        };
        assert_eq!(args, ["--sandbox", "workspace-write"]);
    }

    #[test]
    fn null_removes_only_optional_capabilities() {
        let patch: HarnessPatch = serde_json::from_str(r#"{"capacity":null}"#).unwrap();
        assert!(codex().apply_patch(&patch).unwrap().capacity.is_none());

        assert!(serde_json::from_str::<HarnessPatch>(r#"{"name":null}"#).is_err());
    }

    #[test]
    fn scalar_patch_nulls_are_rejected_while_optional_boundaries_are_preserved() {
        for invalid in [
            r#"{"name":null}"#,
            r#"{"launch":null}"#,
            r#"{"launch":{"kind":null}}"#,
            r#"{"launch":{"command":null}}"#,
            r#"{"launch":{"args":null}}"#,
            r#"{"peon":{"args":null}}"#,
            r#"{"peon":{"supportsModel":null}}"#,
            r#"{"peon":{"timeoutSecs":null}}"#,
            r#"{"voice":{"nativeVoice":null}}"#,
            r#"{"voice":{"requiresMicrophonePermission":null}}"#,
            r#"{"voice":{"orkworksDictation":null}}"#,
            r#"{"voice":{"orkworksVoiceCommands":null}}"#,
        ] {
            assert!(
                serde_json::from_str::<HarnessPatch>(invalid).is_err(),
                "{invalid}"
            );
        }

        let omitted: HarnessPatch = serde_json::from_str("{}").unwrap();
        assert!(omitted.name.is_none());
        assert!(omitted.launch.is_none());

        let optional: HarnessPatch = serde_json::from_str(
            r#"{"defaultModel":null,"resume":null,"models":null,"peon":null,"capacity":null,"sessionSignals":null,"integration":null,"voice":null}"#,
        )
        .unwrap();
        assert_eq!(optional.default_model, Some(None));
        assert_eq!(optional.resume, Some(None));
        assert_eq!(optional.models, Some(None));
        assert_eq!(optional.peon, Some(None));
        assert_eq!(optional.capacity, Some(None));
        assert_eq!(optional.session_signals, Some(None));
        assert_eq!(optional.integration, Some(None));
        assert_eq!(optional.voice, Some(None));
    }

    #[test]
    fn unknown_binding_variant_is_rejected() {
        let invalid = r#"{"kind":"untrusted-handler"}"#;
        assert!(serde_json::from_str::<IntegrationBinding>(invalid).is_err());
    }

    #[test]
    fn custom_definitions_cannot_select_compiled_bindings() {
        let mut custom = codex();
        custom.id = "company-codex".into();
        assert!(custom.validate(DefinitionOrigin::Custom).is_err());
    }

    #[test]
    fn changing_launch_kind_requires_a_complete_and_valid_replacement() {
        let shell = BuiltinDocument::parse(EMBEDDED_BUILTINS)
            .unwrap()
            .builtins
            .into_iter()
            .find(|definition| definition.id == "generic-shell")
            .unwrap();
        let patch: HarnessPatch = serde_json::from_str(
            r#"{"launch":{"kind":"command-template","command":"fish","args":["-i"],"modelPrefix":null}}"#,
        )
        .unwrap();
        assert!(
            matches!(shell.apply_patch(&patch).unwrap().launch, LaunchCapability::CommandTemplate { ref command, .. } if command == "fish")
        );

        let incomplete: HarnessPatch =
            serde_json::from_str(r#"{"launch":{"kind":"command-template","command":"fish"}}"#)
                .unwrap();
        assert!(shell.apply_patch(&incomplete).is_err());
    }

    #[test]
    fn patch_deserialization_rejects_unknown_fields() {
        assert!(serde_json::from_str::<HarnessPatch>(r#"{"untrusted":true}"#).is_err());
        assert!(serde_json::from_str::<HarnessPatch>(
            r#"{"launch":{"command":"codex","unknown":true}}"#
        )
        .is_err());
    }
}
