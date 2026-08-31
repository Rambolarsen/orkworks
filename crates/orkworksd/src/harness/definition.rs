use std::collections::BTreeMap;

use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

use super::CommandTemplate;

pub(crate) const EMBEDDED_BUILTINS: &str = include_str!("../../resources/harnesses-v2.json");

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub retired: bool,
    pub launch: LaunchCapability,
    pub default_model: Option<String>,
    pub resume: Option<ResumeCapability>,
    pub models: Option<ModelCapability>,
    pub peon: Option<PeonCapability>,
    pub capacity: Option<CapacityCapability>,
    pub session_signals: Option<SessionSignalBinding>,
    pub integration: Option<IntegrationBinding>,
    pub voice: Option<VoiceCapability>,
    pub min_version: Option<VersionRequirement>,
    #[serde(default)]
    pub label_reset_commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
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
    #[serde(default)]
    pub prompt_transport: PromptTransport,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptTransport {
    #[default]
    Stdin,
    Argument,
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
pub(crate) struct VersionRequirement {
    pub min: (u64, u64, u64),
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessUserDocument {
    pub version: u32,
    #[serde(default)]
    pub overrides: BTreeMap<String, HarnessPatch>,
    #[serde(default)]
    pub custom: Vec<HarnessDefinition>,
    #[serde(default)]
    compatibility_profiles: BTreeMap<String, super::compatibility::CompatibilityProfile>,
}

impl Default for HarnessUserDocument {
    fn default() -> Self {
        Self {
            version: 3,
            overrides: BTreeMap::new(),
            custom: Vec::new(),
            compatibility_profiles: BTreeMap::new(),
        }
    }
}

impl HarnessUserDocument {
    /// Returns the sidecar-owned profile assigned to a custom harness.
    pub(crate) fn compatibility_profile(
        &self,
        id: &str,
    ) -> Option<super::compatibility::CompatibilityProfile> {
        self.compatibility_profiles.get(id).copied()
    }

    pub(crate) fn compatibility_profiles(
        &self,
    ) -> &BTreeMap<String, super::compatibility::CompatibilityProfile> {
        &self.compatibility_profiles
    }

    /// Assigns a profile through the sidecar-owned mutation boundary.
    pub(crate) fn set_compatibility_profile(
        &mut self,
        id: &str,
        profile: super::compatibility::CompatibilityProfile,
    ) -> Result<(), HarnessDiagnostic> {
        if !self.custom.iter().any(|definition| definition.id == id) {
            return Err(HarnessDiagnostic::for_id(
                id,
                "unknown_compatibility_profile_target",
                "Compatibility profile target must be a custom harness.",
            ));
        }
        self.compatibility_profiles.insert(id.to_owned(), profile);
        Ok(())
    }

    pub(crate) fn clear_compatibility_profiles(&mut self) {
        self.compatibility_profiles.clear();
    }

    /// Removes a custom definition and its sidecar-owned compatibility profile
    /// as one document mutation.
    pub(crate) fn remove_custom_definition(&mut self, id: &str) -> bool {
        let Some(position) = self.custom.iter().position(|custom| custom.id == id) else {
            return false;
        };
        self.custom.remove(position);
        self.compatibility_profiles.remove(id);
        true
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<Option<ResumePatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Option<ModelCapability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peon: Option<Option<PeonPatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<Option<CapacityCapability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_signals: Option<Option<SessionSignalBinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<Option<IntegrationBinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Option<VoicePatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<Option<VersionRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_reset_commands: Option<Option<Vec<String>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_prefix: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact: Option<Option<CommandTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_cwd: Option<Option<CommandTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_repo: Option<Option<CommandTemplate>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PeonPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_override: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_arg_template: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_model: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_transport: Option<PromptTransport>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VoicePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_voice: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_microphone_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orkworks_dictation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
                "voice",
                "minVersion",
                "labelResetCommands",
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
            min_version: optional_boundary_field(&fields, "minVersion")?,
            label_reset_commands: optional_boundary_field(&fields, "labelResetCommands")?,
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
                "promptTransport",
            ],
        )?;
        Ok(Self {
            command_override: optional_boundary_field(&fields, "commandOverride")?,
            args: required_patch_field(&fields, "args")?,
            model_arg_template: optional_boundary_field(&fields, "modelArgTemplate")?,
            supports_model: required_patch_field(&fields, "supportsModel")?,
            timeout_secs: required_patch_field(&fields, "timeoutSecs")?,
            prompt_transport: required_patch_field(&fields, "promptTransport")?,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CustomHarnessDefinition {
    id: String,
    name: String,
    #[serde(default)]
    retired: bool,
    launch: LaunchCapability,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    resume: Option<ResumeCapability>,
    #[serde(default)]
    models: Option<ModelCapability>,
    #[serde(default)]
    peon: Option<PeonCapability>,
    #[serde(default)]
    capacity: Option<CapacityCapability>,
    #[serde(default)]
    voice: Option<VoiceCapability>,
    #[serde(default)]
    min_version: Option<VersionRequirement>,
    #[serde(default)]
    label_reset_commands: Vec<String>,
}

impl From<CustomHarnessDefinition> for HarnessDefinition {
    fn from(value: CustomHarnessDefinition) -> Self {
        Self {
            id: value.id,
            name: value.name,
            retired: value.retired,
            launch: value.launch,
            default_model: value.default_model,
            resume: value.resume,
            models: value.models,
            peon: value.peon,
            capacity: value.capacity,
            session_signals: None,
            integration: None,
            voice: value.voice,
            min_version: value.min_version,
            label_reset_commands: value.label_reset_commands,
        }
    }
}

/// Parses an editable custom harness definition without exposing compiled bindings.
pub(crate) fn parse_custom_definition(
    bytes: &[u8],
) -> Result<HarnessDefinition, Vec<HarnessDiagnostic>> {
    let value = parse_strict_json::<serde_json::Value>(bytes, 256 * 1024)
        .map_err(|diagnostic| vec![diagnostic])?;
    parse_custom_definition_value(value)
}

fn parse_custom_definition_value(
    value: serde_json::Value,
) -> Result<HarnessDefinition, Vec<HarnessDiagnostic>> {
    validate_custom_schema(&value)?;
    let serialized = serde_json::to_vec(&value).map_err(|error| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            &error.to_string(),
            Some("$"),
        )]
    })?;
    let mut deserializer = serde_json::Deserializer::from_slice(&serialized);
    let definition =
        serde_path_to_error::deserialize::<_, CustomHarnessDefinition>(&mut deserializer)
            .map(HarnessDefinition::from)
            .map_err(|error| {
                let raw_path = error.path().to_string();
                let path = if raw_path == "." {
                    "$".into()
                } else {
                    format!("$.{raw_path}")
                };
                vec![HarnessDiagnostic::document(
                    "invalid_schema",
                    &error.inner().to_string(),
                    Some(&path),
                )]
            })?;
    definition.validate(DefinitionOrigin::Custom)?;
    Ok(definition)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserDocumentWire {
    version: u32,
    #[serde(default)]
    overrides: BTreeMap<String, HarnessPatch>,
    #[serde(default)]
    custom: Vec<serde_json::Value>,
    #[serde(default)]
    compatibility_profiles: BTreeMap<String, super::compatibility::CompatibilityProfile>,
}

/// Parses the persisted user document through the same restricted custom schema
/// used by the create/replace API. The public runtime definition remains broad
/// enough to represent built-ins and derived bindings, but persisted custom JSON
/// must never be allowed to populate those compiled-only fields.
pub(crate) fn parse_user_document(
    value: serde_json::Value,
) -> Result<HarnessUserDocument, Vec<HarnessDiagnostic>> {
    let object = value.as_object().ok_or_else(|| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Harness user document must be a JSON object.",
            Some("$"),
        )]
    })?;
    for field in object.keys() {
        if !["version", "overrides", "custom", "compatibilityProfiles"].contains(&field.as_str()) {
            return Err(vec![HarnessDiagnostic::document(
                "unknown_field",
                &format!("Unknown harness document field {field}."),
                Some(&format!("$.{field}")),
            )]);
        }
    }
    let wire = serde_json::from_value::<UserDocumentWire>(value).map_err(|error| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            &error.to_string(),
            Some("$"),
        )]
    })?;
    let mut custom = Vec::with_capacity(wire.custom.len());
    for (index, value) in wire.custom.into_iter().enumerate() {
        match parse_custom_definition_value(value) {
            Ok(definition) => custom.push(definition),
            Err(diagnostics) => {
                return Err(diagnostics
                    .into_iter()
                    .map(|mut diagnostic| {
                        let base = format!("$.custom[{index}]");
                        diagnostic.path = Some(match diagnostic.path.as_deref() {
                            Some("$") | None => base.clone(),
                            Some(path) if path.starts_with("$.") => {
                                format!("{base}{}", &path[1..])
                            }
                            Some(path) => format!("{base}.{path}"),
                        });
                        diagnostic
                    })
                    .collect());
            }
        }
    }
    let mut document = HarnessUserDocument {
        version: wire.version,
        overrides: wire.overrides,
        custom,
        compatibility_profiles: BTreeMap::new(),
    };
    for (id, profile) in wire.compatibility_profiles {
        if let Err(mut diagnostic) = document.set_compatibility_profile(&id, profile) {
            diagnostic.path = Some(format!("$.compatibilityProfiles.{id}"));
            return Err(vec![diagnostic]);
        }
    }
    Ok(document)
}

