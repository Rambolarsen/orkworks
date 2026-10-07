//! Explicitly ignored operator diagnostic. No shipping eligibility or clear bypass.
use super::*;
use std::collections::VecDeque;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
enum Exit {
    Eof,
    Operator,
    Deadline,
    InputRejected,
    RuntimeEnded,
    RuntimeError,
    OutputGap,
    Io,
}

struct TerminalMode {
    fd: RawFd,
    original: libc::termios,
    flags: i32,
}
impl TerminalMode {
    fn enter(fd: RawFd) -> io::Result<Self> {
        // SAFETY: valid caller-held terminal descriptor; checked syscall results.
        unsafe {
            let mut original = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut original) != 0 {
                return Err(io::Error::last_os_error());
            }
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }
            let guard = Self {
                fd,
                original,
                flags,
            };
            let mut raw = original;
            libc::cfmakeraw(&mut raw);
            if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0
                || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(guard)
        }
    }
    fn restore(&self) -> bool {
        // SAFETY: original terminal attributes and flags came from this descriptor.
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) == 0
                && libc::fcntl(self.fd, libc::F_SETFL, self.flags) >= 0
        }
    }
    fn read(&self) -> io::Result<Option<Vec<u8>>> {
        let mut bytes = vec![0u8; 4096];
        // SAFETY: valid terminal descriptor and writable bounded buffer.
        let count = unsafe { libc::read(self.fd, bytes.as_mut_ptr().cast(), bytes.len()) };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Ok(None);
        }
        bytes.truncate(count as usize);
        Ok(Some(bytes))
    }
}
impl Drop for TerminalMode {
    fn drop(&mut self) {
        // SAFETY: caller keeps the terminal descriptor alive until this guard drops.
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
            libc::fcntl(self.fd, libc::F_SETFL, self.flags);
        }
    }
}

