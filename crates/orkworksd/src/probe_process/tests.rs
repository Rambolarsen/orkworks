use super::*;

#[cfg(target_os = "macos")]
pub(super) type BeforeGroupKill = Box<dyn FnOnce(&Owner) + Send>;

#[derive(Default)]
pub(super) struct Faults {
    pub(super) pipe_setup: AtomicBool,
    pub(super) close_after_register: AtomicBool,
    pub(super) cleanup: AtomicBool,
    pub(super) reap: AtomicBool,
    #[cfg(target_os = "macos")]
    pub(super) before_group_kill: Mutex<Option<BeforeGroupKill>>,
    #[cfg(target_os = "macos")]
    pub(super) pending_exit: AtomicBool,
    #[cfg(target_os = "macos")]
    pub(super) pending_checks: std::sync::atomic::AtomicUsize,
    #[cfg(target_os = "macos")]
    pub(super) group_signals: std::sync::atomic::AtomicUsize,
    #[cfg(target_os = "macos")]
    pub(super) membership_result: Mutex<Option<(i32, [i32; 2])>>,
}

#[tokio::test]
async fn saturation_and_shutdown_never_spawn_a_new_probe() {
    let service = ProbeService::new().unwrap();
    let mut permits = Vec::new();
    for _ in 0..64 {
        permits.push(service.shared.permits.clone().try_acquire_owned().unwrap());
    }
    let mut command = Command::new("must-not-be-spawned");
    let policy = ProbePolicy {
        max_bytes: 64,
        reject_overflow: true,
        require_success: true,
    };
    assert_eq!(
        service
            .run(&mut command, policy, Duration::from_secs(3))
            .await
            .unwrap_err(),
        ProbeError::Busy
    );
    drop(permits);
    service.shutdown().await;
    assert_eq!(
        service
            .run(&mut command, policy, Duration::from_secs(3))
            .await
            .unwrap_err(),
        ProbeError::Closed
    );
}

