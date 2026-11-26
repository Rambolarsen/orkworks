//! Opt-in Windows image qualification. Production builds do not include this module.
use super::{has_codex_image_option, ImageBundle, ImageDelivery};
use crate::{
    harness::{
        definition::{BuiltinDocument, HarnessUserDocument, LaunchCapability, EMBEDDED_BUILTINS},
        detect::{probe_image_fixture_version, probe_tool_help},
        registry::resolve_document,
    },
    AppState,
};
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

tokio::task_local! {
    pub(super) static FIXTURE: Arc<Fixture>;
}

pub(crate) struct Fixture {
    command: String,
    deny_reads: bool,
    read_lock: Mutex<Option<File>>,
}

impl Fixture {
    pub(super) fn new(command: String, deny_reads: bool) -> Self {
        Self {
            command,
            deny_reads,
            read_lock: Mutex::new(None),
        }
    }
}

pub(super) async fn delivery(harness: &str, command: &str) -> Option<ImageDelivery> {
    let active = FIXTURE
        .try_with(|fixture| fixture.command == command)
        .unwrap_or(false);
    if !active || harness != "codex" {
        return None;
    }
    let path = Path::new(command);
    let version = probe_image_fixture_version(path).await?;
    if version.trim() != "codex-cli 0.172.0" {
        return None;
    }
    has_codex_image_option(&probe_tool_help(path).await?).then_some(ImageDelivery::NativeCodex)
}

pub(crate) fn capture_fixture() -> Option<Arc<Fixture>> {
    FIXTURE.try_with(Arc::clone).ok()
}

impl Fixture {
    pub(crate) fn observe_bundle(&self, bundle: &ImageBundle) -> io::Result<Vec<File>> {
        if self.deny_reads {
            let image = bundle
                .paths()
                .first()
                .ok_or_else(|| io::Error::other("Missing fixture image"))?;
            let lock = OpenOptions::new().read(true).share_mode(0).open(image)?;
            *self.read_lock.lock().expect("qualification read lock") = Some(lock);
        }
        bundle.verify_dispatch_paths()
    }
}

pub(super) fn release_read_lock() {
    let _ = FIXTURE.try_with(|fixture| {
        fixture
            .read_lock
            .lock()
            .expect("qualification read lock")
            .take();
    });
}

pub(super) fn configure(state: &Arc<AppState>) -> String {
    let path = PathBuf::from(
        std::env::var_os("ORKWORKS_IMAGE_FIXTURE_CODEX")
            .expect("set ORKWORKS_IMAGE_FIXTURE_CODEX to the installed native Codex .exe"),
    );
    assert!(path.is_absolute() && path.is_file());
    assert!(path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")));
    let command = path
        .into_os_string()
        .into_string()
        .expect("native executable path must be Unicode");
    let mut builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
    let codex = builtins
        .builtins
        .iter_mut()
        .find(|tool| tool.id == "codex")
        .unwrap();
    let LaunchCapability::CommandTemplate {
        command: executable,
        ..
    } = &mut codex.launch
    else {
        panic!("Codex fixture requires the built-in native launch template");
    };
    *executable = command.clone();
    let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
    *state.harness_catalog.write().unwrap() = Arc::new(registry);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_image_is_rejected_by_preflight() {
        let directory = tempfile::tempdir().unwrap();
        let bundle = ImageBundle::publish(
            directory.path(),
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            vec![crate::image_context::ValidatedImage {
                bytes: b"synthetic image".to_vec(),
                extension: "png",
            }],
        )
        .unwrap();
        let fixture = Fixture::new("unused".into(), true);
        let error = fixture.observe_bundle(&bundle).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(32));
    }
}