async fn bridge(
    state: &Arc<AppState>,
    id: &str,
    claim: &mut AttachmentClaim,
    mut read: impl FnMut() -> io::Result<Option<Vec<u8>>>,
    output: &mut impl Write,
    deadline: Instant,
    resize_fd: Option<RawFd>,
) -> Exit {
    let (identity, sender) = {
        let sessions = state.sessions.lock().unwrap();
        let Some(handle) = sessions.get(id) else {
            return Exit::InputRejected;
        };
        (handle.runtime.identity(), handle.runtime.control_tx.clone())
    };
    for (_, bytes) in &claim.replay_chunks {
        if output.write_all(bytes).is_err() {
            return Exit::Io;
        }
    }
    let mut cursor = claim.replay_to;
    let mut pending = Vec::new();
    let mut last_size = None;
    let mut next_metrics = Instant::now();
    let mut interval = tokio::time::interval(Duration::from_millis(20));
    loop {
        if Instant::now() >= deadline {
            return Exit::Deadline;
        }
        for _ in 0..64 {
            match claim.events.try_recv() {
                Ok(RuntimeEvent::Output {
                    cursor: received,
                    chunk,
                }) => {
                    if received < cursor {
                        continue;
                    }
                    if received != cursor {
                        return Exit::OutputGap;
                    }
                    cursor += 1;
                    if output.write_all(&chunk).is_err() {
                        return Exit::Io;
                    }
                }
                Ok(RuntimeEvent::Ended { .. }) => return Exit::RuntimeEnded,
                Ok(RuntimeEvent::Error { .. }) => return Exit::RuntimeError,
                Err(broadcast::error::TryRecvError::Lagged(_)) => return Exit::OutputGap,
                Err(broadcast::error::TryRecvError::Closed) => return Exit::RuntimeEnded,
                Err(broadcast::error::TryRecvError::Empty) => break,
            }
        }
        if Instant::now() >= next_metrics {
            if print_metrics(state, id, output).is_err() {
                return Exit::Io;
            }
            next_metrics = Instant::now() + Duration::from_secs(2);
        }
        if let Some(size) = resize_fd
            .and_then(terminal_size)
            .filter(|size| Some(*size) != last_size)
        {
            let mut resize = Box::pin(update_runtime_size(state, id, size.0, size.1));
            let resized = tokio::select! {
                result=&mut resize=>result,
                _=tokio::time::sleep_until(tokio::time::Instant::from_std(deadline))=>{
                    let _=send_runtime_command(state,id,RuntimeCommand::Kill).await;
                    let _=tokio::time::timeout(CLEANUP_DEADLINE,&mut resize).await;
                    return Exit::Deadline;
                }
            };
            if resized.is_err() {
                return Exit::Io;
            }
            last_size = Some(size);
        }
        match read() {
            Ok(None) => return Exit::Eof,
            Ok(Some(bytes)) => {
                // Reserved controls are never submitted as conversation input.
                if bytes.contains(&0x1d) {
                    return Exit::Operator;
                }
                if bytes.contains(&0x1c) {
                    if bytes.len() != 1 || !pending.is_empty() {
                        return Exit::Io;
                    }
                    let sessions = state.sessions.lock().unwrap();
                    let Some(tx) = sessions
                        .get(id)
                        .and_then(|h| h.runtime.native_observer_pause.as_ref())
                    else {
                        return Exit::Io;
                    };
                    let paused = *tx.borrow();
                    tx.send_replace(!paused);
                    continue;
                }
                pending.extend(bytes);
                let complete = match std::str::from_utf8(&pending) {
                    Ok(_) => pending.len(),
                    Err(error) if error.error_len().is_none() => error.valid_up_to(),
                    Err(_) => return Exit::Io,
                };
                if complete == 0 {
                    continue;
                }
                let data = String::from_utf8(pending.drain(..complete).collect())
                    .expect("validated UTF-8");
                let before = attention_is_working(state, id);
                let mut delivery = Box::pin(
                    super::super::terminal_runtime::submit_approved_input_to_runtime(
                        state, id, &identity, &sender, data,
                    ),
                );
                let result = tokio::select! {
                    result=&mut delivery => result,
                    _=tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                        let _=send_runtime_command(state,id,RuntimeCommand::Kill).await;
                        // Keep the accepted-input future alive through bounded owned shutdown.
                        let _=tokio::time::timeout(Duration::from_secs(10),&mut delivery).await;
                        return Exit::Deadline;
                    }
                };
                if result.is_err() {
                    return Exit::InputRejected;
                }
                let sink = {
                    state
                        .sessions
                        .lock()
                        .unwrap()
                        .get(id)
                        .and_then(|h| h.runtime.native_diagnostic.clone())
                };
                if let Some(sink) = sink {
                    let after = attention_is_working(state, id);
                    let mut metrics = sink.lock().unwrap();
                    metrics.accepted_input += 1;
                    metrics.input_to_working += u64::from(!before && after);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Exit::Io,
        }
        if output.flush().is_err() {
            return Exit::Io;
        }
        interval.tick().await;
    }
}

fn terminal_size(fd: RawFd) -> Option<(u16, u16)> {
    // SAFETY: valid terminal descriptor and writable winsize destination.
    unsafe {
        let mut size: libc::winsize = std::mem::zeroed();
        (libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) == 0 && size.ws_row > 0 && size.ws_col > 0)
            .then_some((size.ws_row, size.ws_col))
    }
}
fn attention_is_working(state: &Arc<AppState>, id: &str) -> bool {
    state
        .sessions
        .lock()
        .unwrap()
        .get(id)
        .is_some_and(|h| h.info.attention.as_deref() == Some("working"))
}
fn print_metrics(state: &Arc<AppState>, id: &str, output: &mut impl Write) -> io::Result<()> {
    let root_bound = {
        let workspace = state.workspace.lock().unwrap();
        workspace
            .as_ref()
            .and_then(|w| w.metadata.read_session(id))
            .is_some_and(|m| {
                m.harness_session_id_source.as_deref() == Some("codex_hook")
                    && m.resume
                        .as_ref()
                        .is_some_and(|r| r.harness_session_id.is_some())
            })
    };
    let sessions = state.sessions.lock().unwrap();
    let Some(handle) = sessions.get(id) else {
        return Ok(());
    };
    let attention = match handle.info.attention.as_deref() {
        Some("working") => "working",
        Some("needs_you") => "needs_you",
        _ => "other",
    };
    if let Some(sink) = &handle.runtime.native_diagnostic {
        write!(output,"\r\n[diagnostic clear_enabled=false root_bound={} hook_active={} alive={} attention={} {:?}]\r\n",root_bound,handle.active_work_hook,handle.info.lifecycle=="alive",attention,*sink.lock().unwrap())?;
    }
    Ok(())
}