#[cfg(unix)]
mod unix {
    use super::*;
    fn shell(script: &str, dir: &std::path::Path) -> Command {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(script).current_dir(dir);
        cmd
    }
    fn policy(success: bool, overflow: bool) -> ProbePolicy {
        ProbePolicy {
            max_bytes: 1024,
            require_success: success,
            reject_overflow: overflow,
        }
    }
    async fn quiet(path: &std::path::Path) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let before = std::fs::metadata(path).unwrap().len();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            std::fs::metadata(path).unwrap().len(),
            before,
            "owned descendant survived cleanup"
        );
    }
    struct Cleanup(i32);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            unsafe {
                libc::kill(self.0, libc::SIGKILL);
            }
        }
    }
    fn fixture_child(dir: &std::path::Path) -> Cleanup {
        Cleanup(
            std::fs::read_to_string(dir.join("pid"))
                .unwrap()
                .trim()
                .parse()
                .unwrap(),
        )
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn cleanup_handles_leader_exit_between_status_and_group_signal() {
        for (script, only_leader) in [
            ("exec 2>/dev/null; printf v; while [ ! -f release ]; do :; done", true),
            ("sleep 30 >/dev/null 2>&1 & exec 2>/dev/null; printf v; while [ ! -f release ]; do :; done", false),
        ] {
            let service = ProbeService::new().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let release = dir.path().join("release");
            let observed = Arc::new(Mutex::new(None));
            let observation = observed.clone();
            *service.shared.faults.before_group_kill.lock().unwrap() =
                Some(Box::new(move |owner| {
                    let was_running = matches!(owner.peek_exit(), Ok(None));
                    let live_rejected = !only_exited_leader(owner);
                    if std::fs::write(release, b"exit").is_err() {
                        return;
                    }
                    let deadline = Instant::now() + Duration::from_secs(1);
                    while Instant::now() < deadline {
                        if matches!(owner.peek_exit(), Ok(Some(_))) {
                            *observation.lock().unwrap() = Some((
                                was_running,
                                live_rejected,
                                only_exited_leader(owner),
                            ));
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }));
            // Both streams finish before the leader exits: one stdout byte is
            // enough for version capture, and the shell closes its stderr pipe.
            let result = service
                .run(
                    &mut shell(script, dir.path()),
                    ProbePolicy {
                        max_bytes: 1,
                        reject_overflow: false,
                        require_success: false,
                    },
                    Duration::from_secs(3),
                )
                .await;
            service.shutdown().await;
            assert_eq!(
                *observed.lock().unwrap(),
                Some((true, true, only_leader)),
                "fixture must observe the same unreaped leader exit and distinguish a remaining member"
            );
            assert_eq!(result.unwrap().stdout, b"v");
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
        }
    }

    #[cfg(target_os = "macos")]
    fn hold_post_signal_exit(service: &ProbeService, dir: &std::path::Path) {
        let release = dir.join("release");
        let injected = service.shared.faults.clone();
        *service.shared.faults.before_group_kill.lock().unwrap() = Some(Box::new(move |owner| {
            std::fs::write(release, b"exit").unwrap();
            let until = Instant::now() + Duration::from_secs(1);
            while owner.peek_exit().unwrap().is_none() {
                assert!(Instant::now() < until, "fixture leader did not exit");
                std::thread::sleep(Duration::from_millis(1));
            }
            injected.pending_exit.store(true, Ordering::Relaxed);
        }));
    }

    #[cfg(target_os = "macos")]
    async fn wait_for_pending(service: &ProbeService) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while service.shared.faults.pending_checks.load(Ordering::Relaxed) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn cleanup_confirms_exit_after_first_post_signal_check_is_pending() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        hold_post_signal_exit(&service, dir.path());
        let faults = service.shared.faults.clone();
        let mut command = shell(
            "exec 2>/dev/null; printf v; while [ ! -f release ]; do :; done",
            dir.path(),
        );
        let (result, retained) = tokio::join!(
            service.run(
                &mut command,
                ProbePolicy {
                    max_bytes: 1,
                    reject_overflow: false,
                    require_success: false
                },
                Duration::from_secs(3)
            ),
            async {
                wait_for_pending(&service).await;
                service.shared.sweep();
                let retained = service
                    .shared
                    .owners
                    .lock()
                    .unwrap()
                    .first()
                    .is_some_and(|owner| !owner.lock().unwrap().reaped);
                let charged = service.shared.permits.available_permits() == OWNER_LIMIT - 1;
                faults.pending_exit.store(false, Ordering::Relaxed);
                retained && charged
            }
        );
        service.shutdown().await;
        assert_eq!(result.unwrap().stdout, b"v");
        assert!(
            retained,
            "pending confirmation lost original identity or capacity"
        );
        assert_eq!(faults.group_signals.load(Ordering::Relaxed), 1);
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn pending_confirmation_drop_and_shutdown_never_signal_or_accept_late() {
        for shutdown in [false, true] {
            let service = Arc::new(ProbeService::new().unwrap());
            let dir = tempfile::tempdir().unwrap();
            hold_post_signal_exit(&service, dir.path());
            let running = service.clone();
            let path = dir.path().to_owned();
            let task = tokio::spawn(async move {
                running
                    .run(
                        &mut shell(
                            "exec 2>/dev/null; printf v; while [ ! -f release ]; do :; done",
                            &path,
                        ),
                        ProbePolicy {
                            max_bytes: 1,
                            reject_overflow: false,
                            require_success: false,
                        },
                        Duration::from_secs(3),
                    )
                    .await
            });
            wait_for_pending(&service).await;
            service.shared.sweep();
            {
                let owners = service.shared.owners.lock().unwrap();
                let owner = owners.first().unwrap().lock().unwrap();
                assert_eq!(owner.cleanup, Some(CleanupState::PendingExit));
                assert!(!owner.reaped);
            }
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT - 1);
            if shutdown {
                service.shutdown().await;
                assert_eq!(task.await.unwrap().unwrap_err(), ProbeError::Closed);
            } else {
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
                service.shutdown().await;
            }
            assert_eq!(
                service.shared.faults.group_signals.load(Ordering::Relaxed),
                1
            );
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn permanently_pending_exit_exhausts_cleanup_without_new_window() {
        // The first case reaches cleanup's 1s bound; the second reaches the
        // earlier operation deadline while exit confirmation is still pending.
        for budget in [Duration::from_secs(3), Duration::from_millis(500)] {
            let service = ProbeService::new().unwrap();
            let dir = tempfile::tempdir().unwrap();
            hold_post_signal_exit(&service, dir.path());
            let result = service
                .run(
                    &mut shell(
                        "exec 2>/dev/null; printf v; while [ ! -f release ]; do :; done",
                        dir.path(),
                    ),
                    ProbePolicy {
                        max_bytes: 1,
                        reject_overflow: false,
                        require_success: false,
                    },
                    budget,
                )
                .await;
            // Preserve the existing cleanup-settlement failure mapping even
            // when its endpoint is the operation deadline.
            assert_eq!(result.unwrap_err(), ProbeError::CleanupUnconfirmed);
            assert!(service.shared.faults.pending_checks.load(Ordering::Relaxed) > 1);
            service
                .shared
                .faults
                .pending_exit
                .store(false, Ordering::Relaxed);
            service.shutdown().await;
            assert_eq!(
                service.shared.faults.group_signals.load(Ordering::Relaxed),
                1
            );
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn pending_confirmation_rejects_live_members_and_lost_identity() {
        use std::os::unix::process::CommandExt;
        for lost_identity in [false, true] {
            let service = ProbeService::new().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let mut command = shell(
                "sleep 30 >/dev/null 2>&1 & echo $! > pid; exit 0",
                dir.path(),
            );
            command.process_group(0);
            let child = command.spawn().unwrap();
            let mut owner = Owner {
                child,
                cleanup: Some(CleanupState::PendingExit),
                reaped: false,
                _permit: service.shared.permits.clone().try_acquire_owned().unwrap(),
                faults: service.shared.faults.clone(),
            };
            tokio::time::timeout(Duration::from_secs(3), async {
                while owner.peek_exit().unwrap().is_none() {
                    tokio::time::sleep(POLL).await;
                }
            })
            .await
            .unwrap();
            let _cleanup = fixture_child(dir.path());
            if lost_identity {
                owner.child.wait().unwrap();
            }
            owner.confirm_cleanup();
            assert_eq!(owner.cleanup, Some(CleanupState::Rejected));
            owner.cancel_cleanup();
            assert_eq!(
                service.shared.faults.group_signals.load(Ordering::Relaxed),
                0
            );
            owner.child.wait().unwrap();
            drop(owner);
            service.shutdown().await;
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn pending_confirmation_rejects_failed_or_malformed_membership_queries() {
        use std::os::unix::process::CommandExt;
        for bytes in [-1, 3, 12] {
            let service = ProbeService::new().unwrap();
            let dir = tempfile::tempdir().unwrap();
            let mut command = shell("exit 0", dir.path());
            command.process_group(0);
            let child = command.spawn().unwrap();
            let mut owner = Owner {
                child,
                cleanup: Some(CleanupState::PendingExit),
                reaped: false,
                _permit: service.shared.permits.clone().try_acquire_owned().unwrap(),
                faults: service.shared.faults.clone(),
            };
            tokio::time::timeout(Duration::from_secs(3), async {
                while owner.peek_exit().unwrap().is_none() {
                    tokio::time::sleep(POLL).await;
                }
            })
            .await
            .unwrap();
            *service.shared.faults.membership_result.lock().unwrap() =
                Some((bytes, [owner.child.id() as i32, 0]));
            owner.confirm_cleanup();
            assert_eq!(
                owner.cleanup,
                Some(CleanupState::Rejected),
                "membership bytes={bytes}"
            );
            owner.cancel_cleanup();
            assert_eq!(
                service.shared.faults.group_signals.load(Ordering::Relaxed),
                0
            );
            owner.child.wait().unwrap();
            drop(owner);
            service.shutdown().await;
            assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
        }
    }

    #[tokio::test]
    async fn successful_help_cleans_descendants_without_touching_sentinel() {
        let dir = tempfile::tempdir().unwrap();
        let service = ProbeService::new().unwrap();
        let mut sentinel = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let result = service.run(&mut shell("(while :; do printf x >> heartbeat; sleep 0.01; done) >/dev/null 2>&1 & echo $! > pid; while [ ! -f heartbeat ]; do sleep 0.01; done; printf 'version'; exit 0",dir.path()),policy(true,true),Duration::from_secs(3)).await.unwrap();
        let _cleanup = fixture_child(dir.path());
        assert_eq!(result.stdout, b"version");
        quiet(&dir.path().join("heartbeat")).await;
        let alive = sentinel.try_wait().unwrap().is_none();
        sentinel.kill().unwrap();
        sentinel.wait().unwrap();
        assert!(alive, "unrelated sentinel was killed");
        service.shutdown().await;
    }
    #[tokio::test]
    async fn prefix_capture_preserves_version_policy_and_rejects_help_overflow() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let result=service.run(&mut shell("while :; do printf abcdefghijklmnopqrstuvwxyz; printf abcdefghijklmnopqrstuvwxyz >&2; done",dir.path()),policy(false,false),Duration::from_secs(3)).await.unwrap();
        assert_eq!(result.stdout.len(), 1024);
        assert_eq!(service.run(&mut shell("while :; do printf abcdefghijklmnopqrstuvwxyz; printf abcdefghijklmnopqrstuvwxyz >&2; done",dir.path()),policy(true,true),Duration::from_secs(3)).await.unwrap_err(),ProbeError::Overflow);
        let silent = service
            .run(
                &mut shell("exit 9", dir.path()),
                policy(false, false),
                Duration::from_secs(3),
            )
            .await
            .unwrap();
        assert!(silent.stdout.is_empty());
        assert_eq!(
            service
                .run(
                    &mut shell("exit 9", dir.path()),
                    policy(true, true),
                    Duration::from_secs(3)
                )
                .await
                .unwrap_err(),
            ProbeError::Unsuccessful
        );
    }
    #[tokio::test]
    async fn dropping_probe_future_cleans_without_another_poll() {
        let service = Arc::new(ProbeService::new().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let running = service.clone();
        let path = dir.path().to_owned();
        let task = tokio::spawn(async move {
            running.run(&mut shell("(while :; do printf x >> heartbeat; sleep 0.01; done) & echo $! > pid; wait",&path),policy(true,true),Duration::from_secs(30)).await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while !dir.path().join("heartbeat").exists() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        let _cleanup = fixture_child(dir.path());
        task.abort();
        let _ = task.await;
        quiet(&dir.path().join("heartbeat")).await;
        service.shutdown().await;
        assert_eq!(service.shared.permits.available_permits(), 64);
    }
    #[tokio::test]
    async fn shutdown_terminates_active_probe_and_closes_admission() {
        let service = Arc::new(ProbeService::new().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let running = service.clone();
        let path = dir.path().to_owned();
        let task = tokio::spawn(async move {
            running
                .run(
                    &mut shell(
                        "echo $$ > pid; while :; do printf x >> heartbeat; sleep 0.01; done",
                        &path,
                    ),
                    policy(true, true),
                    Duration::from_secs(30),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while !dir.path().join("heartbeat").exists() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        let _cleanup = fixture_child(dir.path());
        service.shutdown().await;
        assert_eq!(task.await.unwrap().unwrap_err(), ProbeError::Closed);
        quiet(&dir.path().join("heartbeat")).await;
        assert_eq!(service.shared.permits.available_permits(), 64);
    }
    #[tokio::test]
    async fn setup_failure_terminates_before_delayed_configured_output() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        service
            .shared
            .faults
            .pipe_setup
            .store(true, Ordering::Relaxed);
        assert_eq!(
            service
                .run(
                    &mut shell("sleep 0.2; touch unexpected", dir.path()),
                    policy(true, true),
                    Duration::from_secs(3)
                )
                .await
                .unwrap_err(),
            ProbeError::Io
        );
        service.shutdown().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!dir.path().join("unexpected").exists());
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
    }

    #[tokio::test]
    async fn unconfirmed_cleanup_retains_capacity_until_root_is_reaped() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        service.shared.faults.cleanup.store(true, Ordering::Relaxed);
        service.shared.faults.reap.store(true, Ordering::Relaxed);
        assert_eq!(
            service
                .run(
                    &mut shell("printf version; sleep 30", dir.path()),
                    policy(true, true),
                    Duration::from_millis(100)
                )
                .await
                .unwrap_err(),
            ProbeError::CleanupUnconfirmed
        );
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT - 1);
        service.shared.faults.reap.store(false, Ordering::Relaxed);
        service.shutdown().await;
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
    }

    #[tokio::test]
    async fn cleanup_settle_uses_remaining_deadline() {
        use std::os::unix::process::CommandExt;

        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        service.shared.faults.reap.store(true, Ordering::Relaxed);
        let permit = service.shared.permits.clone().try_acquire_owned().unwrap();
        let child = shell("printf version", dir.path())
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let owner = Arc::new(Mutex::new(Owner {
            child,
            cleanup: None,
            reaped: false,
            _permit: permit,
            faults: service.shared.faults.clone(),
        }));
        let lease = Lease(owner.clone());
        assert!(service.shared.register(owner.clone()));

        // Native startup is outside this settlement-only budget. Observe exit
        // without reaping so cleanup retains the original process-group owner.
        let ready_by = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(success) = owner.lock().unwrap().peek_exit().unwrap() {
                assert!(success);
                break;
            }
            assert!(std::time::Instant::now() < ready_by, "child did not exit");
            tokio::time::sleep(POLL).await;
        }
        tokio::time::pause();
        let start = tokio::time::Instant::now();
        let result = service
            .settle(&owner, start + Duration::from_millis(100), None)
            .await;
        assert_eq!(owner.lock().unwrap().cleanup, Some(CleanupState::Accepted));
        drop(lease);
        drop(owner);
        assert_eq!(result.unwrap_err(), ProbeError::CleanupUnconfirmed);
        assert!(start.elapsed() < Duration::from_millis(800));
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT - 1);

        // The background native reaper uses real time during teardown.
        tokio::time::resume();
        service.shared.faults.reap.store(false, Ordering::Relaxed);
        service.shutdown().await;
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
    }

    #[tokio::test]
    async fn completed_owner_keeps_permit_until_registry_removal() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        // An owner remains capacity-accounted after reap until its final drop.
        let mut child = shell("exit 0", dir.path()).spawn().unwrap();
        child.wait().unwrap();
        let mut owner = Owner {
            child,
            cleanup: Some(CleanupState::Accepted),
            reaped: false,
            _permit: service.shared.permits.clone().try_acquire_owned().unwrap(),
            faults: service.shared.faults.clone(),
        };
        owner.reap();
        assert!(owner.reaped);
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT - 1);
        drop(owner);
        assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
    }

    #[tokio::test]
    async fn externally_reaped_root_loses_group_cleanup_authority() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut child = shell("exit 0", dir.path()).spawn().unwrap();
        child.wait().unwrap();
        let mut owner = Owner {
            child,
            cleanup: None,
            reaped: false,
            _permit: service.shared.permits.clone().try_acquire_owned().unwrap(),
            faults: service.shared.faults.clone(),
        };
        assert!(!owner.terminate());
        assert_eq!(owner.cleanup, Some(CleanupState::Rejected));
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn exited_leader_check_rejects_live_group_member() {
        use std::os::unix::process::CommandExt;
        let dir = tempfile::tempdir().unwrap();
        let mut cmd = shell("sleep 30 >/dev/null 2>&1 & exit 0", dir.path());
        cmd.process_group(0);
        let child = cmd.spawn().unwrap();
        let service = ProbeService::new().unwrap();
        let mut owner = Owner {
            child,
            cleanup: None,
            reaped: false,
            _permit: service.shared.permits.clone().try_acquire_owned().unwrap(),
            faults: service.shared.faults.clone(),
        };
        tokio::time::timeout(Duration::from_secs(3), async {
            while owner.peek_exit().unwrap().is_none() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        assert!(!only_exited_leader(&owner));
        assert!(owner.terminate());
        owner.child.wait().unwrap();
    }
    #[tokio::test]
    async fn escaped_session_is_outside_owned_group() {
        let service = ProbeService::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new("python3");
        command.current_dir(dir.path()).arg("-c").arg(
            r#"
import os, time
pid = os.fork()
if pid == 0:
    os.setsid()
    fd = os.open(os.devnull, os.O_RDWR)
    os.dup2(fd, 1)
    os.dup2(fd, 2)
    with open('pid', 'w') as f: f.write(str(os.getpid()))
    while True:
        with open('heartbeat', 'a') as f: f.write('x')
        time.sleep(0.01)
while not os.path.exists('heartbeat'): time.sleep(0.01)
print('version', flush=True)
"#,
        );
        let result = service
            .run(&mut command, policy(true, true), Duration::from_secs(3))
            .await
            .unwrap();
        let _cleanup = fixture_child(dir.path());
        assert_eq!(result.stdout, b"version\n");
        let before = std::fs::metadata(dir.path().join("heartbeat"))
            .unwrap()
            .len();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            std::fs::metadata(dir.path().join("heartbeat"))
                .unwrap()
                .len()
                > before,
            "fixture must demonstrate the documented setsid escape limit"
        );
        service.shutdown().await;
    }
}

#[tokio::test]
async fn shutdown_between_registration_and_attachment_never_panics() {
    let service = ProbeService::new().unwrap();
    service
        .shared
        .faults
        .close_after_register
        .store(true, Ordering::Relaxed);
    #[cfg(unix)]
    let mut command = {
        let mut c = Command::new("/bin/sh");
        c.args(["-c", "sleep 30"]);
        c
    };
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("powershell.exe");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ]);
        c
    };
    let error = service
        .run(
            &mut command,
            ProbePolicy {
                max_bytes: 64,
                reject_overflow: true,
                require_success: true,
            },
            Duration::from_secs(3),
        )
        .await
        .unwrap_err();
    assert_eq!(error, ProbeError::Closed);
    service.shutdown().await;
    assert_eq!(service.shared.permits.available_permits(), OWNER_LIMIT);
}

#[test]
fn shutdown_before_first_probe_closes_global_admission() {
    const FIXTURE: &str = "ORKWORKS_PROBE_SHUTDOWN_FIXTURE";
    if std::env::var_os(FIXTURE).is_some() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            super::shutdown().await;
            let mut command = Command::new("must-not-be-spawned");
            assert_eq!(
                super::run(
                    &mut command,
                    ProbePolicy {
                        max_bytes: 64,
                        reject_overflow: true,
                        require_success: true
                    },
                    Duration::from_secs(3)
                )
                .await
                .unwrap_err(),
                ProbeError::Closed
            );
        });
        return;
    }
    // Global admission is intentionally irreversible. Isolate it from other tests.
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "probe_process::tests::shutdown_before_first_probe_closes_global_admission",
            "--nocapture",
        ])
        .env(FIXTURE, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