fn validate_custom_schema(value: &serde_json::Value) -> Result<(), Vec<HarnessDiagnostic>> {
    let object = value.as_object().ok_or_else(|| {
        vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Custom harness definition must be a JSON object.",
            Some("$"),
        )]
    })?;
    for field in object.keys() {
        let path = format!("$.{field}");
        if matches!(
            field.as_str(),
            "integration" | "sessionSignals" | "compatibilityProfile" | "compatibilityProfiles"
        ) {
            return Err(vec![HarnessDiagnostic::document(
                "custom_authority_binding",
                "Custom definitions cannot select compiled bindings or compatibility profiles.",
                Some(&path),
            )]);
        }
        if ![
            "id",
            "name",
            "retired",
            "launch",
            "defaultModel",
            "resume",
            "models",
            "peon",
            "capacity",
            "voice",
            "minVersion",
            "labelResetCommands",
        ]
        .contains(&field.as_str())
        {
            return Err(vec![HarnessDiagnostic::document(
                "unknown_field",
                &format!("Unknown custom definition field {field}."),
                Some(&path),
            )]);
        }
    }
    require_field(object, "id", "$.id")?;
    require_field(object, "name", "$.name")?;
    require_field(object, "launch", "$.launch")?;
    validate_string_value(object.get("id"), "$.id")?;
    validate_string_value(object.get("name"), "$.name")?;
    validate_json_object_fields(
        object.get("launch"),
        "$.launch",
        &["kind", "command", "args", "modelPrefix", "login"],
    )?;
    if let Some(launch) = object.get("launch").and_then(serde_json::Value::as_object) {
        validate_string_value(launch.get("kind"), "$.launch.kind")?;
        match launch.get("kind").and_then(serde_json::Value::as_str) {
            Some("command-template") => {
                reject_fields(
                    launch,
                    "$.launch",
                    &["kind", "command", "args", "modelPrefix"],
                )?;
                require_field(launch, "command", "$.launch.command")?;
                require_field(launch, "args", "$.launch.args")?;
                validate_string_value(launch.get("command"), "$.launch.command")?;
                validate_string_array(launch.get("args"), "$.launch.args")?;
                validate_nullable_string(launch.get("modelPrefix"), "$.launch.modelPrefix")?;
            }
            Some("platform-shell") => {
                reject_fields(launch, "$.launch", &["kind", "login"])?;
                require_field(launch, "login", "$.launch.login")?;
                validate_bool_value(launch.get("login"), "$.launch.login")?;
            }
            _ => {
                return Err(vec![HarnessDiagnostic::document(
                    "invalid_schema",
                    "Launch kind must be command-template or platform-shell.",
                    Some("$.launch.kind"),
                )]);
            }
        }
    }
    validate_json_object_fields(
        object.get("resume"),
        "$.resume",
        &["exact", "latestCwd", "latestRepo"],
    )?;
    if let Some(resume) = object.get("resume").and_then(serde_json::Value::as_object) {
        for (field, value) in resume {
            validate_json_object_fields(
                Some(value),
                &format!("$.resume.{field}"),
                &["command", "args"],
            )?;
        }
    }
    validate_json_object_fields(
        object.get("models"),
        "$.models",
        &["kind", "models", "command", "args"],
    )?;
    if let Some(models) = object.get("models").and_then(serde_json::Value::as_object) {
        validate_string_value(models.get("kind"), "$.models.kind")?;
        match models.get("kind").and_then(serde_json::Value::as_str) {
            Some("static") => {
                reject_fields(models, "$.models", &["kind", "models"])?;
                require_field(models, "models", "$.models.models")?;
                validate_string_array(models.get("models"), "$.models.models")?;
            }
            Some("command") => {
                reject_fields(models, "$.models", &["kind", "command", "args"])?;
                require_field(models, "command", "$.models.command")?;
                require_field(models, "args", "$.models.args")?;
                validate_string_value(models.get("command"), "$.models.command")?;
                validate_string_array(models.get("args"), "$.models.args")?;
            }
            Some("http") => {
                reject_fields(models, "$.models", &["kind"])?;
            }
            _ => {
                return Err(vec![HarnessDiagnostic::document(
                    "invalid_schema",
                    "Model kind must be static, command, or http.",
                    Some("$.models.kind"),
                )]);
            }
        }
    }
    validate_json_object_fields(
        object.get("peon"),
        "$.peon",
        &[
            "commandOverride",
            "args",
            "modelArgTemplate",
            "supportsModel",
            "timeoutSecs",
            "promptTransport",
        ],
    )?;
    validate_json_object_fields(
        object.get("capacity"),
        "$.capacity",
        &["kind", "limitPatterns"],
    )?;
    validate_json_object_fields(
        object.get("voice"),
        "$.voice",
        &[
            "nativeVoice",
            "requiresMicrophonePermission",
            "orkworksDictation",
            "orkworksVoiceCommands",
        ],
    )?;
    validate_json_object_fields(object.get("minVersion"), "$.minVersion", &["min"])?;
    Ok(())
}

