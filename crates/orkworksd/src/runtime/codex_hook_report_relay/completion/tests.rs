use super::*;
#[test]
fn completion_queue_initialization_is_private_and_durable() {
    let relay = CodexHookReportRelay::new().unwrap();
    let queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    assert_ne!(queue.path(), relay.mailbox_path());
    assert_eq!(
        std::fs::metadata(queue.path()).unwrap().mode() & 0o077,
        0,
        "stream directory must be private"
    );
    queue.identity().expect("fresh stream identity");
    queue.unfinished().expect("fresh stream file layout");
    assert!(
        queue.path().join("queue.lock").is_file(),
        "retained lock must exist before publication"
    );
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(queue.path().join("queue.json")).unwrap()).unwrap();
    assert_eq!(state["version"], 1);
    assert_eq!(state["generation"], 7);
    assert_eq!(state["acknowledgedSequence"], 0);
    assert_eq!(state["nextSequence"], 1);
    assert_eq!(state["records"], serde_json::json!([]));
}
fn fixture_record(seq: u64) -> serde_json::Value {
    serde_json::json!({"version":1,"sequence":seq,"observedMicros":seq,"hookFingerprint":"a".repeat(64),"rootId":"root","turnId":"turn","event":"PreToolUse","toolName":"Bash","toolUseId":format!("tool-{seq}")})
}
fn publish_fixture(queue: &CompletionQueue, records: Vec<serde_json::Value>) {
    let path = queue.path().join("queue.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state["nextSequence"] = serde_json::json!(
        state["acknowledgedSequence"].as_u64().unwrap() + records.len() as u64 + 1
    );
    state["records"] = serde_json::json!(records);
    std::fs::write(path, serde_json::to_vec(&state).unwrap()).unwrap();
}
#[test]
fn completion_queue_applies_and_acknowledges_sender_sequence_once() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_fixture(&queue, vec![fixture_record(1), fixture_record(2)]);
    let mut seen = vec![];
    assert_eq!(
        queue
            .consume("root", |event| {
                seen.push(event.sequence);
                Ok(())
            })
            .unwrap(),
        Poll::Consumed(2)
    );
    assert_eq!(seen, [1, 2]);
    assert_eq!(
        queue
            .consume("root", |_| panic!("acknowledged records must not replay"))
            .unwrap(),
        Poll::Consumed(0)
    );
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(queue.path().join("queue.json")).unwrap()).unwrap();
    assert_eq!(state["acknowledgedSequence"], 2);
    assert_eq!(state["nextSequence"], 3);
    assert_eq!(state["records"], serde_json::json!([]));
}
#[test]
fn completion_queue_inflight_never_grants_authority() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_fixture(&queue, vec![fixture_record(1)]);
    let marker = queue.path().join(format!("inflight-{}", "a".repeat(32)));
    std::fs::write(&marker, b"1").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        queue
            .consume("root", |_| panic!("unfinished writer cannot clear"))
            .unwrap(),
        Poll::Busy
    );
    assert!(marker.exists());
}

#[test]
fn completion_queue_corruption_never_invokes_application_or_recovers() {
    for case in 0..14 {
        let relay = CodexHookReportRelay::new().unwrap();
        let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
        publish_fixture(&queue, vec![fixture_record(1), fixture_record(2)]);
        let path = queue.path().join("queue.json");
        let mut state: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match case {
            0 => state["records"][0]["sequence"] = serde_json::json!(2),
            1 => state["records"][1]["sequence"] = serde_json::json!(1),
            2 => state["generation"] = serde_json::json!(8),
            3 => state["streamId"] = serde_json::json!("b".repeat(32)),
            4 => state["records"][0]["rootId"] = serde_json::json!("child"),
            5 => state["records"][0]["hookFingerprint"] = serde_json::json!("b".repeat(64)),
            6 => state["version"] = serde_json::json!(2),
            7 => state["records"][0]["command"] = serde_json::json!("never allowed"),
            8 => state["records"][0]["observedMicros"] = serde_json::json!(0),
            9 => state["records"][0]["toolName"] = serde_json::json!("bad\nname"),
            10 => state["records"][0]
                .as_object_mut()
                .unwrap()
                .remove("turnId")
                .map(|_| ())
                .unwrap(),
            11 => state["nextSequence"] = serde_json::json!(true),
            12 => state["acknowledgedSequence"] = serde_json::json!(1),
            _ => state["records"].as_array_mut().unwrap().reverse(),
        }
        std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
        assert!(
            queue
                .consume("root", |_| panic!("case {case}: invalid stream applied"))
                .is_err(),
            "case {case}"
        );
        assert!(queue.path().join("poison").exists());
        assert!(queue
            .consume("root", |_| panic!("failed assistance recovered"))
            .is_err());
    }
}

