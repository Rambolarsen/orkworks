//! Declarative inference configuration, not permission to execute a command.
use serde::{Deserialize, Serialize};

/// Opaque model/effort values are bounded, not normalized or provider-prefixed.
pub(crate) fn valid_selection_value(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct InferenceCapability(RawInferenceCapability);

impl InferenceCapability {
    pub(crate) fn command(&self) -> &str {
        &self.0.command
    }
    pub(crate) fn args(&self) -> &[String] {
        &self.0.args
    }
    pub(crate) fn input(&self) -> Input {
        self.0.input
    }
    pub(crate) fn timeout_secs(&self) -> u64 {
        self.0.timeout_secs
    }
    pub(crate) fn reasoning_effort_args(&self) -> Option<&[String]> {
        self.0.reasoning_effort_args.as_deref()
    }
}

impl<'de> Deserialize<'de> for InferenceCapability {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawInferenceCapability::deserialize(deserializer)?;
        raw.validate().map_err(serde::de::Error::custom)?;
        Ok(Self(raw))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawInferenceCapability {
    kind: CommandKind,
    command: String,
    args: Vec<String>,
    input: Input,
    output: Output,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(
        default,
        deserialize_with = "present_effort",
        skip_serializing_if = "Option::is_none"
    )]
    reasoning_effort_args: Option<Vec<String>>,
}

fn present_effort<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(d).map(Some)
}

impl RawInferenceCapability {
    fn validate(&self) -> Result<(), &'static str> {
        if self.command.trim().is_empty()
            || !bounded(&self.command)
            || self.command.contains(['{', '}'])
        {
            return Err("inference executable must be nonempty, bounded, and contain no placeholders or controls");
        }
        if self.command.contains(['/', '\\', ':'])
            && !std::path::Path::new(&self.command).is_absolute()
        {
            return Err("inference executable must be absolute or a bare executable name");
        }
        if !(1..=120).contains(&self.timeout_secs) {
            return Err("inference timeout must be between 1 and 120 seconds");
        }
        if self.args.len() + self.reasoning_effort_args.as_ref().map_or(0, Vec::len) > 64 {
            return Err("inference supports at most 64 combined arguments");
        }
        validate_args(&self.args, [1, 0, usize::from(self.input == Input::File)])?;
        if let Some(args) = &self.reasoning_effort_args {
            validate_args(args, [0, 1, 0])?;
        }
        Ok(())
    }
}

fn bounded(value: &str) -> bool {
    value.len() <= 4096 && !value.chars().any(char::is_control)
}

fn validate_args(args: &[String], expected: [usize; 3]) -> Result<(), &'static str> {
    let mut counts = [0; 3];
    for arg in args {
        if !bounded(arg) {
            return Err("inference argument exceeds 4 KiB or contains controls");
        }
        let mut remaining = arg.as_str();
        while let Some(index) = remaining.find(['{', '}']) {
            remaining = &remaining[index..];
            let Some(end) = remaining.find('}') else {
                return Err("malformed inference placeholder");
            };
            let token = &remaining[..=end];
            let index = match token {
                "{model}" => 0,
                "{effort}" => 1,
                "{promptFile}" => 2,
                _ => return Err("unknown or malformed inference placeholder"),
            };
            counts[index] += 1;
            remaining = &remaining[end + 1..];
        }
    }
    if counts != expected {
        return Err(
            "inference placeholders must occur exactly once in their designated argument list",
        );
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CommandKind {
    Command,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Input {
    Stdin,
    File,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
enum Output {
    #[serde(rename = "result-json-v1")]
    ResultJsonV1,
}

fn default_timeout() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn valid() -> Value {
        json!({"kind":"command","command":"custom-infer","args":["--model","{model}"],"input":"stdin","output":"result-json-v1"})
    }

    #[test]
    fn inference_definition_rejects_invalid_contracts_at_deserialization() {
        for patch in [
            json!({"command":""}),
            json!({"command":"./tool"}),
            json!({"command":"dir/tool"}),
            json!({"command":"tool\n"}),
            json!({"command":"{model}"}),
            json!({"command":"é".repeat(2049)}),
            json!({"args":[]}),
            json!({"args":["{model}","{model}"]}),
            json!({"args":["{model}","{cwd}"]}),
            json!({"args":["{model}","{effort}"]}),
            json!({"args":["{model}","{promptFile}"]}),
            json!({"args":["{model}","unmatched}"]}),
            json!({"args":["{model}","{broken"]}),
            json!({"args":["{model}","x\n"]}),
            json!({"args":["{model}","é".repeat(2049)]}),
            json!({"args":std::iter::once("{model}").chain(std::iter::repeat_n("x",64)).collect::<Vec<_>>()}),
            json!({"input":"file"}),
            json!({"input":"argument"}),
            json!({"output":"text"}),
            json!({"timeoutSecs":0}),
            json!({"timeoutSecs":121}),
            json!({"timeoutSecs":null}),
            json!({"reasoningEffortArgs":[]}),
            json!({"reasoningEffortArgs":null}),
            json!({"reasoningEffortArgs":["{model}"]}),
            json!({"reasoningEffortArgs":["{effort}","{effort}"]}),
            json!({"kind":"builtin"}),
            json!({"trusted":true}),
        ] {
            let mut value = valid();
            value
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(
                serde_json::from_value::<InferenceCapability>(value).is_err(),
                "accepted {patch}"
            );
        }
    }

    #[test]
    fn inference_definition_preserves_file_and_effort_templates() {
        let value = json!({"kind":"command","command":"custom-infer","args":["--model={model}","--input={promptFile}","literal;$value"],"input":"file","output":"result-json-v1","timeoutSecs":120,"reasoningEffortArgs":["--effort={effort}"]});
        let parsed = serde_json::from_value::<InferenceCapability>(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }

    #[test]
    fn inference_definition_accepts_boundary_sizes_and_bounds_combined_args() {
        let mut value = valid();
        value["command"] = serde_json::json!(std::env::current_exe().unwrap().to_str().unwrap());
        value["args"] = json!(std::iter::once("{model}".to_owned())
            .chain(std::iter::repeat_n("é".repeat(2048), 62))
            .collect::<Vec<_>>());
        value["reasoningEffortArgs"] = json!(["{effort}"]);
        value["timeoutSecs"] = json!(1);
        assert!(serde_json::from_value::<InferenceCapability>(value.clone()).is_ok());
        value["reasoningEffortArgs"] = json!(["--effort", "{effort}"]);
        assert!(serde_json::from_value::<InferenceCapability>(value).is_err());
    }
}
