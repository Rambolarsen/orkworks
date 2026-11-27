//! Opt-in Windows image qualification. Production builds do not include this module.
use super::ImageBundle;
use crate::{
    harness::{
        definition::{BuiltinDocument, HarnessUserDocument, LaunchCapability, EMBEDDED_BUILTINS},
        registry::resolve_document,
    },
    AppState,
};
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
    sync::{Arc, Mutex},
};

tokio::task_local! {
    pub(super) static FIXTURE: Arc<Fixture>;
}

pub(crate) struct Fixture {
    deny_reads: bool,
    read_lock: Mutex<Option<File>>,
}

impl Fixture {
    pub(super) fn new(deny_reads: bool) -> Self {
        Self {
            deny_reads,
            read_lock: Mutex::new(None),
        }
    }
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

pub(super) fn configure(state: &Arc<AppState>) {
    let mut workspace = state.workspace.lock().unwrap();
    let workspace = workspace.as_mut().expect("qualification workspace");
    workspace.lease = Some(Arc::new(
        crate::workspace_runtime::WorkspaceLease::acquire(&workspace.metadata.root_path())
            .expect("qualification workspace lease"),
    ));

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
    *executable = command;
    let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
    *state.harness_catalog.write().unwrap() = Arc::new(registry);
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
        let fixture = Fixture::new(true);
        let error = fixture.observe_bundle(&bundle).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(32));
    }
}