fn terminal_pair() -> (OwnedFd, OwnedFd) {
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: valid output pointers, null default terminal attributes/size.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        0
    );
    // SAFETY: openpty returned two independently owned valid descriptors.
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}
fn attributes(fd: RawFd) -> libc::termios {
    // SAFETY: termios is initialized by successful tcgetattr.
    unsafe {
        let mut value = std::mem::zeroed();
        assert_eq!(libc::tcgetattr(fd, &mut value), 0);
        value
    }
}

#[test]
fn diagnostic_restores_tty_and_flags_on_all_exit_paths() {
    let (_master, slave) = terminal_pair();
    let before = attributes(slave.as_raw_fd());
    // SAFETY: valid owned descriptor; F_GETFL is read-only.
    let flags = unsafe { libc::fcntl(slave.as_raw_fd(), libc::F_GETFL) };
    {
        let _mode = TerminalMode::enter(slave.as_raw_fd()).expect("raw mode setup");
        assert_eq!(attributes(slave.as_raw_fd()).c_lflag & libc::ICANON, 0);
    }
    let after = attributes(slave.as_raw_fd());
    // PENDIN is transient line-discipline state, not a terminal mode setting.
    assert_eq!(
        before.c_lflag & !libc::PENDIN,
        after.c_lflag & !libc::PENDIN
    );
    assert_eq!(before.c_iflag, after.c_iflag);
    assert_eq!(before.c_cc, after.c_cc);
    // SAFETY: valid owned descriptor; F_GETFL is read-only.
    assert_eq!(flags, unsafe {
        libc::fcntl(slave.as_raw_fd(), libc::F_GETFL)
    });
}

#[tokio::test]
async fn diagnostic_eof_deadline_and_rejected_input_are_bounded() {
    for expected in [Exit::Eof, Exit::Deadline, Exit::InputRejected] {
        let id = format!("diagnostic-{}", uuid::Uuid::new_v4());
        let dir = tempfile::tempdir().unwrap();
        let (state, control) = fixture_state(dir.path(), &id);
        drop(control);
        let mut claim = claim_attachment(&state, &id).unwrap();
        let mut inputs = VecDeque::from([if expected == Exit::Eof {
            None
        } else {
            Some(b"a".to_vec())
        }]);
        let deadline = Instant::now()
            + if expected == Exit::Deadline {
                Duration::ZERO
            } else {
                Duration::from_secs(1)
            };
        let (_master, slave) = terminal_pair();
        let before = attributes(slave.as_raw_fd());
        let result = {
            let _mode = TerminalMode::enter(slave.as_raw_fd()).unwrap();
            bridge(
                &state,
                &id,
                &mut claim,
                || {
                    inputs
                        .pop_front()
                        .ok_or_else(|| io::ErrorKind::WouldBlock.into())
                },
                &mut io::sink(),
                deadline,
                None,
            )
            .await
        };
        assert_eq!(result, expected);
        assert_eq!(
            attributes(slave.as_raw_fd()).c_lflag & !libc::PENDIN,
            before.c_lflag & !libc::PENDIN
        );
    }
}

#[tokio::test]
async fn diagnostic_runtime_errors_cannot_take_success_artifact_cleanup_path() {
    let dir = tempfile::tempdir().unwrap();
    let (state, control) = fixture_state(dir.path(), "diagnostic-error");
    drop(control);
    let mut claim = claim_attachment(&state, "diagnostic-error").unwrap();
    state.sessions.lock().unwrap()["diagnostic-error"]
        .runtime
        .output_tx
        .send(RuntimeEvent::Error {
            code: "private-code".into(),
            message: "private-message".into(),
        })
        .unwrap();
    let mut output = Vec::new();
    let exit = bridge(
        &state,
        "diagnostic-error",
        &mut claim,
        || Err(io::ErrorKind::WouldBlock.into()),
        &mut output,
        Instant::now() + Duration::from_secs(1),
        None,
    )
    .await;
    assert!(
        !matches!(exit, Exit::Operator | Exit::Eof | Exit::RuntimeEnded),
        "runtime error must retain private failure artifacts"
    );
    assert!(!String::from_utf8(output).unwrap().contains("private"));
}

