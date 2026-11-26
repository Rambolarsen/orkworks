//! Source-owned Antigravity launch qualification with owner-accepted limited coverage.

use super::terminal_status::Profile;
use crate::harness::{
    definition::{BuiltinDocument, DefinitionOrigin, EMBEDDED_BUILTINS},
    registry::ResolvedHarness,
    CommandSpec,
};

// The owner accepted the retained macOS arm64 1.5.16 and 1.5.17 trial limits.
// Other versions/platforms remain unqualified; see the native status record.
const QUALIFIED_VERSIONS: &[&str] = &["1.5.16", "1.5.17"];

#[derive(Clone, Debug)]
pub(crate) struct LaunchRequest {
    command: CommandSpec,
}
impl LaunchRequest {
    pub(crate) fn new(
        harness: &ResolvedHarness,
        command: &CommandSpec,
        model: Option<&str>,
        native_id: Option<&str>,
        initial_prompt: Option<&str>,
    ) -> Option<Self> {
        if harness.definition.id != "antigravity" || harness.origin == DefinitionOrigin::Custom {
            return None;
        }
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).ok()?;
        let builtin = builtins.builtins.iter().find(|h| h.id == "antigravity")?;
        let actual = &harness.definition;
        if actual.launch != builtin.launch
            || actual.resume != builtin.resume
            || actual.integration != builtin.integration
            || actual.session_signals != builtin.session_signals
        {
            return None;
        }
        let mut expected = match native_id {
            Some(id) => harness.build_resume(
                crate::harness::ResumeStrategy::Exact,
                &command.cwd,
                Some(id),
                None,
                model,
            )?,
            None => harness.build_launch(&command.cwd, model),
        };
        // Mirror the existing source-owned initial-prompt route, comparing the
        // exact submitted prompt rather than allowing an arbitrary argv suffix.
        if native_id.is_none() && expected.program == "agy" && expected.args.is_empty() {
            if let Some(prompt) = initial_prompt.filter(|text| !text.trim().is_empty()) {
                expected.args.push(format!("--prompt-interactive={prompt}"));
            }
        }
        (expected == *command).then(|| Self {
            command: command.clone(),
        })
    }
}

fn qualified_version(version: &str, versions: &[&str]) -> bool {
    versions.contains(&version.trim())
}

pub(crate) async fn prepare(
    request: Option<LaunchRequest>,
    command: &mut CommandSpec,
) -> Option<Profile> {
    prepare_qualified(
        request,
        command,
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        QUALIFIED_VERSIONS,
    )
    .await
}

async fn prepare_qualified(
    request: Option<LaunchRequest>,
    command: &mut CommandSpec,
    platform: bool,
    versions: &[&str],
) -> Option<Profile> {
    if versions.is_empty() || !platform {
        return None;
    }
    let request = request?;
    if request.command != *command {
        return None;
    }
    let tool = crate::harness::detect::probe_installed_tool(&command.program)?;
    let executable = std::fs::canonicalize(tool.executable).ok()?;
    let mut probe = std::process::Command::new(&executable);
    probe.arg("--version").env_clear();
    for key in [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "SYSTEMROOT",
    ] {
        if let Some(value) = std::env::var_os(key) {
            probe.env(key, value);
        }
    }
    let output = crate::probe_process::run(
        &mut probe,
        crate::probe_process::ProbePolicy {
            max_bytes: 64 * 1024,
            reject_overflow: true,
            require_success: true,
        },
        std::time::Duration::from_secs(3),
    )
    .await
    .ok()?;
    // A noisy banner is not evidence of the requested executable/version.
    if !output.stderr.is_empty()
        || !qualified_version(std::str::from_utf8(&output.stdout).ok()?, versions)
    {
        return None;
    }
    command.program = executable.to_str()?.to_owned();
    Some(Profile::Antigravity)
}

