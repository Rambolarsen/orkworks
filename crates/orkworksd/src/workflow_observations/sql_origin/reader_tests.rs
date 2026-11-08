use super::*;
const EPOCH: &str = "12345678-1234-4234-8234-123456789abc";
const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod supported {
    use super::*;
    use crate::workflow_observations::sql_origin::{tests::line, value_digest};
    use serde_json::{json, Value};
    use std::{collections::BTreeMap, io::Write};
    fn request() -> ScanRequest {
        ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap()
    }
    // Isolate numeric/streaming bounds from timing; production capture still caps at two seconds.
    fn count_request() -> ScanRequest {
        let mut r = request();
        r.deadline = Instant::now() + Duration::from_secs(120);
        r
    }
    fn scan(root: &std::path::Path) -> Result<Snapshot> {
        request().read_at(&File::open(root).unwrap(), EPOCH, HASH)
    }
    fn write(root: &std::path::Path, session: &str, values: &[Value], counter: u64) {
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend(serde_json::to_vec(value).unwrap());
            bytes.push(b'\n');
        }
        std::fs::write(root.join(format!("{session}.ndjson")), bytes).unwrap();
        std::fs::write(root.join("sequence"), counter.to_string()).unwrap();
    }
    fn observation(index: u64, session: &str) -> Value {
        let mut v = line();
        v["observation"]["id"] = json!(uuid::Uuid::from_u128(u128::from(index)).to_string());
        v["observation"]["sequence"] = json!(index);
        v["observation"]["sessionId"] = json!(session);
        v["origin"]["sessionId"] = json!(session);
        refresh(&mut v);
        v
    }
    fn refresh(v: &mut Value) {
        let o: crate::workflow_observations::WorkflowObservation =
            serde_json::from_value(v["observation"].clone()).unwrap();
        v["origin"]["observationSha256"] = json!(value_digest(&o).unwrap());
        v["payloadHash"] = json!(crate::workflow_observations::hash_payload(
            o.kind,
            &o.description,
            &o.evidence,
            o.reported_impact,
            o.problem_area.as_deref()
        ));
    }
    fn tombstone(index: u64, session: &str) -> Value {
        json!({"record":"tombstone","idempotencyKeyHash":crate::workflow_observations::hash_key(session,&index.to_string()),"payloadHash":"b".repeat(64),"observationId":uuid::Uuid::from_u128(u128::from(index)).to_string(),"sequence":index,"acceptedAt":"2026-11-08T00:00:00Z"})
    }
    #[test]
    fn invalid_segments_counters_and_scope_refuse_without_repair_or_empty_fallback() {
        for case in [
            "tail",
            "interior",
            "counter",
            "missing-counter",
            "lower-counter",
            "foreign-epoch",
            "foreign-workspace",
            "duplicate",
            "unknown-entry",
        ] {
            let root = tempfile::tempdir().unwrap();
            write(root.path(), "s", &[observation(1, "s")], 1);
            let path = root.path().join("s.ndjson");
            match case {
                "tail" => {
                    let mut f = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .unwrap();
                    f.write_all(b"{broken").unwrap();
                }
                "interior" => {
                    let mut b = b"broken\n".to_vec();
                    b.extend(std::fs::read(&path).unwrap());
                    std::fs::write(&path, b).unwrap();
                }
                "counter" => std::fs::write(root.path().join("sequence"), b"broken").unwrap(),
                "missing-counter" => std::fs::remove_file(root.path().join("sequence")).unwrap(),
                "lower-counter" => std::fs::write(root.path().join("sequence"), b"0").unwrap(),
                "foreign-epoch" => {
                    let mut v = observation(1, "s");
                    v["origin"]["databaseEpoch"] = json!(uuid::Uuid::new_v4().to_string());
                    write(root.path(), "s", &[v], 1);
                }
                "foreign-workspace" => {
                    let mut v = observation(1, "s");
                    v["origin"]["workspaceSha256"] = json!("b".repeat(64));
                    write(root.path(), "s", &[v], 1);
                }
                "duplicate" => write(
                    root.path(),
                    "s",
                    &[observation(1, "s"), observation(1, "s")],
                    1,
                ),
                "unknown-entry" => {
                    std::fs::write(root.path().join("s.ndjson.tmp"), b"keep").unwrap()
                }
                _ => unreachable!(),
            }
            let before = std::fs::read(&path).unwrap();
            assert!(scan(root.path()).is_err(), "{case}");
            assert_eq!(std::fs::read(path).unwrap(), before, "{case}");
        }
        let root = tempfile::tempdir().unwrap();
        let empty = scan(root.path()).unwrap();
        assert!(empty.records().is_empty());
        assert_eq!(empty.sequence(), None);
    }
    #[test]
    fn unterminated_whitespace_tail_is_corrupt_without_repair() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "s", &[observation(1, "s")], 1);
        let path = root.path().join("s.ndjson");
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(b" \t");
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(scan(root.path()), Err(StoreError::Corrupt)));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        bytes.push(b'\n');
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(scan(root.path()).unwrap().records().len(), 1);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn raw_line_and_segment_byte_boundaries_refuse_one_extra_without_changes() {
        let root = tempfile::tempdir().unwrap();
        let mut bytes = serde_json::to_vec(&observation(1, "s")).unwrap();
        bytes.resize(MAX_LINE_BYTES, b' ');
        bytes.push(b'\n');
        std::fs::write(root.path().join("s.ndjson"), &bytes).unwrap();
        std::fs::write(root.path().join("sequence"), b"1").unwrap();
        assert_eq!(scan(root.path()).unwrap().records().len(), 1);
        bytes.insert(bytes.len() - 1, b' ');
        std::fs::write(root.path().join("s.ndjson"), &bytes).unwrap();
        assert!(matches!(scan(root.path()), Err(StoreError::TooLarge)));
        assert_eq!(std::fs::read(root.path().join("s.ndjson")).unwrap(), bytes);
        let mut block = vec![b' '; MAX_LINE_BYTES - 1];
        block.push(b'\n');
        let bytes = block.repeat(32);
        std::fs::write(root.path().join("s.ndjson"), &bytes).unwrap();
        assert!(scan(root.path()).unwrap().records().is_empty());
        let mut over = bytes.clone();
        over.push(b' ');
        std::fs::write(root.path().join("s.ndjson"), &over).unwrap();
        assert!(matches!(scan(root.path()), Err(StoreError::TooLarge)));
        assert_eq!(std::fs::read(root.path().join("s.ndjson")).unwrap(), over);
    }
    #[test]
    fn directory_bound_includes_empty_segments_and_refuses_extra_segment() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("sequence"), b"0").unwrap();
        for i in 0..MAX_SEGMENTS {
            File::create(root.path().join(format!("s{i}.ndjson"))).unwrap();
        }
        let directory = File::open(root.path()).unwrap();
        assert!(count_request()
            .read_at(&directory, EPOCH, HASH)
            .unwrap()
            .records()
            .is_empty());
        File::create(root.path().join("extra.ndjson")).unwrap();
        assert!(matches!(
            count_request().read_at(&directory, EPOCH, HASH),
            Err(StoreError::TooLarge)
        ));
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            MAX_ENTRIES + 1
        );
    }
    #[test]
    fn aggregate_read_budget_counts_blank_segment_bytes_and_sequence() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("sequence"), b"0").unwrap();
        let mut block = vec![b' '; MAX_LINE_BYTES - 1];
        block.push(b'\n');
        let bytes = block.repeat(32);
        for i in 0..32 {
            let mut segment = bytes.clone();
            if i == 31 {
                segment.remove(0);
            } // Preserve the final LF at the exact aggregate boundary.
            std::fs::write(root.path().join(format!("s{i}.ndjson")), &segment).unwrap();
        }
        let directory = File::open(root.path()).unwrap();
        assert!(count_request()
            .read_at(&directory, EPOCH, HASH)
            .unwrap()
            .records()
            .is_empty());
        let last = root.path().join("s31.ndjson");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&last)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        let before = std::fs::read(&last).unwrap();
        assert!(matches!(
            count_request().read_at(&directory, EPOCH, HASH),
            Err(StoreError::TooLarge)
        ));
        assert_eq!(std::fs::read(last).unwrap(), before);
    }
    #[test]
    fn global_line_budget_includes_legacy_tombstone_only_segments() {
        let root = tempfile::tempdir().unwrap();
        for segment in 0..64u64 {
            let session = format!("s{segment}");
            let values: Vec<_> = (1..=1024)
                .map(|i| tombstone(segment * 1024 + i, &session))
                .collect();
            write(root.path(), &session, &values, (segment + 1) * 1024);
        }
        let directory = File::open(root.path()).unwrap();
        assert!(count_request()
            .read_at(&directory, EPOCH, HASH)
            .unwrap()
            .records()
            .is_empty());
        write(root.path(), "extra", &[tombstone(65537, "extra")], 65537);
        let before = std::fs::read(root.path().join("extra.ndjson")).unwrap();
        assert!(matches!(
            count_request().read_at(&directory, EPOCH, HASH),
            Err(StoreError::TooLarge)
        ));
        assert_eq!(
            std::fs::read(root.path().join("extra.ndjson")).unwrap(),
            before
        );
    }
    #[test]
    fn per_segment_observation_and_retry_counts_are_not_silently_trimmed() {
        for observations in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let max = if observations { 1000 } else { 1024 };
            let mut values = Vec::new();
            for i in 1..=max + 1 {
                let mut value = if observations {
                    observation(i, "s")
                } else {
                    tombstone(i, "s")
                };
                if observations {
                    // Ordinary legacy values remain unqualified, with their smaller existing encoding.
                    let mut old = value["observation"].clone();
                    old["record"] = json!("observation");
                    old["idempotencyKeyHash"] = value["idempotencyKeyHash"].clone();
                    old["payloadHash"] = value["payloadHash"].clone();
                    value = old;
                }
                values.push(value);
            }
            write(root.path(), "s", &values[..max as usize], max);
            let snapshot = scan(root.path()).unwrap();
            assert_eq!(
                snapshot.records().len(),
                if observations { max as usize } else { 0 }
            );
            assert!(snapshot.records().iter().all(|r| r.origin().is_none()));
            drop(snapshot);
            write(root.path(), "s", &values, max + 1);
            let before = std::fs::read(root.path().join("s.ndjson")).unwrap();
            assert!(matches!(scan(root.path()), Err(StoreError::TooLarge)));
            assert_eq!(std::fs::read(root.path().join("s.ndjson")).unwrap(), before);
        }
    }
    #[test]
    fn selected_snapshot_accepts_exact_encoded_limit_and_never_shrinks_to_fit() {
        let root = tempfile::tempdir().unwrap();
        let (mut values, mut size) = (Vec::<Value>::new(), 2usize);
        for i in 1..10000 {
            let v = observation(i, &format!("s{}", i / 1000));
            let n = super::super::decode_line(
                &serde_json::to_vec(&v).unwrap(),
                v["origin"]["sessionId"].as_str().unwrap(),
            )
            .unwrap()
            .encode()
            .unwrap()
            .len()
                + usize::from(!values.is_empty());
            if size + n > MAX_SELECTED_BYTES {
                break;
            }
            size += n;
            values.push(v);
        }
        let padding = MAX_SELECTED_BYTES - size;
        let last = values.last_mut().unwrap();
        last["observation"]["evidence"] = json!(format!("error{}", " ".repeat(padding)));
        refresh(last);
        let save = |values: &[Value]| {
            let mut sessions: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for v in values {
                sessions
                    .entry(v["origin"]["sessionId"].as_str().unwrap().into())
                    .or_default()
                    .push(v.clone());
            }
            for (session, records) in sessions {
                write(root.path(), &session, &records, values.len() as u64);
            }
        };
        save(&values);
        let snapshot = scan(root.path()).unwrap();
        assert_eq!(
            serde_json::to_vec(snapshot.records()).unwrap().len(),
            MAX_SELECTED_BYTES
        );
        assert_eq!(snapshot.records().len(), values.len());
        drop(snapshot);
        let last = values.last_mut().unwrap();
        let text = last["observation"]["evidence"].as_str().unwrap().to_owned() + "x";
        last["observation"]["evidence"] = json!(text);
        refresh(last);
        save(&values);
        assert!(matches!(scan(root.path()), Err(StoreError::TooLarge)));
    }
    #[test]
    fn original_deadline_includes_wait_and_foreign_scope_never_becomes_empty() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "s", &[observation(1, "s")], 1);
        let directory = File::open(root.path()).unwrap();
        assert!(matches!(
            ScanRequest::capture(Instant::now()),
            Err(StoreError::Expired)
        ));
        let pending = ScanRequest::capture(Instant::now() + Duration::from_millis(5)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(matches!(
            pending.read_at(&directory, EPOCH, HASH),
            Err(StoreError::Expired)
        ));
        assert!(matches!(
            request().read_at(&directory, &uuid::Uuid::new_v4().to_string(), HASH),
            Err(StoreError::Conflict)
        ));
    }
    #[test]
    fn segment_growth_after_validated_metadata_cannot_bypass_its_byte_limit() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "s", &[observation(1, "s")], 1);
        let path = root.path().join("s.ndjson");
        let initial = std::fs::read(&path).unwrap();
        let mut block = vec![b' '; 4095];
        block.push(b'\n');
        let suffix = block.repeat(512);
        let expected = [initial, suffix.clone()].concat();
        let target = path.clone();
        native::AFTER_METADATA.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move || {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(target)
                    .unwrap()
                    .write_all(&suffix)
                    .unwrap();
            }))
        });
        assert!(matches!(
            scan(root.path()),
            Err(StoreError::Conflict | StoreError::TooLarge)
        ));
        assert_eq!(std::fs::read(path).unwrap(), expected);
        assert_eq!(std::fs::read(root.path().join("sequence")).unwrap(), b"1");
    }
    #[test]
    fn directory_descriptor_does_not_follow_replacement_paths() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("root");
        std::fs::create_dir(&root).unwrap();
        write(&root, "s", &[observation(1, "s")], 1);
        let directory = File::open(&root).unwrap();
        std::fs::rename(&root, outer.path().join("original")).unwrap();
        std::fs::create_dir(&root).unwrap();
        write(&root, "s", &[observation(2, "s")], 2);
        let snapshot = request().read_at(&directory, EPOCH, HASH).unwrap();
        assert_eq!(snapshot.sequence(), Some(1));
        assert_eq!(scan(&root).unwrap().sequence(), Some(2));
    }
    #[test]
    fn links_directories_and_fifos_refuse_without_following_or_blocking() {
        for kind in ["symlink", "hardlink", "directory", "fifo"] {
            let root = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(root.path().join("sequence"), b"0").unwrap();
            let leaf = root.path().join("s.ndjson");
            let foreign = outside.path().join("foreign");
            std::fs::write(&foreign, b"keep").unwrap();
            match kind {
                "symlink" => std::os::unix::fs::symlink(&foreign, &leaf).unwrap(),
                "hardlink" => std::fs::hard_link(&foreign, &leaf).unwrap(),
                "directory" => std::fs::create_dir(&leaf).unwrap(),
                "fifo" => {
                    use std::os::unix::ffi::OsStrExt;
                    let name = std::ffi::CString::new(leaf.as_os_str().as_bytes()).unwrap();
                    // SAFETY: owned NUL-terminated temporary path, ordinary FIFO mode.
                    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                }
                _ => unreachable!(),
            }
            let started = Instant::now();
            assert!(scan(root.path()).is_err(), "{kind}");
            assert!(started.elapsed() < Duration::from_secs(2));
            assert_eq!(std::fs::read(&foreign).unwrap(), b"keep");
        }
    }
    #[test]
    fn scan_result_retains_full_job_reservation_until_drop() {
        const FLAG: &str = "ORKWORKS_OBSERVATION_SCAN_BUDGET_TEST";
        if std::env::var_os(FLAG).is_none() {
            let result=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","workflow_observations::sql_origin::reader::tests::supported::scan_result_retains_full_job_reservation_until_drop"]).env(FLAG,"1").output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let directory = File::open(root.path()).unwrap();
        let mut held: Vec<_> = (0..32)
            .map(|_| request().read_at(&directory, EPOCH, HASH).unwrap())
            .collect();
        assert!(matches!(
            ScanRequest::capture(Instant::now() + Duration::from_secs(2)),
            Err(StoreError::QueueFull)
        ));
        held.pop();
        let pending = ScanRequest::capture(Instant::now() + Duration::from_millis(5)).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(matches!(
            pending.read_at(&directory, EPOCH, HASH),
            Err(StoreError::Expired)
        ));
        request().read_at(&directory, EPOCH, HASH).unwrap();
    }
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[test]
fn unqualified_platform_refuses_without_fallback() {
    let file = tempfile::tempfile().unwrap();
    let request = ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap();
    assert!(matches!(
        request.read_at(&file, EPOCH, HASH),
        Err(StoreError::Unsupported)
    ));
}