fn validate_json_object_fields(
    value: Option<&serde_json::Value>,
    path: &str,
    allowed: &[&str],
) -> Result<(), Vec<HarnessDiagnostic>> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let Some(object) = value.as_object() else {
        return Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected a JSON object.",
            Some(path),
        )]);
    };
    for field in object.keys() {
        if !allowed.contains(&field.as_str()) {
            return Err(vec![HarnessDiagnostic::document(
                "unknown_field",
                &format!("Unknown custom definition field {field}."),
                Some(&format!("{path}.{field}")),
            )]);
        }
    }
    Ok(())
}

fn require_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
) -> Result<(), Vec<HarnessDiagnostic>> {
    if object.contains_key(field) {
        Ok(())
    } else {
        Err(vec![HarnessDiagnostic::document(
            "missing_field",
            &format!("Required field {field} is missing."),
            Some(path),
        )])
    }
}

fn reject_fields(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    allowed: &[&str],
) -> Result<(), Vec<HarnessDiagnostic>> {
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(vec![HarnessDiagnostic::document(
            "invalid_capability_combination",
            &format!("Field {field} is not valid for this capability variant."),
            Some(&format!("{path}.{field}")),
        )]);
    }
    Ok(())
}

fn validate_string_value(
    value: Option<&serde_json::Value>,
    path: &str,
) -> Result<(), Vec<HarnessDiagnostic>> {
    if value.is_some_and(serde_json::Value::is_string) {
        Ok(())
    } else {
        Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected a string.",
            Some(path),
        )])
    }
}