fn fixture_state(path: &Path, id: &str) -> (Arc<AppState>, mpsc::Receiver<RuntimeCommand>) {
    let mut state = crate::test_support::test_app_state_with_workspace(path);
    Arc::get_mut(&mut state)
        .expect("new fixture state")
        .peon
        .config
        .enabled = false;
    assert!(
        state.providers.get_applied().provider.is_none(),
        "fixture must not select an inference provider"
    );
    let (runtime, control) = SessionRuntime::live(24, 80);
    let (kill_tx, _) = tokio::sync::watch::channel(false);
    let mut info = crate::test_support::test_session_info(
        id,
        "Diagnostic",
        path.display().to_string(),
        "running",
        "now",
    );
    info.harness = Some("codex".into());
    state.sessions.lock().unwrap().insert(
        id.into(),
        crate::SessionHandle {
            info,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: crate::capacity_state::CapacityState::default(),
            active_work_hook: false,
        },
    );
    let now = crate::workspace_runtime::iso_now();
    let mut meta = crate::test_support::test_session_metadata(
        id,
        "Diagnostic",
        path.display().to_string(),
        "running",
        &now,
        &now,
    );
    meta.harness = "codex".into();
    meta.cwd = path.display().to_string();
    meta.lifecycle = "alive".into();
    meta.lifecycle_phase = "active".into();
    meta.connectivity = "online".into();
    meta.terminal_outcome = None;
    meta.final_observed_status_snapshot = None;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .write_session(&meta);
    state
        .sessions
        .lock()
        .unwrap()
        .get_mut(id)
        .unwrap()
        .runtime
        .native_diagnostic = Some(Arc::new(Mutex::new(Default::default())));
    (state, control)
}

fn install_reporter(root: &Path, workspace: &Path, state: &Arc<AppState>, binary: &Path) -> bool {
    use crate::harness::integration::{
        DetectedTool, IntegrationContext, IntegrationRegistration, ReporterAssetResolver,
    };
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let assets = ReporterAssetResolver {
        source_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts"),
        stable_dir: home.join(".orkworks/hook-scripts"),
    };
    let detected = DetectedTool {
        executable: binary.to_path_buf(),
        version: Some("0.160.0".into()),
        compatible: true,
    };
    let workspace_state = state.workspace.lock().unwrap();
    let Some(workspace_state) = workspace_state.as_ref() else {
        return false;
    };
    let ctx = IntegrationContext {
        workspace,
        workspace_metadata: Some(&workspace_state.metadata),
        orkworks_root: root,
        enabled: true,
        detected_tool: Some(&detected),
        reporter_assets: &assets,
    };
    let handler = crate::harness::integrations::handler(
        &crate::harness::definition::IntegrationBinding::Codex,
    );
    handler
        .install(&ctx)
        .is_ok_and(|status| status.registration == IntegrationRegistration::Installed)
        && assets
            .is_current("report-harness-event.sh")
            .unwrap_or(false)
}

#[test]
fn diagnostic_setup_installs_current_reporter_in_isolated_home() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let workspace = root.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    git2::Repository::init(&workspace).unwrap();
    std::fs::write(
        workspace.join(".gitignore"),
        ".codex/hooks.json\n.orkworks-test/\n",
    )
    .unwrap();
    let _home = crate::test_support::FakeHome::set(&home);
    let (state, _control) = fixture_state(&workspace, "fixture-setup");
    assert!(install_reporter(
        root.path(),
        &workspace,
        &state,
        Path::new("/fake/codex")
    ));
    assert!(home
        .join(".orkworks/hook-scripts/report-harness-event.sh")
        .exists());
    assert!(workspace.join(".codex/hooks.json").exists());
}

const LIVE_DEADLINE: Duration = Duration::from_secs(15 * 60);
const CLEANUP_DEADLINE: Duration = Duration::from_secs(10);

