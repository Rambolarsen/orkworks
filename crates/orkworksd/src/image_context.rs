//! Bounded image ingestion and private files owned by an ordinary session.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::PathBuf};

pub(crate) const BODY_LIMIT: usize = 28 * 1024 * 1024;
const PER_FILE: usize = 5 * 1024 * 1024;
const TOTAL: usize = 20 * 1024 * 1024;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImageInput {
    pub name: String,
    pub mime_type: String,
    pub data_base64: String,
}
pub(crate) struct ValidatedImage {
    bytes: Vec<u8>,
    extension: &'static str,
}
pub(crate) fn validate(
    inputs: &[ImageInput],
    prompt: &str,
) -> Result<Vec<ValidatedImage>, &'static str> {
    if inputs.len() > 8 || prompt.len() > 65536 {
        return Err("Image requests allow eight images and 64 KiB of prompt text.");
    }
    let mut total = 0usize;
    let mut pixels = 0u64;
    let mut result = Vec::with_capacity(inputs.len());
    for input in inputs {
        if input.name.len() > 255 || input.data_base64.len() > PER_FILE.div_ceil(3) * 4 {
            return Err("Image name or data is too large.");
        }
        let bytes = STANDARD
            .decode(&input.data_base64)
            .map_err(|_| "Invalid image encoding.")?;
        total += bytes.len();
        if bytes.is_empty() || bytes.len() > PER_FILE || total > TOTAL {
            return Err("Images allow 5 MiB each and 20 MiB total.");
        }
        let format = image::guess_format(&bytes).map_err(|_| "Use PNG, JPEG, or WebP images.")?;
        let (mime, extension) = match format {
            image::ImageFormat::Png => ("image/png", "png"),
            image::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
            image::ImageFormat::WebP => ("image/webp", "webp"),
            _ => return Err("Use PNG, JPEG, or WebP images."),
        };
        if input.mime_type != mime {
            return Err("Image type does not match its contents.");
        }
        let reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
        let (width, height) = reader
            .into_dimensions()
            .map_err(|_| "Cannot read image dimensions.")?;
        let count = u64::from(width) * u64::from(height);
        pixels += count;
        if count == 0 || count > 16_000_000 || pixels > 40_000_000 {
            return Err(
                "Image dimensions exceed the 16 megapixel per-image or 40 megapixel total limit.",
            );
        }
        let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(width);
        limits.max_image_height = Some(height);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        // Drop each decoded surface before validating the next image.
        drop(
            reader
                .decode()
                .map_err(|_| "Image is corrupt or cannot be decoded within memory limits.")?,
        );
        result.push(ValidatedImage { bytes, extension });
    }
    Ok(result)
}