#[test]
fn completion_queue_duplicate_keys_oversize_and_pending_replacement_fail_closed() {
    for case in 0..3 {
        let relay = CodexHookReportRelay::new().unwrap();
        let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
        let path = queue.path().join("queue.json");
        match case {
            0 => {
                let text = std::fs::read_to_string(&path).unwrap();
                std::fs::write(&path, text.replacen('{', "{\"version\":1,", 1)).unwrap();
            }
            1 => std::fs::write(&path, vec![b' '; MAX_BYTES + 1]).unwrap(),
            _ => {
                let pending = queue.path().join(format!("pending-{}", "a".repeat(32)));
                std::fs::write(&pending, b"incomplete").unwrap();
                std::fs::set_permissions(&pending, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
        assert!(queue
            .consume("root", |_| panic!("invalid storage applied"))
            .is_err());
    }
}

#[test]
fn completion_queue_rejects_replaced_lock_symlink_and_oversized_pending_window() {
    for case in 0..3 {
        let relay = CodexHookReportRelay::new().unwrap();
        let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
        publish_fixture(&queue, vec![fixture_record(1)]);
        match case {
            0 => {
                let path = queue.path().join("queue.lock");
                std::fs::rename(&path, queue.path().join("retired-lock")).unwrap();
                let _ = CompletionQueue::create_file(&queue.mailbox, "queue.lock").unwrap();
            }
            1 => {
                let path = queue.path().join("queue.json");
                std::fs::rename(&path, queue.path().join("retired-state")).unwrap();
                std::os::unix::fs::symlink("retired-state", &path).unwrap();
            }
            _ => publish_fixture(&queue, (1..=257).map(fixture_record).collect()),
        }
        assert!(queue
            .consume("root", |_| panic!("unsafe storage applied"))
            .is_err());
    }
}

#[test]
fn completion_queue_ack_failure_cannot_replay_after_filesystem_recovers() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_fixture(&queue, vec![fixture_record(1)]);
    let directory = queue.path().to_path_buf();
    let before = std::fs::read(directory.join("queue.json")).unwrap();
    let mut applied = 0;
    let result = queue.consume("root", |_| {
        applied += 1;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o500))
    });
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(applied, 1);
    assert_eq!(std::fs::read(directory.join("queue.json")).unwrap(), before);
    assert!(queue
        .consume("root", |_| panic!("unacknowledged effect replayed"))
        .is_err());
}

#[test]
fn completion_queue_callback_failure_and_caught_panic_are_terminal() {
    for panic in [false, true] {
        let relay = CodexHookReportRelay::new().unwrap();
        let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
        publish_fixture(&queue, vec![fixture_record(1)]);
        if panic {
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || queue.consume("root", |_| panic!("application interrupted"))
            ))
            .is_err());
        } else {
            assert!(queue
                .consume("root", |_| Err(io::Error::other("application failure")))
                .is_err());
        }
        assert!(queue
            .consume("root", |_| panic!("application effect replayed"))
            .is_err());
        let lock = File::open(queue.path().join("queue.lock")).unwrap();
        fs2::FileExt::try_lock_exclusive(&lock)
            .expect("callback failure must release retained lock");
    }
}

#[test]
fn completion_queue_acknowledgement_drains_without_a_lifetime_budget() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    let mut seen = vec![];
    for batch in 0..3 {
        publish_fixture(
            &queue,
            (1..=200).map(|i| fixture_record(batch * 200 + i)).collect(),
        );
        assert_eq!(
            queue
                .consume("root", |e| {
                    seen.push(e.sequence);
                    Ok(())
                })
                .unwrap(),
            Poll::Consumed(200)
        );
    }
    assert_eq!(seen.len(), 600);
    assert_eq!(seen.first(), Some(&1));
    assert_eq!(seen.last(), Some(&600));
}

