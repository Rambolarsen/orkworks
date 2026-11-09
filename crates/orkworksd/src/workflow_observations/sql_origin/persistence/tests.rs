use super::*;
use crate::session_store::{ObservationSource as SqlSource, SessionStore};
use std::{
    fs::File,
    time::{Duration, Instant},
};

async fn fixture() -> (tempfile::TempDir, SessionStore, SqlSource, File) {
    let root = tempfile::tempdir().unwrap();
    let store = SessionStore::initialize(root.path().into(), "/workspace".into())
        .await
        .unwrap();
    let meta = crate::test_support::test_session_metadata(
        "s",
        "Before",
        "/workspace",
        "running",
        "2026-01-01T00:00:00Z",
        "2026-01-02T00:00:00Z",
    );
    store
        .create(serde_json::to_value(meta).unwrap())
        .await
        .unwrap();
    let source = store
        .capture_observation_source("s".into(), "2026-01-01T00:00:00Z".into())
        .await
        .unwrap();
    fs::create_dir(root.path().join("workflow-observations")).unwrap();
    let directory = File::open(root.path().join("workflow-observations")).unwrap();
    (root, store, source, directory)
}
fn candidate() -> ObservationCandidate {
    ObservationCandidate {
        kind: ObservationKind::Obstacle,
        description: "blocked".into(),
        evidence: "error".into(),
        problem_area: None,
        reported_impact: Impact::Low,
        confidence: Some(0.8),
    }
}
fn now() -> DateTime<Utc> {
    parse_time("2026-11-09T00:00:00Z").unwrap()
}
fn write(
    directory: &File,
    source: &SqlSource,
    key: &str,
) -> std::result::Result<RecordOutcome, WriteError> {
    persist_files(
        directory,
        ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
        source,
        ObservationOrigin::Peon,
        key,
        candidate(),
        now(),
    )
}
#[tokio::test]
async fn physical_append_retains_original_scope_and_duplicate_has_zero_file_effects() {
    let (root, store, source, directory) = fixture().await;
    let RecordOutcome::Accepted(first) = write(&directory, &source, "key").unwrap() else {
        panic!("accepted")
    };
    let path = root.path().join("workflow-observations/s.ndjson");
    let before = fs::read(&path).unwrap();
    let record = decode_line(before.strip_suffix(b"\n").unwrap(), "s").unwrap();
    let origin = record.origin().unwrap();
    assert_eq!(origin.created_at(), source.created_at());
    assert_eq!(origin.database_epoch(), source.database_epoch());
    assert_eq!(record.observation().unwrap(), &first);
    assert_eq!(
        write(&directory, &source, "key").unwrap(),
        RecordOutcome::Duplicate {
            observation_id: first.id,
            sequence: 1,
            accepted_at: first.observed_at
        }
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn physical_writer_refuses_partial_tail_without_advancing_counter_or_repairing() {
    let (root, store, source, directory) = fixture().await;
    write(&directory, &source, "key").unwrap();
    let path = root.path().join("workflow-observations/s.ndjson");
    let mut before = fs::read(&path).unwrap();
    before.extend_from_slice(b"partial");
    fs::write(&path, &before).unwrap();
    assert!(matches!(
        write(&directory, &source, "other"),
        Err(WriteError::Storage(StoreError::Corrupt))
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn failed_append_keeps_a_gap_and_partial_append_is_preserved() {
    for partial in [false, true] {
        let (root, store, source, directory) = fixture().await;
        native::FAIL.with(|slot| {
            slot.set(Some(if partial {
                native::Fault::PartialAppend
            } else {
                native::Fault::AfterCounter
            }))
        });
        assert!(write(&directory, &source, "key").is_err());
        let dir = root.path().join("workflow-observations");
        assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
        if partial {
            let bytes = fs::read(dir.join("s.ndjson")).unwrap();
            assert!(!bytes.ends_with(b"\n"));
            assert!(write(&directory, &source, "key").is_err());
            assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), bytes);
        } else {
            assert!(
                matches!(write(&directory,&source,"key").unwrap(),RecordOutcome::Accepted(v) if v.sequence==2)
            );
        }
        store.close().await.unwrap();
    }
}
fn seed(source: &SqlSource, sequence: u64, key: &str, recent: bool) -> RecordData {
    let mut value = super::super::tests::line();
    value["observation"]["id"] =
        serde_json::json!(uuid::Uuid::from_u128(sequence as u128).to_string());
    value["observation"]["sequence"] = serde_json::json!(sequence);
    value["observation"]["observedAt"] = serde_json::json!(if recent {
        "2026-11-09T00:00:00Z"
    } else {
        "2026-11-08T00:00:00Z"
    });
    value["origin"]["databaseEpoch"] = serde_json::json!(source.database_epoch());
    value["origin"]["workspaceSha256"] = serde_json::json!(source.workspace_sha256());
    value["origin"]["createdAt"] = serde_json::json!(source.created_at());
    value["origin"]["observationSha256"] = serde_json::json!(value_digest(
        &serde_json::from_value(value["observation"].clone()).unwrap()
    )
    .unwrap());
    value["idempotencyKeyHash"] = serde_json::json!(hash_key("s", key));
    decode_line(&serde_json::to_vec(&value).unwrap(), "s").unwrap()
}
fn seed_file(root: &Path, records: &[RecordData], counter: u64) {
    let dir = root.join("workflow-observations");
    let mut bytes = Vec::new();
    for r in records {
        bytes.extend(r.encode().unwrap());
        bytes.push(b'\n');
    }
    fs::write(dir.join("s.ndjson"), bytes).unwrap();
    fs::write(dir.join("sequence"), counter.to_string()).unwrap();
}
#[tokio::test]
async fn physical_count_rewrite_copies_evicted_origin_and_tombstone_retry_is_read_only() {
    let (root, store, source, directory) = fixture().await;
    let records: Vec<_> = (1..=1000)
        .map(|i| seed(&source, i, &format!("old-{i}"), i == 1))
        .collect();
    seed_file(root.path(), &records, 1000);
    assert!(
        matches!(write(&directory,&source,"new").unwrap(),RecordOutcome::Accepted(v) if v.sequence==1001)
    );
    let path = root.path().join("workflow-observations/s.ndjson");
    let bytes = fs::read(&path).unwrap();
    let retained: Vec<_> = bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| decode_line(l, "s").unwrap())
        .collect();
    assert_eq!(
        retained
            .iter()
            .filter(|r| r.observation().is_some())
            .count(),
        1000
    );
    let tomb = retained.iter().find(|r| r.sequence() == 1).unwrap();
    assert!(tomb.observation().is_none());
    assert_eq!(tomb.origin(), records[0].origin());
    assert!(matches!(
        write(&directory, &source, "old-1").unwrap(),
        RecordOutcome::Duplicate { sequence: 1, .. }
    ));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1001"
    );
    for original in &records[1..] {
        assert!(retained.contains(original));
    }
    store.close().await.unwrap();
}
#[tokio::test]
async fn failed_rewrite_preserves_original_segment_and_non_target_bytes() {
    let (root, store, source, directory) = fixture().await;
    let records: Vec<_> = (1..=1000)
        .map(|i| seed(&source, i, &format!("old-{i}"), false))
        .collect();
    seed_file(root.path(), &records, 1000);
    let path = root.path().join("workflow-observations/s.ndjson");
    let before = fs::read(&path).unwrap();
    fs::write(
        root.path().join("workflow-observations/other.ndjson"),
        b"\n",
    )
    .unwrap();
    native::FAIL.with(|slot| slot.set(Some(native::Fault::BeforeRename)));
    assert!(write(&directory, &source, "new").is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read(root.path().join("workflow-observations/other.ndjson")).unwrap(),
        b"\n"
    );
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1001"
    );
    assert_eq!(
        fs::read_dir(root.path().join("workflow-observations"))
            .unwrap()
            .count(),
        3
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn physical_retry_collision_and_counter_overflow_refuse_before_effects() {
    let (root, store, source, directory) = fixture().await;
    write(&directory, &source, "key").unwrap();
    let dir = root.path().join("workflow-observations");
    let before = fs::read(dir.join("s.ndjson")).unwrap();
    let mut changed = candidate();
    changed.evidence = "different".into();
    assert!(persist_files(
        &directory,
        ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
        &source,
        ObservationOrigin::Peon,
        "key",
        changed,
        now()
    )
    .is_err());
    assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
    fs::write(dir.join("sequence"), u64::MAX.to_string()).unwrap();
    assert!(matches!(
        write(&directory, &source, "key"),
        Ok(RecordOutcome::Duplicate { sequence: 1, .. })
    ));
    assert!(matches!(
        write(&directory, &source, "other"),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(
        fs::read(dir.join("sequence")).unwrap(),
        u64::MAX.to_string().as_bytes()
    );
    assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test]
async fn oversized_intent_refuses_before_any_counter_or_segment_creation() {
    let (root, store, source, directory) = fixture().await;
    assert!(matches!(
        write(&directory, &source, &"x".repeat(64 * 1024 + 1)),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(
        fs::read_dir(root.path().join("workflow-observations"))
            .unwrap()
            .count(),
        0
    );
    // Escaped representation, including provenance, is charged too.
    assert!(matches!(
        write(&directory, &source, &"\u{0001}".repeat(12 * 1024)),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(
        fs::read_dir(root.path().join("workflow-observations"))
            .unwrap()
            .count(),
        0
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn native_identity_replacement_during_preflight_refuses_before_counter_effects() {
    for replace_counter in [false, true] {
        let (root, store, source, directory) = fixture().await;
        write(&directory, &source, "key").unwrap();
        let dir = root.path().join("workflow-observations");
        let changed = dir.join(if replace_counter {
            "sequence"
        } else {
            "s.ndjson"
        });
        AFTER_SCAN.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move || {
                fs::rename(&changed, changed.with_extension("original")).unwrap();
                fs::write(&changed, b"replacement").unwrap();
            }))
        });
        assert!(matches!(
            write(&directory, &source, "other"),
            Err(WriteError::Storage(StoreError::Conflict))
        ));
        assert_eq!(
            fs::read(dir.join(if replace_counter {
                "sequence"
            } else {
                "s.ndjson"
            }))
            .unwrap(),
            b"replacement"
        );
        if !replace_counter {
            assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
        }
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn linked_files_and_unknown_inventory_refuse_without_repair_or_sequence_changes() {
    for variant in 0..4 {
        let (root, store, source, directory) = fixture().await;
        write(&directory, &source, "key").unwrap();
        let dir = root.path().join("workflow-observations");
        let original = fs::read(dir.join("s.ndjson")).unwrap();
        let outside = root.path().join("outside");
        fs::write(&outside, &original).unwrap();
        match variant {
            0 => {
                fs::remove_file(dir.join("s.ndjson")).unwrap();
                std::os::unix::fs::symlink(&outside, dir.join("s.ndjson")).unwrap();
            }
            1 => {
                fs::remove_file(dir.join("s.ndjson")).unwrap();
                fs::hard_link(&outside, dir.join("s.ndjson")).unwrap();
            }
            2 => {
                fs::write(dir.join("unknown.tmp"), b"recoverable").unwrap();
            }
            _ => {
                fs::write(dir.join("other.ndjson"), b"partial").unwrap();
            }
        }
        assert!(write(&directory, &source, "other").is_err());
        assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
        assert_eq!(fs::read(&outside).unwrap(), original);
        store.close().await.unwrap();
    }
}
#[tokio::test]
async fn physical_rate_limit_counts_retry_tombstones_but_never_blocks_duplicate_receipts() {
    let (root, store, source, directory) = fixture().await;
    let records: Vec<_> = (1..=60)
        .map(|i| {
            mutation::evicted_retry("s", &seed(&source, i, &format!("key-{i}"), true), now())
                .unwrap()
                .unwrap()
        })
        .collect();
    seed_file(root.path(), &records, 60);
    let before = fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap();
    assert!(matches!(
        write(&directory, &source, "key-1"),
        Ok(RecordOutcome::Duplicate { sequence: 1, .. })
    ));
    assert!(matches!(
        write(&directory, &source, "new"),
        Err(WriteError::Candidate(RecordError::RateLimited))
    ));
    assert_eq!(
        fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap(),
        before
    );
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"60"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn complete_inventory_preserves_legacy_refusal_and_foreign_creation_retry_conflicts() {
    for legacy in [false, true] {
        let (root, store, source, directory) = fixture().await;
        let mut record = seed(&source, 1, "key", true);
        let Record::Observation(v) = &mut record.0 else {
            unreachable!()
        };
        if legacy {
            record = RecordData(Record::LegacyObservation(StoredObservation {
                observation: v.observation.clone(),
                idempotency_key_hash: v.idempotency_key_hash.clone(),
                payload_hash: v.payload_hash.clone(),
            }));
        } else {
            v.origin.0.created_at = "reused".into();
        }
        seed_file(root.path(), &[record], 1);
        let before = fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap();
        assert!(matches!(
            write(&directory, &source, "key"),
            Err(WriteError::Storage(StoreError::Conflict))
        ));
        assert_eq!(
            fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap(),
            before
        );
        assert_eq!(
            fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
            b"1"
        );
        store.close().await.unwrap();
    }
}

#[tokio::test]
async fn blank_bytes_at_segment_limit_force_a_bounded_rewrite() {
    let (root, store, source, directory) = fixture().await;
    let dir = root.path().join("workflow-observations");
    let mut block = vec![b' '; 64 * 1024];
    *block.last_mut().unwrap() = b'\n';
    fs::write(dir.join("s.ndjson"), block.repeat(32)).unwrap();
    fs::write(dir.join("sequence"), b"0").unwrap();
    assert!(matches!(
        write(&directory, &source, "key"),
        Ok(RecordOutcome::Accepted(_))
    ));
    assert!(fs::metadata(dir.join("s.ndjson")).unwrap().len() <= MAX_SEGMENT_BYTES);
    let snapshot = ScanRequest::capture(Instant::now() + Duration::from_secs(2))
        .unwrap()
        .read_at(
            &directory,
            source.database_epoch(),
            source.workspace_sha256(),
        )
        .unwrap();
    assert_eq!(snapshot.records().len(), 1);
    drop(snapshot);
    store.close().await.unwrap();
}

#[tokio::test]
async fn byte_limited_rewrite_charges_origins_and_preserves_the_evicted_value_digest() {
    let (root, store, source, directory) = fixture().await;
    let mut records: Vec<_> = (1..=800)
        .map(|i| seed(&source, i, &format!("old-{i}"), i == 1))
        .collect();
    let base: usize = records.iter().map(|r| r.encode().unwrap().len() + 1).sum();
    let mut remaining = MAX_SEGMENT_BYTES as usize - base;
    for record in &mut records {
        let Record::Observation(v) = &mut record.0 else {
            unreachable!()
        };
        let extra = remaining.min(MAX_EVIDENCE_CHARS - v.observation.evidence.len());
        v.observation.evidence.push_str(&"e".repeat(extra));
        remaining -= extra;
        v.payload_hash = hash_payload(
            v.observation.kind,
            &v.observation.description,
            &v.observation.evidence,
            v.observation.reported_impact,
            None,
        );
        v.origin.0.observation_sha256 = value_digest(&v.observation).unwrap();
    }
    assert_eq!(remaining, 0);
    seed_file(root.path(), &records, 800);
    let path = root.path().join("workflow-observations/s.ndjson");
    assert_eq!(fs::metadata(&path).unwrap().len(), MAX_SEGMENT_BYTES);
    write(&directory, &source, "new").unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.len() as u64 <= MAX_SEGMENT_BYTES);
    let retained: Vec<_> = bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| decode_line(l, "s").unwrap())
        .collect();
    assert_eq!(
        retained
            .iter()
            .filter(|r| r.observation().is_some())
            .count(),
        800
    );
    let tomb = retained.iter().find(|r| r.sequence() == 1).unwrap();
    assert_eq!(tomb.origin(), records[0].origin());
    assert!(tomb.observation().is_none());
    for record in &records[1..] {
        assert!(retained.contains(record));
    }
    store.close().await.unwrap();
}
#[tokio::test]
async fn descriptor_writes_do_not_adopt_a_replacement_workflow_directory() {
    let (root, store, source, directory) = fixture().await;
    let old = root.path().join("original");
    fs::rename(root.path().join("workflow-observations"), &old).unwrap();
    fs::create_dir(root.path().join("workflow-observations")).unwrap();
    fs::write(
        root.path().join("workflow-observations/s.ndjson"),
        b"replacement",
    )
    .unwrap();
    write(&directory, &source, "key").unwrap();
    assert_eq!(
        fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap(),
        b"replacement"
    );
    assert!(old.join("s.ndjson").exists());
    store.close().await.unwrap();
}
#[test]
fn rewrite_read_budget_reserves_the_later_copy_before_effects() {
    let mut budget = ReadBudget {
        used: reader::MAX_TOTAL_BYTES - 100,
    };
    assert!(budget.check(100).is_ok());
    assert!(matches!(budget.check(101), Err(StoreError::TooLarge)));
    budget.charge(100).unwrap();
    assert!(matches!(budget.charge(1), Err(StoreError::TooLarge)));
}

#[tokio::test]
async fn writer_intent_accepts_exact_encoded_allowance_and_refuses_one_extra_byte() {
    let (root, store, source, directory) = fixture().await;
    let value = candidate();
    let base = serde_json::to_vec(&(
        &source,
        "",
        &value.description,
        &value.evidence,
        &value.problem_area,
        value.kind,
        value.reported_impact,
        value.confidence,
    ))
    .unwrap()
    .len();
    let key = "x".repeat(MAX_INTENT - base);
    assert!(matches!(
        write(&directory, &source, &key),
        Ok(RecordOutcome::Accepted(_))
    ));
    let dir = root.path().join("workflow-observations");
    let before = fs::read(dir.join("s.ndjson")).unwrap();
    assert!(matches!(
        write(&directory, &source, &(key + "x")),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
    assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), before);
    store.close().await.unwrap();
}

#[tokio::test]
async fn raw_intent_budget_precedes_normalization_and_candidate_validation() {
    let (root, store, source, directory) = fixture().await;
    let mut value = candidate();
    value.problem_area = Some("\u{0000}".repeat(MAX_INTENT + 1));
    let result = persist_files(
        &directory,
        ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap(),
        &source,
        ObservationOrigin::Peon,
        "key",
        value,
        now(),
    );
    assert!(matches!(
        result,
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(
        fs::read_dir(root.path().join("workflow-observations"))
            .unwrap()
            .count(),
        0
    );
    store.close().await.unwrap();
}

fn numeric_write(
    directory: &File,
    source: &SqlSource,
    key: &str,
) -> std::result::Result<RecordOutcome, WriteError> {
    persist_files(
        directory,
        ScanRequest::capture(Instant::now() + Duration::from_secs(2))
            .unwrap()
            .for_numeric_test(),
        source,
        ObservationOrigin::Peon,
        key,
        candidate(),
        now(),
    )
}
fn fill_blank_inventory(root: &Path, mut bytes: usize, target: bool) {
    let dir = root.join("workflow-observations");
    let mut index = 0;
    while bytes > 0 {
        let n = bytes.min(MAX_SEGMENT_BYTES as usize);
        let mut data = vec![b' '; n];
        for end in (64 * 1024 - 1..n).step_by(64 * 1024) {
            data[end] = b'\n';
        }
        *data.last_mut().unwrap() = b'\n';
        let name = if target && bytes == n {
            "s".to_owned()
        } else {
            format!("other-{index}")
        };
        fs::write(dir.join(format!("{name}.ndjson")), data).unwrap();
        bytes -= n;
        index += 1;
    }
    fs::write(dir.join("sequence"), b"0").unwrap();
}
#[tokio::test]
async fn projected_workspace_bytes_accept_exact_limit_then_refuse_overflow_before_counter() {
    let (root, store, source, directory) = fixture().await;
    write(&directory, &source, "sizing").unwrap();
    let dir = root.path().join("workflow-observations");
    let line_len = fs::read(dir.join("s.ndjson")).unwrap().len();
    fs::remove_file(dir.join("s.ndjson")).unwrap();
    fill_blank_inventory(root.path(), reader::MAX_TOTAL_BYTES - line_len - 1, false);
    assert!(matches!(
        numeric_write(&directory, &source, "key"),
        Ok(RecordOutcome::Accepted(_))
    ));
    assert_eq!(
        fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().len())
            .sum::<u64>(),
        reader::MAX_TOTAL_BYTES as u64
    );
    let before = fs::read(dir.join("s.ndjson")).unwrap();
    assert!(matches!(
        numeric_write(&directory, &source, "other"),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
    assert_eq!(fs::read(dir.join("s.ndjson")).unwrap(), before);
    store.close().await.unwrap();
}
#[tokio::test]
async fn projected_workspace_bytes_subtract_replaced_segment_before_accepting_rewrite() {
    for counter in [0, 9] {
        let (root, store, source, directory) = fixture().await;
        fill_blank_inventory(root.path(), reader::MAX_TOTAL_BYTES - 1, true);
        let dir = root.path().join("workflow-observations");
        fs::write(dir.join("sequence"), counter.to_string()).unwrap();
        let result = numeric_write(&directory, &source, "key");
        if counter == 0 {
            assert!(matches!(result, Ok(RecordOutcome::Accepted(_))));
            assert!(
                fs::read_dir(&dir)
                    .unwrap()
                    .map(|e| e.unwrap().metadata().unwrap().len())
                    .sum::<u64>()
                    < reader::MAX_TOTAL_BYTES as u64
            );
            assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"1");
        } else {
            // Counter9->10 alone must not overflow the global read allowance,
            // even though the eventual segment rewrite would reclaim room.
            assert!(matches!(
                result,
                Err(WriteError::Storage(StoreError::TooLarge))
            ));
            assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"9");
            assert_eq!(
                fs::read_dir(&dir)
                    .unwrap()
                    .map(|e| e.unwrap().metadata().unwrap().len())
                    .sum::<u64>(),
                reader::MAX_TOTAL_BYTES as u64
            );
        }
        store.close().await.unwrap();
    }
}
fn fill_line_inventory(root: &Path, target: bool) {
    let dir = root.join("workflow-observations");
    let mut sequence = 1u64;
    if target {
        fs::write(dir.join("s.ndjson"), legacy_line(sequence)).unwrap();
        sequence += 1;
    }
    let mut file = 0;
    while sequence <= 65536 {
        let mut bytes = Vec::new();
        for _ in 0..MAX_TOMBSTONES {
            if sequence > 65536 {
                break;
            }
            bytes.extend(legacy_line(sequence));
            sequence += 1;
        }
        fs::write(dir.join(format!("other-{file}.ndjson")), bytes).unwrap();
        file += 1;
    }
    fs::write(dir.join("sequence"), b"65536").unwrap();
}
fn legacy_line(sequence: u64) -> Vec<u8> {
    let mut bytes=serde_json::to_vec(&serde_json::json!({"record":"tombstone","idempotencyKeyHash":"f".repeat(64),"payloadHash":"f".repeat(64),"observationId":uuid::Uuid::from_u128(sequence as u128).to_string(),"sequence":sequence,"acceptedAt":"2026-11-08T00:00:00Z"})).unwrap();
    bytes.push(b'\n');
    bytes
}
#[tokio::test]
async fn projected_workspace_lines_refuse_append_but_allow_rewrite_reclaiming_a_record() {
    for reclaim in [false, true] {
        let (root, store, source, directory) = fixture().await;
        fill_line_inventory(root.path(), reclaim);
        let dir = root.path().join("workflow-observations");
        let result = numeric_write(&directory, &source, "key");
        if reclaim {
            assert!(matches!(result,Ok(RecordOutcome::Accepted(v)) if v.sequence==65537));
            assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"65537");
        } else {
            assert!(matches!(
                result,
                Err(WriteError::Storage(StoreError::TooLarge))
            ));
            assert!(!dir.join("s.ndjson").exists());
            assert_eq!(fs::read(dir.join("sequence")).unwrap(), b"65536");
        }
        store.close().await.unwrap();
    }
}
