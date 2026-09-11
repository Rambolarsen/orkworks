//! Custom transport only: preparation is not permission to execute.
//! Production activation must gate scheduling, spawn, cache and final acceptance on trust.

#[cfg(test)]
mod transport_tests;

use super::{ProviderOperationError, ProviderOperationErrorCode, MAX_PROVIDER_OUTPUT_BYTES};
use crate::harness::{
    definition::parse_strict_json,
    inference::{valid_selection_value, InferenceCapability, Input},
};
use serde::Deserialize;
use std::{io::Write, path::Path, process::Command};

pub(crate) struct PreparedCustomInference {
    command: Command,
    stdin: String,
    timeout_secs: u64,
    // File drops before directory; both remain owned through synchronous runner cleanup.
    _prompt_file: Option<tempfile::NamedTempFile>,
    _directory: tempfile::TempDir,
}

fn invalid(message: &str) -> ProviderOperationError {
    ProviderOperationError {
        code: ProviderOperationErrorCode::Malformed,
        message: message.into(),
    }
}

/// `resolved_path` must come from a freshly checked runtime trust identity.
/// This transport does not load registry/trust state or confer authorization.
pub(crate) fn prepare(
    capability: &InferenceCapability,
    resolved_path: &Path,
    model: &str,
    effort: Option<&str>,
    prompt: String,
) -> Result<PreparedCustomInference, ProviderOperationError> {
    if !valid_selection_value(model) || effort.is_some_and(|value| !valid_selection_value(value)) {
        return Err(invalid("invalid custom inference model or effort"));
    }
    if effort.is_some() && capability.reasoning_effort_args().is_none() {
        return Err(invalid(
            "custom inference adapter does not support reasoning effort",
        ));
    }
    if prompt.len() > 256 * 1024 {
        return Err(invalid("inference prompt exceeds 256 KiB"));
    }
    if !resolved_path.is_absolute() {
        return Err(invalid(
            "custom inference requires a resolved absolute executable",
        ));
    }
    let directory = tempfile::Builder::new()
        .prefix("orkworks-custom-inference-")
        .tempdir()
        .map_err(|_| invalid("could not create private inference directory"))?;
    let mut prompt_file = None;
    if capability.input() == Input::File {
        let mut file = tempfile::NamedTempFile::new_in(directory.path())
            .map_err(|_| invalid("could not create private inference input"))?;
        file.write_all(prompt.as_bytes())
            .and_then(|()| file.flush())
            .map_err(|_| invalid("could not write private inference input"))?;
        prompt_file = Some(file);
    }
    let file_path = prompt_file
        .as_ref()
        .map(|file| {
            file.path()
                .to_str()
                .ok_or_else(|| invalid("inference input path is not UTF-8"))
        })
        .transpose()?;
    let mut command = Command::new(resolved_path);
    command.current_dir(directory.path());
    for arg in capability.args() {
        command.arg(expand(arg, model, effort, file_path)?);
    }
    if effort.is_some() {
        for arg in capability.reasoning_effort_args().unwrap_or_default() {
            command.arg(expand(arg, model, effort, file_path)?);
        }
    }
    // Freeze a filtered copy: late process-global environment additions cannot
    // reintroduce action/report capabilities between preparation and spawn.
    command.env_clear();
    for (key, value) in std::env::vars_os() {
        let normalized = key.to_string_lossy().to_ascii_uppercase();
        if !normalized.starts_with("ORKWORKS_")
            && !matches!(normalized.as_str(), "BASH_ENV" | "ENV")
        {
            command.env(key, value);
        }
    }
    Ok(PreparedCustomInference {
        command,
        stdin: if capability.input() == Input::Stdin {
            prompt
        } else {
            String::new()
        },
        timeout_secs: capability.timeout_secs(),
        _prompt_file: prompt_file,
        _directory: directory,
    })
}