fn validate_nullable_string(
    value: Option<&serde_json::Value>,
    path: &str,
) -> Result<(), Vec<HarnessDiagnostic>> {
    if value.is_none() || value.is_some_and(|value| value.is_null() || value.is_string()) {
        Ok(())
    } else {
        Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected a string or null.",
            Some(path),
        )])
    }
}

fn validate_bool_value(
    value: Option<&serde_json::Value>,
    path: &str,
) -> Result<(), Vec<HarnessDiagnostic>> {
    if value.is_some_and(serde_json::Value::is_boolean) {
        Ok(())
    } else {
        Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected a boolean.",
            Some(path),
        )])
    }
}

fn validate_string_array(
    value: Option<&serde_json::Value>,
    path: &str,
) -> Result<(), Vec<HarnessDiagnostic>> {
    let Some(array) = value.and_then(serde_json::Value::as_array) else {
        return Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected an array of strings.",
            Some(path),
        )]);
    };
    if array.iter().all(serde_json::Value::is_string) {
        Ok(())
    } else {
        Err(vec![HarnessDiagnostic::document(
            "invalid_schema",
            "Expected an array of strings.",
            Some(path),
        )])
    }
}

/// Parses JSON while rejecting duplicate object keys, trailing input, and oversized input.
pub(crate) fn parse_strict_json<T>(bytes: &[u8], max_bytes: usize) -> Result<T, HarnessDiagnostic>
where
    T: DeserializeOwned,
{
    if bytes.len() > max_bytes {
        return Err(HarnessDiagnostic::document(
            "document_too_large",
            "Harness document exceeds the maximum size.",
            Some("$"),
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    StrictJson::deserialize(&mut deserializer).map_err(strict_json_error)?;
    deserializer.end().map_err(strict_json_error)?;
    serde_json::from_slice(bytes).map_err(strict_json_error)
}

struct StrictJson;

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)?;
        Ok(Self)
    }
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("valid JSON")
    }

    fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<StrictJson>()?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = std::collections::BTreeSet::new();
        while let Some(field) = map.next_key::<String>()? {
            if !fields.insert(field.clone()) {
                return Err(serde::de::Error::custom(format!("duplicate key {field}")));
            }
            map.next_value::<StrictJson>()?;
        }
        Ok(())
    }
}