async fn start_fixture(
    state: &Arc<AppState>,
    id: &str,
    workspace: &Path,
    control: mpsc::Receiver<RuntimeCommand>,
    plan: super::super::codex_native::NativeLaunchPlan,
    environment: Vec<(String, String)>,
) -> bool {
    let (output, kill) = {
        let sessions = state.sessions.lock().unwrap();
        let handle = &sessions[id];
        (handle.runtime.output_tx.clone(), handle.kill_tx.subscribe())
    };
    start_session_runtime_inner(
        state.clone(),
        id.into(),
        crate::harness::CommandSpec {
            program: "/bin/false".into(),
            args: vec![],
            cwd: workspace.display().to_string(),
        },
        None,
        control,
        output,
        kill,
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        },
        None,
        Some((plan, environment)),
    )
    .await
    .is_ok()
}

async fn cleanup(state: &Arc<AppState>, id: &str, deadline: Instant) -> bool {
    let _ = send_runtime_command(state, id, RuntimeCommand::Kill).await;
    loop {
        let complete = {
            let sessions = state.sessions.lock().unwrap();
            sessions.get(id).is_some_and(|h| {
                h.runtime
                    .diagnostic_cleanup
                    .as_ref()
                    .is_some_and(|rx| *rx.borrow())
                    && h.info.lifecycle_phase == "ended"
            })
        };
        if complete && !super::super::terminal_runtime::has_workflow_report_capability(id) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn validate_auth_home(path: &Path, root: &Path, original_home: &Path) -> Option<PathBuf> {
    if !path.is_absolute() || !path.is_dir() {
        return None;
    }
    let canonical = path.canonicalize().ok()?;
    // Explicit canonical input rejects aliases/symlinks and protects cleanup ownership.
    if canonical != path
        || canonical.starts_with(root)
        || root.starts_with(&canonical)
        || canonical == original_home
        || canonical
            == original_home
                .join(".codex")
                .canonicalize()
                .unwrap_or_else(|_| original_home.join(".codex"))
    {
        return None;
    }
    Some(canonical)
}

#[test]
fn diagnostic_auth_home_must_be_dedicated_canonical_and_outside_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let auth = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let home = home.path().canonicalize().unwrap();
    let auth = auth.path().canonicalize().unwrap();
    std::fs::create_dir(home.join(".codex")).unwrap();
    assert!(validate_auth_home(&auth, &root, &home).is_some());
    for rejected in [&root, &home, &home.join(".codex"), &root.join("missing")] {
        assert!(validate_auth_home(rejected, &root, &home).is_none());
    }
    let alias = home.join("alias");
    std::os::unix::fs::symlink(&auth, &alias).unwrap();
    assert!(validate_auth_home(&alias, &root, &home).is_none());
}

#[tokio::test]
async fn diagnostic_fake_owned_runtime_disconnect_input_and_drain_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    let home = root.path().join("home");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::create_dir(&home).unwrap();
    git2::Repository::init(&workspace).unwrap();
    std::fs::write(
        workspace.join(".gitignore"),
        ".codex/hooks.json\n.orkworks-test/\n",
    )
    .unwrap();
    let _home = crate::test_support::FakeHome::set(&home);
    let id = format!("diagnostic-owned-{}", uuid::Uuid::new_v4());
    let (state, control) = fixture_state(&workspace, &id);
    assert!(install_reporter(
        root.path(),
        &workspace,
        &state,
        Path::new("/fake/codex")
    ));
    let (plan, mut environment) = super::super::codex_native::fixture_plan_and_env(
        &workspace,
        &state.integration_probe_cache,
        false,
    );
    environment.extend([
        ("HOME".into(), home.display().to_string()),
        ("SHELL".into(), "/bin/sh".into()),
    ]);
    assert!(start_fixture(&state, &id, &workspace, control, plan, environment).await);
    let sink = state.sessions.lock().unwrap()[&id]
        .runtime
        .native_diagnostic
        .clone()
        .unwrap();
    let mut claim = claim_attachment(&state, &id).unwrap();
    // Real authenticated passive startup Client is closed; retained server/TUI remain alive.
    state.sessions.lock().unwrap()[&id]
        .runtime
        .native_observer_pause
        .as_ref()
        .unwrap()
        .send_replace(true);
    let limit = Instant::now() + Duration::from_secs(2);
    while !sink.lock().unwrap().observer_paused && Instant::now() < limit {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let disconnected = sink.lock().unwrap().observer_disconnects;
    // Establish a fake held Codex wait; approved input must independently clear it.
    {
        let workspace = state.workspace.lock().unwrap();
        let store = &workspace.as_ref().unwrap().metadata;
        let mut meta = store.read_session(&id).unwrap();
        meta.attention = Some("needs_you".into());
        meta.observed_status = Some("waiting_for_input".into());
        meta.metadata_source = "codex_hook".into();
        meta.metadata_confidence = 1.0;
        store.write_session(&meta);
        let mut sessions = state.sessions.lock().unwrap();
        let handle = sessions.get_mut(&id).unwrap();
        handle.active_work_hook = true;
        super::super::observed_status::apply_live_attention_fields(
            &mut handle.info,
            &mut handle.runtime.attention_owner,
            "waiting_for_input",
            None,
            "codex_hook",
            1.0,
        );
    }
    let (_master, slave) = terminal_pair();
    let before = attributes(slave.as_raw_fd());
    {
        let _mode = TerminalMode::enter(slave.as_raw_fd()).unwrap();
        let mut input = VecDeque::from([Some(b"a".to_vec()), None]);
        assert_eq!(
            bridge(
                &state,
                &id,
                &mut claim,
                || input
                    .pop_front()
                    .ok_or_else(|| io::ErrorKind::WouldBlock.into()),
                &mut io::sink(),
                Instant::now() + Duration::from_secs(2),
                Some(slave.as_raw_fd())
            )
            .await,
            Exit::Eof
        );
    }
    assert_eq!(
        attributes(slave.as_raw_fd()).c_lflag & !libc::PENDIN,
        before.c_lflag & !libc::PENDIN
    );
    assert_eq!(sink.lock().unwrap().accepted_input, 1);
    assert!(super::super::terminal_runtime::has_workflow_report_capability(&id));
    // Zero cleanup budget fails honestly, preserving artifacts/HOME while owned work drains.
    assert!(!cleanup(&state, &id, Instant::now()).await);
    assert!(workspace.exists());
    assert_eq!(dirs::home_dir().unwrap(), home);
    assert!(cleanup(&state, &id, Instant::now() + CLEANUP_DEADLINE).await);
    assert!(*state.sessions.lock().unwrap()[&id]
        .runtime
        .diagnostic_cleanup
        .as_ref()
        .unwrap()
        .borrow());
    assert!(!super::super::terminal_runtime::has_workflow_report_capability(&id));
    assert_eq!(disconnected, 1);
    assert_eq!(sink.lock().unwrap().input_to_working, 1);
    assert_eq!(
        sink.lock().unwrap().effects[1][0],
        0,
        "input transition is not native clear"
    );
    release_attachment(&state, &id, claim.generation);
}

/// Operator-only. Compile normally, then run only this exact test in a clean
/// environment with --exact --ignored --nocapture --test-threads=1. Never tee.
#[tokio::test]
#[ignore = "operator-run authenticated installed Codex diagnostic; may use model"]
async fn installed_codex_native_approval_diagnostic() {
    let deadline = Instant::now() + LIVE_DEADLINE;
    let binary = std::env::var_os("ORK690_DIAGNOSTIC_BINARY")
        .map(PathBuf::from)
        .expect("set ORK690_DIAGNOSTIC_BINARY to canonical native codex");
    let auth = std::env::var_os("ORK690_DIAGNOSTIC_CODEX_HOME")
        .map(PathBuf::from)
        .expect("set ORK690_DIAGNOSTIC_CODEX_HOME to dedicated authenticated home");
    let original_home = dirs::home_dir().expect("HOME required");
    // Keep immediately: every failure retains private artifacts; success deletes
    // only after acknowledged owned teardown. Auth home never belongs to fixture.
    let root = tempfile::Builder::new()
        .prefix("ork690-diagnostic-")
        .tempdir()
        .expect("fixture directory")
        .keep()
        .canonicalize()
        .expect("canonical fixture");
    let auth = validate_auth_home(&auth, &root, &original_home)
        .expect("dedicated canonical CODEX_HOME required; normal home/aliases rejected");
    let workspace = root.join("workspace");
    let home = root.join("home");
    std::fs::create_dir(&workspace).expect("fixture workspace");
    std::fs::create_dir(&home).expect("fixture HOME");
    git2::Repository::init(&workspace).expect("fixture Git workspace");
    std::fs::write(
        workspace.join(".gitignore"),
        ".codex/hooks.json\n.orkworks-test/\n",
    )
    .expect("fixture ignore");
    eprintln!("Diagnostic fixture: {}. Private artifacts are never evidence; failure retains this directory. Auth home is operator-owned.",root.display());
    eprintln!("15-minute deadline. Ctrl-] exits; Ctrl-\\ toggles passive observer pause. Production native clear remains DISABLED. Trust workspace and all six /hooks, then /new; wait for root_bound=true and hook_active=true. Input-driven working is separate evidence.");
    let fake_home = crate::test_support::FakeHome::set(&home);
    let id = format!("diagnostic-{}", uuid::Uuid::new_v4());
    let (state, control) = fixture_state(&workspace, &id);
    assert!(
        install_reporter(&root, &workspace, &state, &binary),
        "reporter setup failed; private fixture retained"
    );
    let command = crate::harness::CommandSpec {
        program: binary.display().to_string(),
        args: vec![],
        cwd: workspace.display().to_string(),
    };
    let plan = match super::super::codex_native::installed_diagnostic_plan(
        &command,
        &state.integration_probe_cache,
    )
    .await
    {
        Ok(plan) => plan,
        Err(_) => {
            panic!("installed binary/version/platform/probe gate failed; private fixture retained")
        }
    };
    let environment = vec![
        ("HOME".into(), home.display().to_string()),
        ("CODEX_HOME".into(), auth.display().to_string()),
        ("SHELL".into(), "/bin/sh".into()),
        ("BASH_ENV".into(), "/dev/null".into()),
        ("ENV".into(), "/dev/null".into()),
        ("ZDOTDIR".into(), home.display().to_string()),
    ];
    // Validate operator TTY before spawning; the guard restores on every return.
    // A new open-file description keeps nonblocking input flags independent
    // of shell-duplicated stdin/stdout descriptors.
    let input = std::fs::File::open("/dev/tty").expect("interactive controlling TTY required");
    let terminal = TerminalMode::enter(input.as_raw_fd()).expect("interactive TTY required");
    let started = start_fixture(&state, &id, &workspace, control, plan, environment).await;
    if !started {
        drop(terminal);
        panic!("owned startup failed; private fixture retained");
    }
    let mut claim = match claim_attachment(&state, &id) {
        Some(claim) => claim,
        None => {
            let clean = cleanup(&state, &id, Instant::now() + CLEANUP_DEADLINE).await;
            drop(terminal);
            if !clean {
                std::mem::forget(fake_home);
            }
            panic!("diagnostic attachment failed; private fixture retained");
        }
    };
    let exit = bridge(
        &state,
        &id,
        &mut claim,
        || terminal.read(),
        &mut io::stdout(),
        deadline,
        Some(input.as_raw_fd()),
    )
    .await;
    let clean = cleanup(&state, &id, Instant::now() + CLEANUP_DEADLINE).await;
    release_attachment(&state, &id, claim.generation);
    let restored = terminal.restore();
    drop(terminal);
    eprintln!("Diagnostic exit={exit:?} teardown_acknowledged={clean} tty_restored={restored}; installed gates still require operator evidence.");
    if !clean {
        // Keep process-global HOME valid through runtime/task destruction in this
        // exact-test subprocess. Do not delete potentially active artifacts.
        std::mem::forget(fake_home);
        panic!("owned cleanup deadline failed; fixture HOME and private artifacts retained until process exit");
    }
    drop(fake_home);
    if restored && matches!(exit, Exit::Operator | Exit::Eof | Exit::RuntimeEnded) {
        std::fs::remove_dir_all(&root).expect("delete only acknowledged fixture");
    } else {
        panic!("diagnostic failed; private fixture retained; auth home unchanged");
    }
}