mod storage;
pub(crate) use storage::recover;
pub(crate) use storage::{prepare_cleanup, ImageBundle};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageDelivery {
    NativeCodex,
}
pub(crate) async fn delivery(harness: &str, command: &str) -> Option<ImageDelivery> {
    // Exact installed qualification rows. Future versions/platforms need a fresh fixture.
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return None;
    }
    let (qualified_versions, route) = match harness {
        "codex" => ([(0, 163, 0), (0, 164, 0)], ImageDelivery::NativeCodex),
        _ => return None,
    };
    let tool = crate::harness::detect::probe_installed_tool(command)?;
    let version = crate::harness::detect::probe_tool_version(&tool.executable).await?;
    qualified_versions
        .contains(&crate::harness::detect::parse_version_token(&version)?)
        .then_some(route)
}
pub(crate) async fn capabilities(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<crate::AppState>>,
) -> axum::Json<serde_json::Value> {
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned")
        .clone();
    let mut supported = Vec::new();
    for id in ["codex"] {
        if let Some(harness) = registry.get(id) {
            if !harness.definition.retired
                && delivery(id, &harness.launch_command()).await.is_some()
            {
                supported.push(id);
            }
        }
    }
    axum::Json(serde_json::json!({"version":1, "supportedHarnesses":supported}))
}
pub(crate) fn apply_delivery(
    route: ImageDelivery,
    command: &mut crate::harness::CommandSpec,
    prompt: &str,
    paths: &[PathBuf],
) -> String {
    let prompt = if prompt.trim().is_empty() {
        "Describe the attached images."
    } else {
        prompt
    };
    match route {
        ImageDelivery::NativeCodex => {
            for path in paths {
                command.args.push(format!("--image={}", path.display()));
            }
            command.args.push("--".into());
            command.args.push(prompt.to_owned());
            prompt.to_owned()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    fn remove_bundle(root: &std::path::Path, id: &str) -> std::io::Result<()> {
        if let Some(cleanup) = prepare_cleanup(root, id)? {
            cleanup.remove()?;
        }
        Ok(())
    }
    fn png() -> ImageInput {
        // Real 1x1 PNG, used independently of the decoder under test.
        ImageInput { name: "../a.png".into(), mime_type: "image/png".into(), data_base64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=".into() }
    }
    #[test]
    fn validates_real_bytes_and_rejects_corruption_and_spoofed_type() {
        let images = validate(&[png()], "inspect").unwrap();
        assert_eq!(images[0].extension, "png");
        let mut bad = png();
        bad.data_base64 = base64::engine::general_purpose::STANDARD.encode(b"not an image");
        assert!(validate(&[bad], "inspect").is_err());
        let mut bad = png();
        bad.mime_type = "image/jpeg".into();
        assert!(validate(&[bad], "inspect").is_err());
        let mut bad = png();
        bad.name = "x".repeat(256);
        assert!(validate(&[bad], "inspect").is_err());
    }
    #[test]
    #[cfg(unix)]
    fn private_bundle_uses_generated_paths_and_rolls_back_unaccepted_files() {
        let root = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let images = validate(&[png()], "inspect").unwrap();
        let bundle = ImageBundle::publish(root.path(), &id, images).unwrap();
        let path = bundle.paths()[0].clone();
        assert_eq!(path.file_name().unwrap(), "0.png");
        assert!(path.starts_with(root.path()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(bundle);
        assert!(!path.exists());
    }
    #[test]
    #[cfg(unix)]
    fn accepted_bundle_survives_runtime_and_is_removed_with_owner() {
        let root = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let mut bundle =
            ImageBundle::publish(root.path(), &id, validate(&[png()], "inspect").unwrap()).unwrap();
        let path = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);
        assert!(path.exists());
        remove_bundle(root.path(), &id).unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn limits_apply_before_decoding_and_reject_unbounded_prompt() {
        assert!(validate(&vec![png(); 9], "inspect").is_err());
        assert!(validate(&[png()], &"x".repeat(65537)).is_err());
        assert!(remove_bundle(std::path::Path::new("/tmp"), "../escape").is_err());
    }
    #[tokio::test]
    async fn image_creation_requires_main_authentication_before_admission() {
        let _token = crate::test_support::open_plan_token::set("image-context-test-token").await;
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let app = crate::build_router(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let capability: serde_json::Value = client
            .get(format!("{url}/session-image-context"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(capability["version"], 1);
        let request = serde_json::json!({"harnessId":"generic-shell","initialImages":[png()]});
        let response = client
            .post(format!("{url}/sessions"))
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(state.sessions.lock().unwrap().is_empty());
        let response = client
            .post(format!("{url}/sessions"))
            .header("x-orkworks-open-plan-token", "image-context-test-token")
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
        assert!(state.sessions.lock().unwrap().is_empty());
        server.abort();
    }
    #[test]
    #[cfg(unix)]
    fn recovery_preserves_retained_or_unreadable_metadata_and_discards_owned_orphans() {
        let root = tempfile::tempdir().unwrap();
        let metadata = crate::metadata::MetadataStore::new(root.path());
        let id = uuid::Uuid::new_v4().to_string();
        let mut bundle =
            ImageBundle::publish(root.path(), &id, validate(&[png()], "inspect").unwrap()).unwrap();
        let path = bundle.paths()[0].clone();
        bundle.accept();
        drop(bundle);
        std::fs::create_dir_all(metadata.sessions_dir()).unwrap();
        std::fs::write(
            metadata.sessions_dir().join(format!("{id}.json")),
            "corrupt metadata",
        )
        .unwrap();
        recover(root.path(), &metadata).unwrap();
        assert!(path.exists());
        std::fs::remove_file(metadata.sessions_dir().join(format!("{id}.json"))).unwrap();
        recover(root.path(), &metadata).unwrap();
        assert!(!path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn rollback_remains_in_the_pinned_directory_after_path_replacement() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let bundle =
            ImageBundle::publish(root.path(), &id, validate(&[png()], "inspect").unwrap()).unwrap();
        std::fs::create_dir(outside.path().join(&id)).unwrap();
        std::fs::write(outside.path().join(&id).join("keep"), "sentinel").unwrap();
        std::fs::rename(
            root.path().join("session-images"),
            root.path().join("original"),
        )
        .unwrap();
        symlink(outside.path(), root.path().join("session-images")).unwrap();
        drop(bundle);
        assert_eq!(
            std::fs::read_to_string(outside.path().join(&id).join("keep")).unwrap(),
            "sentinel"
        );
        assert!(!root.path().join("original").join(id).exists());
    }
    #[test]
    fn native_image_arguments_own_the_text_prompt_and_preserve_literal_data() {
        let mut command = crate::harness::CommandSpec {
            program: "codex".into(),
            args: vec!["--model".into(), "example".into()],
            cwd: "/tmp".into(),
        };
        let prompt = "--literal prompt; $(touch forbidden)";
        assert_eq!(
            apply_delivery(
                ImageDelivery::NativeCodex,
                &mut command,
                prompt,
                &[PathBuf::from("/tmp/a b/é.png")]
            ),
            prompt
        );
        assert_eq!(
            command.args,
            vec![
                "--model",
                "example",
                "--image=/tmp/a b/é.png",
                "--",
                "--literal prompt; $(touch forbidden)"
            ]
        );
    }
    #[test]
    #[cfg(unix)]
    fn contended_cleanup_preserves_metadata_and_can_be_retried() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_ref().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let storage = ws.metadata.root_path();
        ws.metadata
            .write_session(&crate::test_support::test_session_metadata(
                &id,
                "image session",
                root.path().display().to_string(),
                "ended",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ));
        let mut old =
            ImageBundle::publish(&storage, &id, validate(&[png()], "inspect").unwrap()).unwrap();
        let image = old.paths()[0].clone();
        old.accept();
        drop(old);
        let publishing = ImageBundle::publish(
            &storage,
            &uuid::Uuid::new_v4().to_string(),
            validate(&[png()], "inspect").unwrap(),
        )
        .unwrap();
        assert!(crate::runtime::retention::delete_session_evidence(ws, &id, |_| Ok(())).is_err());
        assert!(
            ws.metadata.session_file_exists(&id),
            "Cleanup contention must preserve the owning metadata"
        );
        assert!(image.exists());
        drop(publishing);
        crate::runtime::retention::delete_session_evidence(ws, &id, |_| Ok(())).unwrap();
        assert!(!ws.metadata.session_file_exists(&id));
        assert!(!image.exists());
    }
    #[test]
    #[cfg(unix)]
    fn failed_bundle_removal_preserves_metadata_for_retry() {
        let root = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let workspace = state.workspace.lock().unwrap();
        let ws = workspace.as_ref().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        ws.metadata
            .write_session(&crate::test_support::test_session_metadata(
                &id,
                "image session",
                root.path().display().to_string(),
                "ended",
                "2026-01-01T00:00:00Z",
                "2026-01-01T00:00:00Z",
            ));
        let mut bundle = ImageBundle::publish(
            &ws.metadata.root_path(),
            &id,
            validate(&[png()], "inspect").unwrap(),
        )
        .unwrap();
        let directory = bundle.paths()[0].parent().unwrap().to_path_buf();
        bundle.accept();
        drop(bundle);
        // Removal deliberately refuses to recurse into unexpected child directories.
        let obstruction = directory.join("unexpected-directory");
        std::fs::create_dir(&obstruction).unwrap();
        assert!(crate::runtime::retention::delete_session_evidence(ws, &id, |_| Ok(())).is_err());
        assert!(
            ws.metadata.session_file_exists(&id),
            "I/O failure must preserve durable cleanup ownership"
        );
        std::fs::remove_dir(obstruction).unwrap();
        crate::runtime::retention::delete_session_evidence(ws, &id, |_| Ok(())).unwrap();
        assert!(!ws.metadata.session_file_exists(&id));
        assert!(!directory.exists());
    }
    /// Explicitly opt in: uses the installed tool/account for a tiny visual task.
    #[tokio::test]
    #[ignore = "requires an installed, authenticated coding tool and a trusted checkout"]
    async fn qualify_installed_image_delivery() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_test_writer()
            .try_init();
        let harness =
            std::env::var("ORKWORKS_IMAGE_FIXTURE_HARNESS").unwrap_or_else(|_| "codex".into());
        let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        let root = tempfile::Builder::new()
            .prefix("image qualification é ")
            .tempdir_in(target)
            .unwrap();
        let state = crate::test_support::test_app_state_with_workspace(root.path());
        let make_image = |circle: [u8; 3], rectangle: [u8; 3]| {
            let mut image = image::RgbImage::from_pixel(256, 128, image::Rgb([255, 255, 255]));
            for y in 0i32..128 {
                for x in 0i32..256 {
                    if (x - 64).pow(2) + (y - 64).pow(2) < 40 * 40 {
                        image.put_pixel(x as u32, y as u32, image::Rgb(circle));
                    }
                    if (160..224).contains(&x) && (32..96).contains(&y) {
                        image.put_pixel(x as u32, y as u32, image::Rgb(rectangle));
                    }
                }
            }
            let mut bytes = Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image)
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            ImageInput {
                name: "visual.png".into(),
                mime_type: "image/png".into(),
                data_base64: STANDARD.encode(bytes.into_inner()),
            }
        };
        for image_only in [false, true] {
            let inputs = if image_only {
                vec![make_image([255, 0, 0], [0, 180, 0])]
            } else {
                vec![
                    make_image([255, 0, 0], [0, 180, 0]),
                    make_image([0, 0, 255], [255, 160, 0]),
                ]
            };
            let info = crate::session_application::SessionApplication::new(state.clone()).create_session(crate::session_application::CreateSessionCommand {
            reasoning_effort: None,
                harness_id:Some(harness.clone()),model:None,
                initial_prompt: if image_only {None} else {Some("For each image in order, give the circle color then rectangle color. Reply with only those four color names.".into())},
                initial_images:inputs,expected_workspace_path:Some(root.path().display().to_string()),
            }).await.unwrap();
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(90);
            let mut saw = false;
            let mut last_output = String::new();
            while tokio::time::Instant::now() < deadline {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let (output, status) = state
                    .sessions
                    .lock()
                    .unwrap()
                    .get(&info.id)
                    .map(|h| {
                        (
                            crate::peon::strip_ansi(&String::from_utf8_lossy(
                                &h.runtime
                                    .replay
                                    .snapshot()
                                    .into_iter()
                                    .flat_map(|(_, bytes)| bytes)
                                    .collect::<Vec<_>>(),
                            )),
                            h.info.status.clone(),
                        )
                    })
                    .unwrap_or_default();
                if status == "error" || status == "ended" {
                    last_output = format!("status={status}\n{output}");
                    break;
                }
                let lower = output.to_lowercase();
                let expected = if image_only {
                    vec!["red", "green"]
                } else {
                    vec!["red", "green", "blue", "orange"]
                };
                if expected.iter().all(|color| lower.contains(color))
                    && (image_only || !lower.contains("what would you like"))
                {
                    saw = true;
                    last_output = output;
                    break;
                }
                last_output = output;
            }
            crate::session_application::SessionApplication::new(state.clone())
                .delete_session(&info.id)
                .await
                .unwrap();
            for _ in 0..40 {
                if !state.session_pids.lock().unwrap().contains_key(&info.id) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            println!("qualification {harness} image_only={image_only} output:\n{last_output}");
            assert!(
                saw,
                "Model did not identify fixture colors; no qualification granted"
            );
        }
    }
}