fn strict_json_error(error: serde_json::Error) -> HarnessDiagnostic {
    let message = error.to_string();
    let code = message
        .starts_with("duplicate key ")
        .then_some("duplicate_key")
        .unwrap_or("invalid_json");
    HarnessDiagnostic::document(code, &message, Some("$"))
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
                            ));
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
                    ));
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
        if let Some(value) = &patch.min_version {
            result.min_version = value.clone();
        }
        if let Some(commands) = &patch.label_reset_commands {
            result.label_reset_commands = commands.clone().unwrap_or_default();
        }
        result
            .validate(DefinitionOrigin::Override)
            .map_err(|mut errors| errors.remove(0))?;
        Ok(result)
    }

    pub(crate) fn validate(&self, origin: DefinitionOrigin) -> Result<(), Vec<HarnessDiagnostic>> {
        let mut errors = Vec::new();
        let valid_id = !self.id.is_empty()
            && self.id.split('-').all(|segment| {
                !segment.is_empty()
                    && segment
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            });
        if !valid_id {
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
        if let Some(peon) = &self.peon {
            let command_is_empty = match &peon.command_override {
                Some(over) => over.trim().is_empty(),
                None => matches!(self.launch, LaunchCapability::PlatformShell { .. }),
            };
            if command_is_empty {
                errors.push(HarnessDiagnostic::for_id(
                    &self.id,
                    "invalid_peon_command",
                    "Peon requires a non-empty command: set peon.commandOverride or use a command-template launch.",
                ));
            }
            if let Some(template) = &peon.model_arg_template {
                validate_templates(&self.id, std::slice::from_ref(template), &mut errors);
            }
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
        prompt_transport: PromptTransport::Stdin,
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
        prompt_transport: patch
            .prompt_transport
            .clone()
            .unwrap_or(existing.prompt_transport),
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
        let mut index = 0;
        while index < value.len() {
            let remainder = &value[index..];
            let Some(offset) = remainder.find(['{', '}']) else {
                break;
            };
            index += offset;
            let token = &value[index..];
            let Some(placeholder) = ["{model}", "{cwd}", "{repoRoot}", "{harnessSessionId}"]
                .iter()
                .find(|placeholder| token.starts_with(**placeholder))
            else {
                errors.push(HarnessDiagnostic::for_id(
                    id,
                    "invalid_placeholder",
                    "Command templates use only {model}, {cwd}, {repoRoot}, or {harnessSessionId}.",
                ));
                break;
            };
            index += placeholder.len();
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessDiagnostic {
    pub harness_id: Option<String>,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl HarnessDiagnostic {
    pub(crate) fn for_id(id: &str, code: &str, message: &str) -> Self {
        Self {
            harness_id: Some(id.to_owned()),
            code: code.to_owned(),
            message: message.to_owned(),
            path: None,
        }
    }

    pub(crate) fn document(code: &str, message: &str, path: Option<&str>) -> Self {
        Self {
            harness_id: None,
            code: code.to_owned(),
            message: message.to_owned(),
            path: path.map(ToOwned::to_owned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::registry::resolve_document;

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
                "antigravity",
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
        assert!(resolved.get("gemini").unwrap().definition.retired);
        assert!(!resolved.get("antigravity").unwrap().definition.retired);
        assert_eq!(
            resolved
                .get("claude-code")
                .unwrap()
                .definition
                .label_reset_commands,
            ["/clear", "/reset", "/new"]
        );
        assert_eq!(
            resolved
                .get("opencode")
                .unwrap()
                .definition
                .label_reset_commands,
            ["/clear", "/new"]
        );
        assert_eq!(
            resolved
                .get("copilot")
                .unwrap()
                .definition
                .label_reset_commands,
            ["/clear", "/new"]
        );
        assert!(resolved
            .get("codex")
            .unwrap()
            .definition
            .label_reset_commands
            .is_empty());
    }

    #[test]
    fn label_reset_commands_default_for_legacy_custom_documents() {
        let document: HarnessUserDocument = serde_json::from_str(
            r#"{"version":2,"custom":[{
              "id":"company-tool","name":"Company Tool",
              "launch":{"kind":"command-template","command":"company-tool","args":[],"modelPrefix":null},
              "defaultModel":null,"resume":null,"models":null,"peon":null,
              "capacity":null,"sessionSignals":null,"integration":null,"voice":null
            }]}"#,
        ).unwrap();
        assert!(document.custom[0].label_reset_commands.is_empty());
    }

    #[test]
    fn label_reset_command_patch_replaces_or_clears_the_builtin_list() {
        let original = BuiltinDocument::parse(EMBEDDED_BUILTINS)
            .unwrap()
            .builtins
            .into_iter()
            .find(|h| h.id == "claude-code")
            .unwrap();
        assert_eq!(original.label_reset_commands, ["/clear", "/reset", "/new"]);

        let replacement: HarnessPatch =
            serde_json::from_str(r#"{"labelResetCommands":["/fresh"]}"#).unwrap();
        assert_eq!(
            original
                .apply_patch(&replacement)
                .unwrap()
                .label_reset_commands,
            ["/fresh"]
        );

        let cleared: HarnessPatch = serde_json::from_str(r#"{"labelResetCommands":null}"#).unwrap();
        assert!(original
            .apply_patch(&cleared)
            .unwrap()
            .label_reset_commands
            .is_empty());
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
    fn sparse_json_patch_serializes_without_null_placeholders() {
        let patch = HarnessPatch {
            name: Some("Configured Codex".into()),
            ..Default::default()
        };

        let json = serde_json::to_string(&patch).unwrap();

        assert_eq!(json, r#"{"name":"Configured Codex"}"#);
        assert_eq!(serde_json::from_str::<HarnessPatch>(&json).unwrap(), patch);
    }

    #[test]
    fn boundary_nulls_still_serialize_as_null() {
        let patch = HarnessPatch {
            capacity: Some(None),
            ..Default::default()
        };

        let json = serde_json::to_string(&patch).unwrap();

        assert_eq!(json, r#"{"capacity":null}"#);
        assert_eq!(serde_json::from_str::<HarnessPatch>(&json).unwrap(), patch);
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
    fn min_version_round_trips_through_serde_and_patch_and_the_codex_builtin_has_it_set() {
        // The codex builtin entry declares the hooks framework's minimum version.
        let definition = codex();
        assert_eq!(
            definition.min_version,
            Some(VersionRequirement { min: (0, 114, 0) })
        );

        // A sparse patch can set min_version on a harness that doesn't have one.
        let set_patch: HarnessPatch =
            serde_json::from_str(r#"{"minVersion":{"min":[1,2,3]}}"#).unwrap();
        let patched = definition.apply_patch(&set_patch).unwrap();
        assert_eq!(
            patched.min_version,
            Some(VersionRequirement { min: (1, 2, 3) })
        );

        // Explicit null clears it, same as every other optional capability.
        let clear_patch: HarnessPatch = serde_json::from_str(r#"{"minVersion":null}"#).unwrap();
        assert!(definition
            .apply_patch(&clear_patch)
            .unwrap()
            .min_version
            .is_none());

        // Omitting the field entirely leaves the builtin's min_version untouched.
        let noop_patch: HarnessPatch =
            serde_json::from_str(r#"{"name":"Configured Codex"}"#).unwrap();
        assert_eq!(
            definition.apply_patch(&noop_patch).unwrap().min_version,
            definition.min_version
        );
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
            r#"{"defaultModel":null,"resume":null,"models":null,"peon":null,"capacity":null,"voice":null}"#,
        )
        .unwrap();
        assert_eq!(optional.default_model, Some(None));
        assert_eq!(optional.resume, Some(None));
        assert_eq!(optional.models, Some(None));
        assert_eq!(optional.peon, Some(None));
        assert_eq!(optional.capacity, Some(None));
        assert_eq!(optional.voice, Some(None));

        for binding in [r#"{"sessionSignals":null}"#, r#"{"integration":null}"#] {
            assert!(serde_json::from_str::<HarnessPatch>(binding).is_err());
        }
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
    fn peon_with_platform_shell_launch_and_no_override_is_rejected() {
        let mut definition = codex();
        definition.integration = None;
        definition.session_signals = None;
        definition.launch = LaunchCapability::PlatformShell { login: true };
        assert!(definition.peon.is_some());

        let errors = definition
            .validate(DefinitionOrigin::Builtin)
            .expect_err("empty peon command must fail validation");
        assert!(errors
            .iter()
            .any(|error| error.code == "invalid_peon_command"));
    }

    #[test]
    fn peon_with_blank_command_override_is_rejected() {
        let mut definition = codex();
        definition.integration = None;
        definition.session_signals = None;
        definition.peon.as_mut().unwrap().command_override = Some("   ".into());

        let errors = definition
            .validate(DefinitionOrigin::Builtin)
            .expect_err("blank commandOverride must fail validation");
        assert!(errors
            .iter()
            .any(|error| error.code == "invalid_peon_command"));
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

    #[test]
    fn custom_json_cannot_select_a_compiled_integration() {
        let error = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null},"integration":{"kind":"copilot"}}"#,
        )
        .expect_err("custom JSON must not select an integration binding");

        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.code == "custom_authority_binding"));
        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.integration")));
    }

    #[test]
    fn custom_json_rejects_unknown_fields_with_a_json_path() {
        let error = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null},"unknown":true}"#,
        )
        .expect_err("unknown custom fields must be rejected");

        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.code == "unknown_field"));
        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.unknown")));

        let nested = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null,"hookPath":"/tmp/untrusted"}}"#,
        )
        .expect_err("unknown nested custom fields must be rejected");
        assert!(nested
            .iter()
            .any(|diagnostic| diagnostic.code == "unknown_field"));
        assert!(nested
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.launch.hookPath")));
    }

    #[test]
    fn custom_json_rejects_malformed_command_placeholders() {
        let error = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":["--model={unknown}"],"modelPrefix":null}}"#,
        )
        .expect_err("unknown command placeholders must be rejected");

        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid_placeholder"));
    }

    #[test]
    fn custom_json_reports_nested_schema_paths() {
        let invalid_type = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":true,"args":[],"modelPrefix":null}}"#,
        )
        .expect_err("nested type errors must be rejected");
        assert!(invalid_type
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.launch.command")));

        let missing = parse_custom_definition(br#"{"id":"copilot-local","name":"Copilot Local"}"#)
            .expect_err("required fields must be rejected");
        assert!(missing
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.launch")));
    }

    #[test]
    fn custom_json_requires_lowercase_kebab_case_ids() {
        let valid = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null}}"#,
        );
        let invalid = parse_custom_definition(
            br#"{"id":"Copilot_Local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null}}"#,
        );

        assert!(valid.is_ok());
        assert!(invalid.is_err());

        for id in ["-copilot", "copilot-", "copilot--local"] {
            let value = format!(
                r#"{{"id":"{id}","name":"Copilot Local","launch":{{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null}}}}"#
            );
            assert!(
                parse_custom_definition(value.as_bytes()).is_err(),
                "invalid ID: {id}"
            );
        }
    }

    #[test]
    fn custom_json_rejects_fields_from_the_wrong_launch_variant() {
        let error = parse_custom_definition(
            br#"{"id":"shell-tool","name":"Shell Tool","launch":{"kind":"platform-shell","login":true,"command":"unexpected"}}"#,
        )
        .expect_err("platform-shell must not accept command-template fields");
        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.path.as_deref() == Some("$.launch.command")));
    }

    #[test]
    fn custom_json_validates_peon_model_placeholders() {
        let error = parse_custom_definition(
            br#"{"id":"peon-tool","name":"Peon Tool","launch":{"kind":"command-template","command":"tool","args":[],"modelPrefix":null},"peon":{"commandOverride":null,"args":[],"modelArgTemplate":"--model={unknown}","supportsModel":true,"timeoutSecs":30,"promptTransport":"stdin"}}"#,
        )
        .expect_err("Peon model templates must use the closed placeholder set");
        assert!(error
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid_placeholder"));
    }

    #[test]
    fn removing_a_custom_definition_also_removes_its_profile() {
        let definition = parse_custom_definition(
            br#"{"id":"copilot-local","name":"Copilot Local","launch":{"kind":"command-template","command":"copilot-local","args":[],"modelPrefix":null}}"#,
        )
        .expect("custom definition");
        let mut document = HarnessUserDocument::default();
        document.custom.push(definition);
        document.compatibility_profiles.insert(
            "copilot-local".into(),
            super::super::compatibility::CompatibilityProfile::Copilot,
        );

        assert!(document.remove_custom_definition("copilot-local"));
        assert!(document.custom.is_empty());
        assert!(document.compatibility_profiles.is_empty());
        assert!(!document.remove_custom_definition("copilot-local"));
    }

    #[test]
    fn user_document_uses_the_v3_profile_wire_name_and_rejects_unknown_fields() {
        let serialized = serde_json::to_value(HarnessUserDocument::default()).unwrap();
        assert!(serialized.get("compatibilityProfiles").is_some());

        assert!(serde_json::from_str::<HarnessUserDocument>(
            r#"{"version":3,"overrides":{},"custom":[],"compatibilityProfiles":{},"unknown":true}"#,
        )
        .is_err());
    }
}