fn expand(
    template: &str,
    model: &str,
    effort: Option<&str>,
    file: Option<&str>,
) -> Result<String, ProviderOperationError> {
    let mut output = String::new();
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        output.push_str(&remaining[..start]);
        let end = remaining[start..]
            .find('}')
            .map(|end| start + end)
            .ok_or_else(|| invalid("invalid inference argument template"))?;
        let replacement = match &remaining[start..=end] {
            "{model}" => Some(model),
            "{effort}" => effort,
            "{promptFile}" => file,
            _ => None,
        }
        .ok_or_else(|| invalid("missing inference argument value"))?;
        output.push_str(replacement);
        remaining = &remaining[end + 1..];
    }
    output.push_str(remaining);
    Ok(output)
}

fn decode(stdout: &str) -> Result<String, ProviderOperationError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        version: u8,
        status: String,
        result: String,
    }
    let envelope: Envelope = parse_strict_json(stdout.as_bytes(), MAX_PROVIDER_OUTPUT_BYTES)
        .map_err(|_| invalid("invalid custom inference response"))?;
    if envelope.version != 1 || envelope.status != "success" || envelope.result.trim().is_empty() {
        return Err(invalid(
            "custom inference did not return a successful result",
        ));
    }
    parse_strict_json::<serde_json::Value>(envelope.result.as_bytes(), MAX_PROVIDER_OUTPUT_BYTES)
        .map_err(|_| invalid("invalid custom inference result JSON"))?;
    Ok(envelope.result)
}

impl PreparedCustomInference {
    #[cfg(test)]
    fn run(self) -> Result<String, ProviderOperationError> {
        self.run_with_spawn(|command| Ok(command.spawn()))
    }