#[test]
fn completion_queue_never_changes_the_legacy_mailboxes() {
    let relay = CodexHookReportRelay::new().unwrap();
    let identity = relay
        .mailbox_path()
        .join(format!("{}.json", "a".repeat(32)));
    let approval = relay.mailbox_path().join("approval-00.json");
    std::fs::write(&identity, b"legacy identity").unwrap();
    std::fs::write(&approval, b"legacy approval").unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    assert_eq!(
        queue
            .consume("root", |_| panic!("empty queue applied"))
            .unwrap(),
        Poll::Consumed(0)
    );
    assert_eq!(std::fs::read(&identity).unwrap(), b"legacy identity");
    assert_eq!(std::fs::read(&approval).unwrap(), b"legacy approval");
}

#[test]
fn completion_queue_counter_exhaustion_is_terminal_even_without_pending_records() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    queue.accepted = u64::MAX - 1;
    let path = queue.path().join("queue.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    state["acknowledgedSequence"] = serde_json::json!(u64::MAX - 1);
    state["nextSequence"] = serde_json::json!(u64::MAX);
    std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(queue
        .consume("root", |_| panic!("exhausted stream applied"))
        .is_err());
}

#[test]
fn completion_queue_ack_does_not_overwrite_a_raced_queue_replacement() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_fixture(&queue, vec![fixture_record(1)]);
    let path = queue.path().join("queue.json");
    let replacement = queue.path().join("replacement");
    let mut applied = 0;
    let result = queue.consume("root", |_| {
        applied += 1;
        std::fs::write(&replacement, b"unexpected replacement")?;
        std::fs::set_permissions(&replacement, std::fs::Permissions::from_mode(0o600))?;
        std::fs::rename(&replacement, &path)
    });
    assert!(result.is_err());
    assert_eq!(applied, 1);
    assert_eq!(std::fs::read(&path).unwrap(), b"unexpected replacement");
    assert!(queue
        .consume("root", |_| panic!("raced effect replayed"))
        .is_err());
}

pub(crate) fn publish_native(path: &Path, event: &str, invocation: Option<&str>) {
    use std::process::{Command, Stdio};
    let mut child = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/codex-completion-report.py"
        ))
        .arg(path)
        .arg(event)
        .arg("a".repeat(64))
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let tool =
        matches!(event, "PreToolUse" | "PermissionRequest" | "PostToolUse").then_some("Bash");
    let raw = serde_json::to_vec(&serde_json::json!({"session_id":"root","turn_id":"turn","hook_event_name":event,"tool_name":tool,"tool_use_id":invocation,"tool_input":{"command":"must-not-be-retained"}})).unwrap();
    child.stdin.take().unwrap().write_all(&raw).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
}

#[test]
fn completion_queue_accepts_real_parallel_publishers_without_receive_order_counters() {
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    let threads: Vec<_> = (0..12)
        .map(|i| {
            let path = queue.path().to_path_buf();
            std::thread::spawn(move || {
                publish_native(&path, "PreToolUse", Some(&format!("tool-{i}")))
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let bytes = std::fs::read(queue.path().join("queue.json")).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("must-not-be-retained"));
    let mut seen = vec![];
    assert_eq!(
        queue
            .consume("root", |e| {
                seen.push(e.sequence);
                Ok(())
            })
            .unwrap(),
        Poll::Consumed(12)
    );
    assert_eq!(seen, (1..=12).collect::<Vec<_>>());
    assert_eq!(
        queue
            .consume("root", |_| panic!("publisher records replayed"))
            .unwrap(),
        Poll::Consumed(0)
    );
}

#[test]
fn completion_queue_malformed_publisher_poison_prevents_prior_record_application() {
    use std::process::{Command, Stdio};
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_native(queue.path(), "UserPromptSubmit", None);
    let mut child = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/codex-completion-report.py"
        ))
        .arg(queue.path())
        .arg("PreToolUse")
        .arg("a".repeat(64))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"not-json").unwrap();
    assert_eq!(child.wait_with_output().unwrap().status.code(), Some(2));
    assert!(queue
        .consume("root", |_| panic!("poisoned publisher stream applied"))
        .is_err());
}