// Synthetic executables only. This seam never changes the shipping allowlist.
#[cfg(test)]
pub(crate) async fn prepare_fixture(command: &mut CommandSpec) -> Option<Profile> {
    prepare_qualified(
        Some(LaunchRequest {
            command: command.clone(),
        }),
        command,
        true,
        QUALIFIED_VERSIONS,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{BuiltinDocument, HarnessUserDocument, EMBEDDED_BUILTINS};
    use crate::harness::registry::resolve_document;

    fn harness() -> crate::harness::registry::ResolvedHarness {
        resolve_document(
            &BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap(),
            &HarnessUserDocument::default(),
        )
        .unwrap()
        .get("antigravity")
        .unwrap()
        .clone()
    }

    #[test]
    fn antigravity_terminal_request_rejects_overrides_and_changed_argv() {
        let original = harness();
        let command = original.build_launch("/repo", Some("fixture-model"));
        assert!(
            LaunchRequest::new(&original, &command, Some("fixture-model"), None, None).is_some()
        );
        let mut changed = original.clone();
        changed.origin = crate::harness::definition::DefinitionOrigin::Custom;
        assert!(
            LaunchRequest::new(&changed, &command, Some("fixture-model"), None, None).is_none()
        );
        let mut command_override = command.clone();
        command_override.program = "/custom/agy".into();
        assert!(LaunchRequest::new(
            &original,
            &command_override,
            Some("fixture-model"),
            None,
            None
        )
        .is_none());
        let mut argv_override = command.clone();
        argv_override.args.push("--unqualified-flag".into());
        assert!(
            LaunchRequest::new(&original, &argv_override, Some("fixture-model"), None, None)
                .is_none()
        );
    }

    #[test]
    fn antigravity_terminal_request_accepts_only_exact_source_resume() {
        let original = harness();
        let id = "12345678-1234-1234-1234-123456789abc";
        let command = original
            .build_resume(
                crate::harness::ResumeStrategy::Exact,
                "/repo",
                Some(id),
                None,
                Some("fixture-model"),
            )
            .unwrap();
        assert!(
            LaunchRequest::new(&original, &command, Some("fixture-model"), Some(id), None)
                .is_some()
        );
        assert!(
            LaunchRequest::new(&original, &command, Some("fixture-model"), None, None).is_none()
        );
        assert!(LaunchRequest::new(
            &original,
            &command,
            Some("fixture-model"),
            Some("other"),
            None
        )
        .is_none());
    }

    #[test]
    fn antigravity_terminal_request_compares_the_source_initial_prompt_exactly() {
        let original = harness();
        let mut command = original.build_launch("/repo", None);
        command
            .args
            .push("--prompt-interactive=fixture task".into());
        assert!(
            LaunchRequest::new(&original, &command, None, None, Some("fixture task")).is_some()
        );
        assert!(LaunchRequest::new(&original, &command, None, None, Some("other task")).is_none());
        assert!(LaunchRequest::new(&original, &command, None, None, None).is_none());
    }

    #[test]
    fn antigravity_terminal_version_evidence_requires_the_entire_exact_version() {
        for version in ["1.5.16", "1.5.16\n", "1.5.17", "1.5.17\n"] {
            assert!(qualified_version(version, QUALIFIED_VERSIONS));
        }
        for version in [
            "1.5.15",
            "1.5.18",
            "1.5.16-beta",
            "1.5.17-beta",
            "agy 1.5.16",
            "warning\n1.5.16",
            "agy 1.5.17",
            "warning\n1.5.17",
            "",
        ] {
            assert!(!qualified_version(version, QUALIFIED_VERSIONS));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn antigravity_qualified_probe_pins_canonical_executable_and_falls_back_without_mutation()
    {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("fake-avy");
        let link = root.path().join("fake-link");
        symlink(&executable, &link).unwrap();
        for (body, accepted) in [
            ("printf '1.5.16\\n'", true),
            ("printf '1.5.16\\n'; exit 1", false),
            ("printf '1.5.16-beta\\n'", false),
            ("printf 'warning' >&2; printf '1.5.16\\n'", false),
            ("printf '1.5.15\\n'", false),
            ("printf '1.5.17\\n'", true),
            ("printf '1.5.17\\n'; exit 1", false),
            ("printf '1.5.17-beta\\n'", false),
            ("printf 'warning' >&2; printf '1.5.17\\n'", false),
            ("printf '1.5.18\\n'", false),
            ("sleep 4", false),
        ] {
            std::fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut command = CommandSpec {
                program: link.to_str().unwrap().into(),
                args: vec![],
                cwd: root.path().to_str().unwrap().into(),
            };
            let before = command.clone();
            assert_eq!(
                prepare_fixture(&mut command).await.is_some(),
                accepted,
                "{body}"
            );
            if accepted {
                assert_eq!(
                    std::path::Path::new(&command.program),
                    std::fs::canonicalize(&executable).unwrap()
                );
                for (proof, platform, changed_argv) in [
                    (None, true, false),
                    (
                        Some(LaunchRequest {
                            command: before.clone(),
                        }),
                        false,
                        false,
                    ),
                    (
                        Some(LaunchRequest {
                            command: before.clone(),
                        }),
                        true,
                        true,
                    ),
                ] {
                    let mut rejected = before.clone();
                    if changed_argv {
                        rejected.args.push("--unqualified-flag".into());
                    }
                    let unchanged = rejected.clone();
                    assert!(
                        prepare_qualified(proof, &mut rejected, platform, QUALIFIED_VERSIONS)
                            .await
                            .is_none()
                    );
                    assert_eq!(rejected, unchanged);
                }
            } else {
                assert_eq!(command, before);
            }
        }
    }

    #[tokio::test]
    async fn antigravity_native_profile_requires_source_proof() {
        let original = harness();
        let mut command = original.build_launch("/repo", None);
        let before = command.clone();
        assert!(prepare(None, &mut command).await.is_none());
        assert_eq!(command, before);
    }
}
