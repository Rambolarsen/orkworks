//! Pure source projection, not native validity, custody or launch authority.
use super::{codex, portable_reporter_invocation_from_path, ReporterInvocation, ReporterPlatform};
use crate::harness::definition::parse_strict_json;
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::{json, Value};
use std::path::Path;

const INPUT_CAP: usize = 256 * 1024;
const OUTPUT_CAP: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodexFragmentError {
    InputTooLarge,
    MalformedCarrier,
    UnsupportedFragment,
    OutputTooLarge,
}

/// Contains reconstructed source bytes only; no deserializer or public constructor.
pub(crate) struct OwnedCodexFragment(Vec<u8>);
impl OwnedCodexFragment {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

fn contains_namespace(value: &Value) -> bool {
    match value {
        Value::String(value) => value.contains(codex::MARKER_PREFIX),
        Value::Array(values) => values.iter().any(contains_namespace),
        Value::Object(values) => values
            .iter()
            .any(|(key, value)| key.contains(codex::MARKER_PREFIX) || contains_namespace(value)),
        _ => false,
    }
}

#[derive(Serialize)]
struct NativeHook<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    command: &'a str,
}
#[derive(Serialize)]
struct NativeGroup<'a> {
    hooks: [NativeHook<'a>; 1],
}
struct NativeEvents<'a>(&'a [ReporterInvocation; 6]);
impl Serialize for NativeEvents<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut events = serializer.serialize_map(Some(6))?;
        for (event, command) in codex::CODEX_EVENTS.iter().zip(self.0) {
            let group = NativeGroup {
                hooks: [NativeHook {
                    kind: "command",
                    command: &command.shell_command,
                }],
            };
            events.serialize_entry(event, &[group])?;
        }
        events.end()
    }
}
#[derive(Serialize)]
struct NativeDocument<'a> {
    hooks: NativeEvents<'a>,
}

pub(crate) fn project_standard_posix_codex_fragment(
    bytes: &[u8],
) -> Result<Option<OwnedCodexFragment>, CodexFragmentError> {
    use CodexFragmentError::{
        InputTooLarge, MalformedCarrier, OutputTooLarge, UnsupportedFragment,
    };
    if bytes.len() > INPUT_CAP {
        return Err(InputTooLarge);
    }
    // Detailed parser diagnostics may contain user key names: never propagate them.
    let value = parse_strict_json::<Value>(bytes, INPUT_CAP).map_err(|_| MalformedCarrier)?;
    let root = value.as_object().ok_or(MalformedCarrier)?;
    if root.iter().any(|(key, value)| {
        key != "hooks" && (key.contains(codex::MARKER_PREFIX) || contains_namespace(value))
    }) {
        return Err(UnsupportedFragment);
    }
    let Some(hooks) = root.get("hooks") else {
        return Ok(None);
    };
    let events = hooks.as_object().ok_or(MalformedCarrier)?;
    let base = portable_reporter_invocation_from_path(
        Path::new("$HOME/.orkworks/hook-scripts/report-harness-event.sh"),
        codex::MARKER,
    );
    let (_, commands) = codex::command_bundle(&base, ReporterPlatform::Posix);
    let mut seen = [false; 6];
    for (event, groups) in events {
        if event.contains(codex::MARKER_PREFIX) {
            return Err(UnsupportedFragment);
        }
        let groups = groups.as_array().ok_or(MalformedCarrier)?;
        for group in groups {
            if !contains_namespace(group) {
                continue;
            }
            let index = codex::CODEX_EVENTS
                .iter()
                .position(|known| known == event)
                .ok_or(UnsupportedFragment)?;
            let expected = json!({"hooks":[{"type":"command","command":commands[index].shell_command.as_str()}]});
            if seen[index] || group != &expected {
                return Err(UnsupportedFragment);
            }
            seen[index] = true;
        }
    }
    if !seen.iter().any(|seen| *seen) {
        return Ok(None);
    }
    if !seen.iter().all(|seen| *seen) {
        return Err(UnsupportedFragment);
    }
    let output = serde_json::to_vec(&NativeDocument {
        hooks: NativeEvents(&commands),
    })
    .map_err(|_| UnsupportedFragment)?;
    if output.len() > OUTPUT_CAP {
        return Err(OutputTooLarge);
    }
    Ok(Some(OwnedCodexFragment(output)))
}
