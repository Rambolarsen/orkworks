//! Explicit custom executable trust, separate from editable harness definitions.
mod identity;
pub(crate) use identity::AdapterIdentity;

use super::runtime::{persistence_file_lock, PersistenceGuard, PERSISTENCE_LOCK};
use crate::harness::definition::parse_strict_json;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
};

const MAX_TRUST_BYTES: usize = 1024 * 1024;
const MAX_GRANTS: usize = 1024;

#[derive(Clone, Debug)]
pub(crate) struct TrustRevision {
    pub generation: u64,
    pub digest: String,
}
pub(crate) struct TrustStatus {
    pub revision: TrustRevision,
    pub approved: bool,
}
pub(crate) struct InferenceTrustStore {
    root: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrustDocument {
    version: u8,
    generation: u64,
    grants: BTreeMap<String, Grant>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Grant {
    digest: String,
    resolved_path: PathBuf,
}

impl InferenceTrustStore {
    pub(crate) fn generation(&self) -> Result<u64, String> {
        self.locked(|| Ok(self.read()?.generation))
    }
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Inspect a freshly resolved identity. This never grants permission or probes a process.
    pub(crate) fn inspect(&self, identity: &AdapterIdentity) -> Result<TrustStatus, String> {
        let guard = PersistenceGuard::acquire(&self.root)?;
        self.inspect_guarded(&guard, identity)
    }

    /// Reuse a held runtime lease rather than recursively acquiring its mutex.
    pub(super) fn inspect_guarded(
        &self,
        guard: &PersistenceGuard<'_>,
        identity: &AdapterIdentity,
    ) -> Result<TrustStatus, String> {
        if !guard.protects(&self.root) {
            return Err("inference trust guard belongs to another root".into());
        }
        let document = self.read()?;
        Ok(TrustStatus {
            revision: TrustRevision {
                generation: document.generation,
                digest: identity.digest().into(),
            },
            approved: document
                .grants
                .get(identity.harness_id())
                .is_some_and(|grant| {
                    grant.digest == identity.digest()
                        && grant.resolved_path == identity.resolved_path()
                }),
        })
    }

    /// Caller must resolve current registry identity under its mutation boundary before approval.
    /// The expected revision is the identity and trust generation previously shown to the user.
    pub(crate) fn approve(
        &self,
        identity: &AdapterIdentity,
        expected: &TrustRevision,
    ) -> Result<(), String> {
        self.locked(|| {
            let mut document = self.read()?;
            if expected.digest != identity.digest() || expected.generation != document.generation {
                return Err("inference trust revision conflict".into());
            }
            document.grants.insert(
                identity.harness_id().into(),
                Grant {
                    digest: identity.digest().into(),
                    resolved_path: identity.resolved_path().into(),
                },
            );
            self.persist_next(document)
        })
    }

    /// Revocation does not require the executable to remain installed or resolvable.
    pub(crate) fn revoke(&self, harness_id: &str, generation: u64) -> Result<(), String> {
        self.locked(|| {
            let mut document = self.read()?;
            if generation != document.generation {
                return Err("inference trust revision conflict".into());
            }
            document.grants.remove(harness_id);
            self.persist_next(document)
        })
    }

    fn locked<T>(&self, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let _process = PERSISTENCE_LOCK
            .lock()
            .map_err(|_| "inference trust lock unavailable")?;
        let _file = persistence_file_lock(&self.root)?;
        operation()
    }

    fn read(&self) -> Result<TrustDocument, String> {
        let path = self.root.join("inference-trust.json");
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Err("inference trust data is not a regular file".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TrustDocument {
                    version: 1,
                    generation: 0,
                    grants: BTreeMap::new(),
                });
            }
            Err(_) => return Err("inference trust data is unreadable".into()),
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // Do not follow a swapped symlink or block opening a swapped FIFO.
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(
                windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT,
            );
        }
        let file = options
            .open(&path)
            .map_err(|_| "inference trust data is unreadable")?;
        if !file
            .metadata()
            .map_err(|_| "inference trust data is unreadable")?
            .is_file()
        {
            return Err("inference trust data is not a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take((MAX_TRUST_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "inference trust data is unreadable")?;
        let document: TrustDocument = parse_strict_json(&bytes, MAX_TRUST_BYTES)
            .map_err(|_| "inference trust data is malformed")?;
        validate_document(&document)?;
        Ok(document)
    }

    fn persist_next(&self, mut document: TrustDocument) -> Result<(), String> {
        document.generation = document
            .generation
            .checked_add(1)
            .ok_or("inference trust generation exhausted")?;
        validate_document(&document)?;
        let bytes = serde_json::to_vec(&document).map_err(|_| "cannot encode inference trust")?;
        if bytes.len() > MAX_TRUST_BYTES {
            return Err("inference trust data exceeds size limit".into());
        }
        // A unique private file avoids fixed-name temporary-file collisions and is removed on error.
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|_| "cannot create inference trust file")?;
        temporary
            .write_all(&bytes)
            .map_err(|_| "cannot write inference trust file")?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| "cannot sync inference trust file")?;
        let temporary = temporary.into_temp_path();
        let target = self.root.join("inference-trust.json");
        crate::harness::integration::atomic_replace(&temporary, &target, target.exists())
            .map_err(|_| "cannot replace inference trust file")?;
        Ok(())
    }
}

