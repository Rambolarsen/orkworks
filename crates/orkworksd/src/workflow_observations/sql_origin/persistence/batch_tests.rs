use super::tests::{candidate, fixture, now, seed, seed_file};
use super::*;
use std::time::Duration;
fn request() -> ScanRequest {
    ScanRequest::capture(Instant::now() + Duration::from_secs(2)).unwrap()
}
fn inputs(n: usize) -> Vec<(usize, ObservationCandidate)> {
    (0..n).map(|i| (i, candidate())).collect()
}
#[tokio::test]
async fn five_records_share_one_inventory_and_retry_original_receipts() {
    let (root, store, source, directory) = fixture().await;
    reader::INVENTORY_SCANS.with(|n| n.set(0));
    let first = persist_batch_files(&directory, request(), &source, 7, inputs(5), now).unwrap();
    assert_eq!(reader::INVENTORY_SCANS.with(|n| n.get()), 1);
    assert_eq!(first.len(), 5);
    let mut ids = Vec::new();
    for (index, result) in first {
        let RecordOutcome::Accepted(v) = result.unwrap() else {
            panic!("accepted")
        };
        assert_eq!(v.sequence, index as u64 + 1);
        ids.push(v.id);
    }
    let path = root.path().join("workflow-observations/s.ndjson");
    let before = fs::read(&path).unwrap();
    let retry = persist_batch_files(&directory, request(), &source, 7, inputs(5), now).unwrap();
    for (index, result) in retry {
        assert!(
            matches!(result.unwrap(),RecordOutcome::Duplicate{observation_id,sequence,..}if observation_id==ids[index]&&sequence==index as u64+1)
        );
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"5"
    );
    for line in before.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
        let record = decode_line(line, "s").unwrap();
        let origin = record.origin().unwrap();
        assert_eq!(origin.database_epoch(), source.database_epoch());
        assert_eq!(origin.created_at(), source.created_at());
    }
    store.close().await.unwrap();
}
#[tokio::test]
async fn batch_preserves_candidate_indices_validation_and_conflict_results() {
    let (root, store, source, directory) = fixture().await;
    persist_batch_files(&directory, request(), &source, 7, inputs(1), now).unwrap();
    let mut bad = candidate();
    bad.evidence.clear();
    let mut changed = candidate();
    changed.evidence = "different".into();
    let results = persist_batch_files(
        &directory,
        request(),
        &source,
        7,
        vec![(0, changed), (1, bad), (3, candidate())],
        now,
    )
    .unwrap();
    assert!(matches!(
        &results[0],
        (0, Err(WriteError::Storage(StoreError::Conflict)))
    ));
    assert!(matches!(
        &results[1],
        (1, Err(WriteError::Candidate(RecordError::EmptyEvidence)))
    ));
    assert!(matches!(&results[2],(3,Ok(RecordOutcome::Accepted(v)))if v.sequence==2));
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"2"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn batch_stops_after_uncertain_io_and_preserves_prior_receipts_and_gap() {
    let (root, store, source, directory) = fixture().await;
    test_after_counter(|| native::FAIL.with(|n| n.set(Some(native::Fault::AfterCounter))));
    let result = persist_batch_files(&directory, request(), &source, 7, inputs(3), now).unwrap();
    assert!(matches!(&result[0],(0,Ok(RecordOutcome::Accepted(v)))if v.sequence==1));
    assert!(matches!(&result[1], (1, Err(WriteError::Storage(_)))));
    assert!(matches!(
        &result[2],
        (2, Err(WriteError::Storage(StoreError::Unavailable)))
    ));
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"2"
    );
    let lines = fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap();
    assert_eq!(lines.iter().filter(|b| **b == b'\n').count(), 1);
    store.close().await.unwrap();
}
#[tokio::test]
async fn batch_updates_own_pins_and_offsets_across_successive_rewrites() {
    let (root, store, source, directory) = fixture().await;
    let records: Vec<_> = (1..=1000)
        .map(|i| seed(&source, i, &format!("old-{i}"), i == 1))
        .collect();
    seed_file(root.path(), &records, 1000);
    let results = persist_batch_files(&directory, request(), &source, 7, inputs(3), now).unwrap();
    for (index, result) in results {
        assert!(
            matches!(result.unwrap(),RecordOutcome::Accepted(v)if v.sequence==1001+index as u64)
        );
    }
    let bytes = fs::read(root.path().join("workflow-observations/s.ndjson")).unwrap();
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
    assert_eq!(
        retained
            .iter()
            .find(|r| r.sequence() == 1)
            .unwrap()
            .origin(),
        records[0].origin()
    );
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1003"
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn admitted_batch_finishes_after_first_effect_crosses_original_deadline() {
    let (_root, store, source, directory) = fixture().await;
    let deadline = Instant::now() + Duration::from_millis(500);
    test_after_counter(move || {
        std::thread::sleep(
            deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(10),
        )
    });
    let results = persist_batch_files(
        &directory,
        ScanRequest::capture(deadline).unwrap(),
        &source,
        7,
        inputs(5),
        now,
    )
    .unwrap();
    assert!(Instant::now() > deadline);
    assert!(results
        .iter()
        .all(|(_, r)| matches!(r, Ok(RecordOutcome::Accepted(_)))));
    store.close().await.unwrap();
}
#[tokio::test]
async fn batch_rejects_oversized_and_duplicate_indices_before_effects() {
    let (root, store, source, directory) = fixture().await;
    assert!(matches!(
        persist_batch_files(&directory, request(), &source, 7, inputs(6), now),
        Err(WriteError::Storage(StoreError::TooLarge))
    ));
    assert!(matches!(
        persist_batch_files(
            &directory,
            request(),
            &source,
            7,
            vec![(0, candidate()), (0, candidate())],
            now
        ),
        Err(WriteError::Storage(StoreError::Invalid(_)))
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
async fn batch_rate_limit_includes_records_accepted_earlier_in_the_same_batch() {
    let (root, store, source, directory) = fixture().await;
    let n = MAX_ACCEPTED_PER_SESSION_MINUTE - 2;
    let records: Vec<_> = (1..=n)
        .map(|i| seed(&source, i as u64, &format!("old-{i}"), true))
        .collect();
    seed_file(root.path(), &records, n as u64);
    let results = persist_batch_files(&directory, request(), &source, 7, inputs(5), now).unwrap();
    for (index, result) in results {
        if index < 2 {
            assert!(matches!(result, Ok(RecordOutcome::Accepted(_))));
        } else {
            assert!(matches!(
                result,
                Err(WriteError::Candidate(RecordError::RateLimited))
            ));
        }
    }
    assert_eq!(
        fs::read_to_string(root.path().join("workflow-observations/sequence")).unwrap(),
        MAX_ACCEPTED_PER_SESSION_MINUTE.to_string()
    );
    store.close().await.unwrap();
}
#[tokio::test]
async fn successive_batch_rewrites_consume_one_aggregate_actual_read_budget() {
    let (root, store, source, directory) = fixture().await;
    let records: Vec<_> = (1..=1000)
        .map(|i| seed(&source, i, &format!("old-{i}"), i == 1))
        .collect();
    let target_bytes: usize = records.iter().map(|r| r.encode().unwrap().len() + 1).sum();
    // Enough remaining actual reads for two copies, fewer than three. Foreign
    // blank bytes count in inventory, but never become retained payloads.
    super::tests::fill_blank_inventory(
        root.path(),
        reader::MAX_TOTAL_BYTES - target_bytes - target_bytes * 5 / 2 - 4,
        false,
    );
    seed_file(root.path(), &records, 1000);
    let results = persist_batch_files(
        &directory,
        request().for_numeric_test(),
        &source,
        7,
        inputs(5),
        now,
    )
    .unwrap();
    for (index, result) in results {
        if index < 2 {
            assert!(
                matches!(result, Ok(RecordOutcome::Accepted(_))),
                "index {index}: {result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(WriteError::Storage(StoreError::TooLarge))),
                "index {index}: {result:?}"
            );
        }
    }
    assert_eq!(
        fs::read(root.path().join("workflow-observations/sequence")).unwrap(),
        b"1002"
    );
    store.close().await.unwrap();
}

#[tokio::test]
async fn aggregate_escaped_intent_counts_original_problem_areas_before_effects() {
    let (root, store, source, directory) = fixture().await;
    let mut candidate = candidate();
    candidate.problem_area = Some("\"".repeat(30 * 1024));
    let result = persist_batch_files(
        &directory,
        request(),
        &source,
        7,
        vec![(0, candidate.clone()), (1, candidate)],
        now,
    );
    assert!(
        matches!(result, Err(WriteError::Storage(StoreError::TooLarge))),
        "{result:?}"
    );
    assert!(!root.path().join("workflow-observations/sequence").exists());
    store.close().await.unwrap();
}