#[test]
fn completion_queue_retained_lock_blocks_real_publisher_marker_creation() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    publish_fixture(&queue, vec![fixture_record(1)]);
    let path = queue.path().to_path_buf();
    let code = r#"
import importlib.util,sys,fcntl
spec=importlib.util.spec_from_file_location('publisher',sys.argv[1])
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
original=m._lock
first=True
def observed_lock(fd):
 global first
 if first:
  first=False
  try:
   fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
  except BlockingIOError:
   print('blocked',flush=True)
  else:
   print('unexpected-acquisition',flush=True)
   raise RuntimeError('Rust consumer did not retain shared lock')
 return original(fd)
m._lock=observed_lock
m.publish(sys.argv[2],'PreToolUse','a'*64)
"#;
    let mut publisher = None;
    assert_eq!(queue.consume("root", |_| {
        let mut child = Command::new("python3").arg("-c").arg(code)
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/codex-completion-report.py"))
            .arg(&path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        child.stdin.take().unwrap().write_all(br#"{"hook_event_name":"PreToolUse","session_id":"root","turn_id":"turn","tool_name":"Bash","tool_use_id":"tool-2"}"#)?;
        let mut ready=String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready)?;
        assert_eq!(ready.trim(), "blocked");
        assert!(!std::fs::read_dir(&path)?.any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with("inflight-")));
        publisher=Some(child);
        Ok(())
    }).unwrap(), Poll::Consumed(1));
    let output = publisher.unwrap().wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut seen = vec![];
    assert_eq!(
        queue
            .consume("root", |e| {
                seen.push(e.sequence);
                Ok(())
            })
            .unwrap(),
        Poll::Consumed(1)
    );
    assert_eq!(seen, [2]);
}

#[test]
fn completion_queue_late_publisher_sync_failure_is_visible_before_unlock() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let relay = CodexHookReportRelay::new().unwrap();
    let mut queue = CompletionQueue::create(&relay, 7, &"a".repeat(64)).unwrap();
    let code = r#"
import importlib.util,sys,stat
spec=importlib.util.spec_from_file_location('publisher',sys.argv[1])
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
read=m._read_bounded
payload=b'{"hook_event_name":"UserPromptSubmit","session_id":"root","turn_id":"turn"}'
m._read_bounded=lambda fd,limit: payload if fd==sys.stdin.fileno() else read(fd,limit)
unlink=m.os.unlink
sync=m.os.fsync
flock=m.fcntl.flock
removed=False
injected=False
announced=False
def observed_unlink(name,*args,**kwargs):
 global removed
 result=unlink(name,*args,**kwargs)
 if str(name).startswith('inflight-'): removed=True
 return result
def failed_sync(fd):
 global injected
 if removed and not injected and stat.S_ISDIR(m.os.fstat(fd).st_mode):
  injected=True
  raise OSError('injected post-unlink directory sync failure')
 return sync(fd)
def observed_unlock(fd,operation):
 global announced
 result=flock(fd,operation)
 if injected and operation==m.fcntl.LOCK_UN and not announced:
  announced=True
  print('released-after-failure',flush=True)
  sys.stdin.buffer.readline()
 return result
m.os.unlink=observed_unlink
m.os.fsync=failed_sync
m.fcntl.flock=observed_unlock
try:
 m.publish(sys.argv[2],'UserPromptSubmit','a'*64)
except BaseException:
 sys.stderr.write(m.ERROR)
 sys.exit(2)
"#;
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(code)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/codex-completion-report.py"
        ))
        .arg(queue.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert_eq!(ready.trim(), "released-after-failure");
    let mut applied = vec![];
    let result = queue.consume("root", |event| {
        applied.push(event.sequence);
        Ok(())
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"continue\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        result.is_err() || result.unwrap() == Poll::Busy,
        "failed publisher appeared clean"
    );
    assert!(
        applied.is_empty(),
        "failed publication reached the application"
    );
}