    /// Consume the invocation so private input survives every runner exit path.
    /// The caller must keep its authorization guard through `Command::spawn`,
    /// then release it before returning the child. This does not accept results
    /// into Taskmaster; final acceptance requires a fresh identity check.
    pub(crate) fn run_with_spawn(
        mut self,
        spawn: impl FnOnce(
            &mut Command,
        )
            -> Result<std::io::Result<std::process::Child>, ProviderOperationError>,
    ) -> Result<String, ProviderOperationError> {
        let outcome = super::ProcessRunner.run_prepared_with_spawn(
            "custom-inference",
            &mut self.command,
            &self.stdin,
            self.timeout_secs,
            true,
            spawn,
        )?;
        let result = match outcome {
            super::ProcessOutcome::Finished(result) => result,
            super::ProcessOutcome::TimedOut => {
                return Err(ProviderOperationError {
                    code: ProviderOperationErrorCode::Timeout,
                    message: "custom inference timed out".into(),
                })
            }
        };
        if !result.success {
            return Err(ProviderOperationError {
                code: ProviderOperationErrorCode::ProviderFailure,
                message: "custom inference failed; check the trusted adapter locally".into(),
            });
        }
        decode(&result.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::inference::InferenceCapability;
    use serde_json::json;

    fn capability(file: bool, effort: bool) -> InferenceCapability {
        let mut value = json!({"kind":"command","command":std::env::current_exe().unwrap(),
            "args":["--model={model}","literal;$value"],"input":"stdin","output":"result-json-v1"});
        if file {
            value["input"] = json!("file");
            value["args"] = json!(["--model={model}", "{promptFile}"]);
        }
        if effort {
            value["reasoningEffortArgs"] = json!(["--effort={effort}"]);
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn custom_inference_preserves_literal_model_and_owns_private_prompt_file() {
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        for file in [false, true] {
            let prepared = prepare(
                &capability(file, true),
                &executable,
                "vendor model;$(nothing){promptFile}{effort}",
                Some("high"),
                "private context".into(),
            )
            .unwrap();
            assert_eq!(prepared.command.get_program(), executable);
            let args: Vec<_> = prepared
                .command
                .get_args()
                .map(|arg| arg.to_str().unwrap())
                .collect();
            assert_eq!(
                args[0],
                "--model=vendor model;$(nothing){promptFile}{effort}"
            );
            assert_eq!(args[2], "--effort=high");
            let directory = prepared.command.get_current_dir().unwrap().to_path_buf();
            assert!(directory.is_dir());
            if file {
                assert_eq!(std::fs::read_to_string(args[1]).unwrap(), "private context");
                assert!(std::path::Path::new(args[1]).starts_with(&directory));
                assert!(prepared.stdin.is_empty());
            } else {
                assert_eq!(args[1], "literal;$value");
                assert_eq!(prepared.stdin, "private context");
            }
            drop(prepared);
            assert!(!directory.exists());
        }
    }

    #[test]
    fn custom_inference_rejects_invalid_inputs_without_dispatch() {
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        for model in ["".into(), "a\nb".into(), "é".repeat(129)] {
            assert!(prepare(
                &capability(false, false),
                &executable,
                &model,
                None,
                "prompt".into()
            )
            .is_err());
        }
        assert!(prepare(
            &capability(false, false),
            &executable,
            "chosen",
            Some("high"),
            "prompt".into()
        )
        .is_err());
        for effort in ["", "high\n", &"e".repeat(257)] {
            assert!(prepare(
                &capability(false, true),
                &executable,
                "chosen",
                Some(effort),
                "prompt".into()
            )
            .is_err());
        }
        assert!(prepare(
            &capability(false, false),
            &executable,
            "chosen",
            None,
            "x".repeat(256 * 1024 + 1)
        )
        .is_err());
        assert!(prepare(
            &capability(false, false),
            std::path::Path::new("relative"),
            "chosen",
            None,
            "prompt".into()
        )
        .is_err());
        assert!(prepare(
            &capability(false, false),
            &executable,
            &"é".repeat(128),
            None,
            "x".repeat(256 * 1024)
        )
        .is_ok());
    }

    #[test]
    fn custom_inference_omits_unselected_effort_and_cleans_up_spawn_failure() {
        let root = tempfile::tempdir().unwrap();
        let prepared = prepare(
            &capability(true, true),
            &root.path().join("missing.exe"),
            "chosen",
            None,
            "private context".into(),
        )
        .unwrap();
        assert_eq!(prepared.command.get_args().count(), 2);
        let directory = prepared.command.get_current_dir().unwrap().to_path_buf();
        let file = prepared._prompt_file.as_ref().unwrap().path().to_path_buf();
        assert!(file.exists());
        assert_eq!(
            prepared.run().unwrap_err().code,
            ProviderOperationErrorCode::ProviderFailure
        );
        assert!(!file.exists());
        assert!(!directory.exists());
    }

    #[test]
    fn custom_inference_guard_denial_cleans_up_private_input_and_preserves_error() {
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        let prepared = prepare(
            &capability(true, false),
            &executable,
            "chosen",
            None,
            "private fixture context".into(),
        )
        .unwrap();
        let directory = prepared._directory.path().to_path_buf();
        let file = prepared._prompt_file.as_ref().unwrap().path().to_path_buf();
        let error = prepared
            .run_with_spawn(|_| {
                Err(ProviderOperationError {
                    code: ProviderOperationErrorCode::StaleGeneration,
                    message: "fixture authorization denied".into(),
                })
            })
            .unwrap_err();
        assert_eq!(error.code, ProviderOperationErrorCode::StaleGeneration);
        assert!(!file.exists());
        assert!(!directory.exists());
    }

    #[test]
    fn custom_inference_decodes_only_strict_success_envelopes_and_nested_json() {
        let result = r#"{"enrichments":[],"proposals":[]}"#;
        let valid = json!({"version":1,"status":"success","result":result}).to_string();
        assert_eq!(decode(&valid).unwrap(), result);
        for raw in [
            "".into(),
            "not json".into(),
            format!("log\n{valid}"),
            format!("{valid}\n{valid}"),
            r#"{"version":1,"version":1,"status":"success","result":"{}"}"#.into(),
            json!({"version":2,"status":"success","result":"{}"}).to_string(),
            json!({"version":1,"status":"error","result":"{}"}).to_string(),
            json!({"version":1,"status":"success","result":"{}","extra":true}).to_string(),
            json!({"version":1,"status":"success","result":" "}).to_string(),
            json!({"version":1,"status":"success","result":"not-json"}).to_string(),
            json!({"version":1,"status":"success","result":"{\"a\":{\"x\":1,\"x\":2}}"})
                .to_string(),
            json!({"version":1,"status":"success","result":"{} {}"}).to_string(),
            format!("{valid}{}", " ".repeat(64 * 1024)),
        ] {
            assert!(decode(&raw).is_err(), "accepted {raw}");
        }
    }
}