fn validate_document(document: &TrustDocument) -> Result<(), String> {
    if document.version != 1
        || document.grants.len() > MAX_GRANTS
        || document.grants.iter().any(|(id, grant)| {
            id.is_empty()
                || id.len() > 256
                || id.chars().any(char::is_control)
                || grant.digest.len() != 64
                || !grant
                    .digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !normalized_absolute_path(&grant.resolved_path)
        })
    {
        return Err("inference trust data is invalid or unsupported".into());
    }
    Ok(())
}

fn normalized_absolute_path(path: &std::path::Path) -> bool {
    // Do not canonicalize against the live filesystem: uninstalling a tool must not prevent revocation.
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|text| !text.chars().any(char::is_control))
        && !path.components().any(|part| {
            matches!(
                part,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
        && path.components().collect::<PathBuf>().as_os_str() == path.as_os_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::{definition::DefinitionOrigin, inference::InferenceCapability};
    use serde_json::json;

    fn identity(id: &str) -> AdapterIdentity {
        let cap: InferenceCapability = serde_json::from_value(json!({"kind":"command",
            "command":std::env::current_exe().unwrap(),"args":["{model}"],
            "input":"stdin","output":"result-json-v1"}))
        .unwrap();
        AdapterIdentity::resolve(id, DefinitionOrigin::Custom, &cap, None).unwrap()
    }

    #[test]
    fn grants_survive_restart_and_revoke_reapprove_invalidates_old_revisions() {
        let dir = tempfile::tempdir().unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        let identity = identity("custom");
        let initial = store.inspect(&identity).unwrap();
        assert!(!initial.approved);
        assert!(!dir.path().join("inference-trust.json").exists());
        store.approve(&identity, &initial.revision).unwrap();
        let reopened = InferenceTrustStore::new(dir.path().into());
        let approved = reopened.inspect(&identity).unwrap();
        assert!(approved.approved);
        assert_eq!(approved.revision.generation, 1);
        assert!(store.approve(&identity, &initial.revision).is_err());
        reopened.revoke("custom", 1).unwrap();
        let revoked = store.inspect(&identity).unwrap();
        assert!(!revoked.approved);
        assert_eq!(revoked.revision.generation, 2);
        store.approve(&identity, &revoked.revision).unwrap();
        assert_eq!(reopened.inspect(&identity).unwrap().revision.generation, 3);
        assert!(reopened.revoke("custom", 1).is_err());
    }

    #[test]
    fn mismatched_identity_cannot_reuse_a_reviewed_revision() {
        let dir = tempfile::tempdir().unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        let first = identity("first");
        let other = identity("other");
        let revision = store.inspect(&first).unwrap().revision;
        assert!(store.approve(&other, &revision).is_err());
        assert!(!dir.path().join("inference-trust.json").exists());
    }

    #[test]
    fn path_resolution_change_invalidates_a_grant_for_the_same_definition() {
        let dir = tempfile::tempdir().unwrap();
        let first_dir = dir.path().join("first");
        let second_dir = dir.path().join("second");
        for directory in [&first_dir, &second_dir] {
            std::fs::create_dir(directory).unwrap();
            let executable = directory.join("adapter.exe");
            std::fs::write(&executable, b"not executed by identity inspection").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
        }
        let cap: InferenceCapability = serde_json::from_value(json!({"kind":"command",
            "command":"adapter.exe","args":["{model}"],"input":"stdin","output":"result-json-v1"}))
        .unwrap();
        let first = AdapterIdentity::resolve(
            "custom",
            DefinitionOrigin::Custom,
            &cap,
            Some(first_dir.as_os_str()),
        )
        .unwrap();
        let second = AdapterIdentity::resolve(
            "custom",
            DefinitionOrigin::Custom,
            &cap,
            Some(second_dir.as_os_str()),
        )
        .unwrap();
        let store = InferenceTrustStore::new(dir.path().join("trust"));
        store
            .approve(&first, &store.inspect(&first).unwrap().revision)
            .unwrap();
        let reviewed = store.inspect(&first).unwrap();
        assert!(reviewed.approved);
        assert!(!store.inspect(&second).unwrap().approved);
        assert!(store.approve(&second, &reviewed.revision).is_err());
        // Revoke by ID still works after both executable files disappear.
        std::fs::remove_file(first_dir.join("adapter.exe")).unwrap();
        std::fs::remove_file(second_dir.join("adapter.exe")).unwrap();
        store
            .revoke("custom", reviewed.revision.generation)
            .unwrap();
        assert!(!store.inspect(&first).unwrap().approved);
    }

    #[test]
    fn malformed_trust_fails_closed_without_overwriting_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        let identity = identity("custom");
        let revision = store.inspect(&identity).unwrap().revision;
        for raw in [
            "broken",
            r#"{"version":2,"generation":0,"grants":{}}"#,
            r#"{"version":1,"generation":0,"grants":{},"trusted":true}"#,
            r#"{"version":1,"generation":0,"generation":1,"grants":{}}"#,
            r#"{"version":1,"generation":0,"grants":{"x":{"digest":"bad","resolvedPath":"relative"}}}"#,
        ] {
            let path = dir.path().join("inference-trust.json");
            std::fs::write(&path, raw).unwrap();
            assert!(store.inspect(&identity).is_err(), "accepted {raw}");
            assert!(store.approve(&identity, &revision).is_err());
            assert!(store.revoke("custom", 0).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        }
    }

    #[test]
    fn concurrent_store_views_cannot_both_approve_the_same_revision() {
        let dir = tempfile::tempdir().unwrap();
        let identity = identity("custom");
        let store = InferenceTrustStore::new(dir.path().into());
        let revision = store.inspect(&identity).unwrap().revision;
        let barrier = std::sync::Barrier::new(2);
        let outcomes = std::thread::scope(|scope| {
            let run = || {
                let view = InferenceTrustStore::new(dir.path().into());
                barrier.wait();
                view.approve(&identity, &revision).is_ok()
            };
            let a = scope.spawn(run);
            let b = scope.spawn(run);
            [a.join().unwrap(), b.join().unwrap()]
        });
        assert_eq!(outcomes.into_iter().filter(|accepted| *accepted).count(), 1);
        assert_eq!(store.inspect(&identity).unwrap().revision.generation, 1);
    }

    #[test]
    #[ignore = "subprocess helper invoked only by the cross-process lock test"]
    fn trust_approval_child() {
        let root = PathBuf::from(std::env::var_os("ORKWORKS_TEST_TRUST_ROOT").expect("test root"));
        let store = InferenceTrustStore::new(root.clone());
        let identity = identity("custom");
        std::fs::write(root.join("child-ready"), b"ready").unwrap();
        store
            .approve(
                &identity,
                &TrustRevision {
                    generation: 0,
                    digest: identity.digest().into(),
                },
            )
            .unwrap();
    }

    #[test]
    fn cross_process_approval_waits_for_taskmaster_file_lock() {
        use std::time::{Duration, Instant};
        let dir = tempfile::tempdir().unwrap();
        let lock = persistence_file_lock(dir.path()).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "taskmaster::inference_trust::tests::trust_approval_child",
                "--ignored",
            ])
            .env("ORKWORKS_TEST_TRUST_ROOT", dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !dir.path().join("child-ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let ready = dir.path().join("child-ready").exists();
        std::thread::sleep(Duration::from_millis(100));
        let blocked = child.try_wait().unwrap().is_none();
        let no_write = !dir.path().join("inference-trust.json").exists();
        drop(lock);
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if Instant::now() >= deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if status.is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
        assert!(
            ready && blocked && no_write,
            "child did not wait for the shared file lock"
        );
        assert!(
            status.is_some_and(|status| status.success()),
            "child failed or timed out"
        );
        assert!(
            InferenceTrustStore::new(dir.path().into())
                .inspect(&identity("custom"))
                .unwrap()
                .approved
        );
    }

    #[test]
    fn oversized_and_exhausted_documents_cannot_be_replaced_by_approval() {
        let dir = tempfile::tempdir().unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        let identity = identity("custom");
        let initial = store.inspect(&identity).unwrap().revision;
        let path = dir.path().join("inference-trust.json");
        let oversized = " ".repeat(MAX_TRUST_BYTES + 1);
        std::fs::write(&path, &oversized).unwrap();
        assert!(store.inspect(&identity).is_err());
        assert!(store.approve(&identity, &initial).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), oversized);
        let exhausted = r#"{"version":1,"generation":18446744073709551615,"grants":{}}"#;
        std::fs::write(&path, exhausted).unwrap();
        let revision = store.inspect(&identity).unwrap().revision;
        assert!(store.approve(&identity, &revision).is_err());
        assert!(store.revoke("custom", revision.generation).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), exhausted);
    }

    #[test]
    fn malformed_absolute_grant_paths_fail_closed_without_filesystem_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        let identity = identity("custom");
        let revision = store.inspect(&identity).unwrap().revision;
        for path in [
            dir.path().join("../tool"),
            dir.path().join("./tool"),
            dir.path().join("tool\0"),
            dir.path().join("tool\n"),
        ] {
            let raw = serde_json::to_vec(&json!({"version":1,"generation":0,
                "grants":{"custom":{"digest":"a".repeat(64),"resolvedPath":path}}}))
            .unwrap();
            let file = dir.path().join("inference-trust.json");
            std::fs::write(&file, &raw).unwrap();
            assert!(store.inspect(&identity).is_err(), "accepted {path:?}");
            assert!(store.approve(&identity, &revision).is_err());
            assert_eq!(std::fs::read(file).unwrap(), raw);
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_trust_is_not_treated_as_owned_approval_state() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("external.json");
        std::fs::write(&target, r#"{"version":1,"generation":0,"grants":{}}"#).unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("inference-trust.json")).unwrap();
        let store = InferenceTrustStore::new(dir.path().into());
        assert!(store.inspect(&identity("custom")).is_err());
    }
}
