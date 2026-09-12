use crate::harness::{definition::DefinitionOrigin, inference::InferenceCapability};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct AdapterIdentity {
    harness_id: String,
    digest: String,
    resolved_path: PathBuf,
}

impl AdapterIdentity {
    pub(crate) fn resolve(
        harness_id: &str,
        origin: DefinitionOrigin,
        capability: &InferenceCapability,
        path: Option<&OsStr>,
    ) -> Result<Self, String> {
        if harness_id.is_empty()
            || harness_id.len() > 256
            || harness_id.chars().any(char::is_control)
        {
            return Err("invalid inference harness identity".into());
        }
        let command = Path::new(capability.command());
        let resolved_path = if command.is_absolute() {
            executable_target(command)
        } else {
            path.and_then(|path| {
                std::env::split_paths(path)
                    .filter(|entry| entry.is_absolute())
                    .find_map(|entry| {
                        let candidate = entry.join(command);
                        #[cfg(windows)]
                        let candidate = if candidate.extension().is_none() {
                            candidate.with_extension("exe")
                        } else {
                            candidate
                        };
                        executable_target(&candidate)
                    })
            })
        }
        .ok_or("inference executable is unavailable")?;
        // Reject non-UTF-8 paths rather than collapsing distinct OS paths via lossy conversion.
        let path_text = resolved_path
            .to_str()
            .ok_or("inference executable path is not UTF-8")?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Record<'a> {
            version: u8,
            harness_id: &'a str,
            origin: &'a str,
            capability: &'a InferenceCapability,
            resolved_path: &'a str,
        }
        let record = Record {
            version: 1,
            harness_id,
            origin: match origin {
                DefinitionOrigin::Builtin => "builtin",
                DefinitionOrigin::Override => "override",
                DefinitionOrigin::Custom => "custom",
            },
            capability,
            resolved_path: path_text,
        };
        let bytes = serde_json::to_vec(&record).map_err(|_| "cannot encode inference identity")?;
        Ok(Self {
            harness_id: harness_id.into(),
            digest: format!("{:x}", Sha256::digest(bytes)),
            resolved_path,
        })
    }

    pub(crate) fn harness_id(&self) -> &str {
        &self.harness_id
    }
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn resolved_path(&self) -> &Path {
        &self.resolved_path
    }
}

fn executable_target(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    let metadata = canonical.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    #[cfg(windows)]
    if !canonical
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    Some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn capability(command: &str) -> InferenceCapability {
        serde_json::from_value(json!({"kind":"command","command":command,
            "args":["--model","{model}"],"input":"stdin","output":"result-json-v1"}))
        .unwrap()
    }

    #[test]
    fn defaulted_identity_is_stable_but_execution_changes_invalidate_it() {
        let exe = std::env::current_exe().unwrap();
        let cap = capability(exe.to_str().unwrap());
        let first =
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &cap, None).unwrap();
        let mut raw = serde_json::to_value(&cap).unwrap();
        raw["timeoutSecs"] = json!(60);
        let explicit = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            first,
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &explicit, None).unwrap()
        );
        raw["args"] = json!(["--different", "{model}"]);
        let changed = serde_json::from_value(raw).unwrap();
        assert_ne!(
            first,
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &changed, None).unwrap()
        );
        assert_ne!(
            first,
            AdapterIdentity::resolve("other", DefinitionOrigin::Custom, &cap, None).unwrap()
        );
        assert_ne!(
            first,
            AdapterIdentity::resolve("custom", DefinitionOrigin::Override, &cap, None).unwrap()
        );
        assert_eq!(first.resolved_path, exe.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn path_order_selects_first_executable_and_skips_non_executable_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        for directory in [&first, &second] {
            std::fs::create_dir(directory).unwrap();
            std::fs::write(directory.join("tool"), b"not executed").unwrap();
            std::fs::set_permissions(
                directory.join("tool"),
                std::fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        let path = std::env::join_paths([&first, &second]).unwrap();
        let cap = capability("tool");
        let resolved =
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &cap, Some(&path))
                .unwrap();
        assert_eq!(
            resolved.resolved_path(),
            first.join("tool").canonicalize().unwrap()
        );
        std::fs::set_permissions(first.join("tool"), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let resolved =
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &cap, Some(&path))
                .unwrap();
        assert_eq!(
            resolved.resolved_path(),
            second.join("tool").canonicalize().unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolves_symlinks_and_absolute_path_entries_without_running_commands() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        let marker = dir.path().join("executed");
        std::fs::write(
            &target,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&target, dir.path().join("tool")).unwrap();
        let path = std::env::join_paths([
            std::path::Path::new(""),
            std::path::Path::new("relative"),
            dir.path(),
        ])
        .unwrap();
        let cap = capability("tool");
        let identity =
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &cap, Some(&path))
                .unwrap();
        assert_eq!(identity.resolved_path, target.canonicalize().unwrap());
        assert!(!marker.exists());
        assert!(AdapterIdentity::resolve(
            "custom",
            DefinitionOrigin::Custom,
            &cap,
            Some(OsStr::new(":relative"))
        )
        .is_err());
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            AdapterIdentity::resolve("custom", DefinitionOrigin::Custom, &cap, Some(&path))
                .is_err()
        );
    }
}
