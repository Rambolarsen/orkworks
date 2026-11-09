use super::*;
use serde_json::json;

fn record(sequence: u64, key: &str) -> RecordData {
    let mut value = super::super::tests::line();
    value["observation"]["id"] = json!(uuid::Uuid::from_u128(sequence as u128).to_string());
    value["observation"]["sequence"] = json!(sequence);
    value["idempotencyKeyHash"] = json!(hash_key("s", key));
    value["origin"]["observationSha256"] = json!(value_digest(
        &serde_json::from_value(value["observation"].clone()).unwrap()
    )
    .unwrap());
    decode_line(&serde_json::to_vec(&value).unwrap(), "s").unwrap()
}
fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-08T00:01:00Z")
        .unwrap()
        .with_timezone(&Utc)
}
#[test]
fn qualified_retry_returns_original_receipt_without_changing_either_record() {
    let original = record(1, "key");
    let incoming = record(99, "key");
    let before = (original.encode().unwrap(), incoming.encode().unwrap());
    let result = resolve_retry(std::slice::from_ref(&original), &incoming, now())
        .unwrap()
        .unwrap();
    assert_eq!(
        result,
        RecordOutcome::Duplicate {
            observation_id: original.identity().0.into(),
            sequence: 1,
            accepted_at: "2026-11-08T00:00:00Z".into(),
        }
    );
    assert_eq!(
        before,
        (original.encode().unwrap(), incoming.encode().unwrap())
    );
}
#[test]
fn eviction_copies_exact_origin_and_retry_does_not_recreate_value() {
    let original = record(1, "key");
    let before = original.encode().unwrap();
    let tombstone = evicted_retry("s", &original, now()).unwrap().unwrap();
    assert!(tombstone.observation().is_none());
    assert_eq!(tombstone.origin(), original.origin());
    assert_eq!(tombstone.identity(), original.identity());
    let incoming = record(2, "key");
    assert!(matches!(
        resolve_retry(std::slice::from_ref(&tombstone), &incoming, now()),
        Ok(Some(RecordOutcome::Duplicate { sequence: 1, .. }))
    ));
    assert_eq!(before, original.encode().unwrap());
    assert_eq!(
        decode_line(&tombstone.encode().unwrap(), "s").unwrap(),
        tombstone
    );
}

fn change(mut record: RecordData, apply: impl FnOnce(&mut ObservationData)) -> RecordData {
    let Record::Observation(v) = &mut record.0 else {
        panic!("qualified observation")
    };
    apply(v);
    v.origin.0.observation_sha256 = value_digest(&v.observation).unwrap();
    v.payload_hash = hash_payload(
        v.observation.kind,
        &v.observation.description,
        &v.observation.evidence,
        v.observation.reported_impact,
        v.observation.problem_area.as_deref(),
    );
    record.validate("s").unwrap();
    record
}
fn legacy(record: &RecordData) -> RecordData {
    match &record.0 {
        Record::Observation(v) => RecordData(Record::LegacyObservation(StoredObservation {
            observation: v.observation.clone(),
            idempotency_key_hash: v.idempotency_key_hash.clone(),
            payload_hash: v.payload_hash.clone(),
        })),
        _ => panic!("qualified observation"),
    }
}
#[test]
fn retries_refuse_every_scope_change_and_payload_change_for_values_and_tombstones() {
    let original = record(1, "key");
    let tomb = evicted_retry("s", &original, now()).unwrap().unwrap();
    for changed in [
        change(record(2, "key"), |v| {
            v.origin.0.database_epoch = uuid::Uuid::nil().to_string()
        }),
        change(record(2, "key"), |v| {
            v.origin.0.workspace_sha256 = "b".repeat(64)
        }),
        change(record(2, "key"), |v| {
            v.origin.0.created_at = "reused".into()
        }),
        change(record(2, "key"), |v| {
            v.observation.description = "different".into();
            v.observation.fingerprint = "v1:obstacle:different".into();
        }),
    ] {
        for retained in [&original, &tomb] {
            let before = retained.encode().unwrap();
            assert!(matches!(
                resolve_retry(std::slice::from_ref(retained), &changed, now()),
                Err(StoreError::Conflict)
            ));
            assert_eq!(before, retained.encode().unwrap());
        }
    }
}
#[test]
fn legacy_collisions_cannot_gain_qualification_and_noncolliding_legacy_is_preserved() {
    let original = record(1, "key");
    let legacy_value = legacy(&original);
    let legacy_tomb = evicted_retry("s", &legacy_value, now()).unwrap().unwrap();
    assert!(legacy_tomb.origin().is_none());
    for retained in [legacy_value, legacy_tomb] {
        let before = retained.encode().unwrap();
        assert!(matches!(
            resolve_retry(std::slice::from_ref(&retained), &record(2, "key"), now()),
            Err(StoreError::Conflict)
        ));
        assert!(resolve_retry(
            std::slice::from_ref(&retained),
            &record(2, "other-key"),
            now()
        )
        .unwrap()
        .is_none());
        assert_eq!(before, retained.encode().unwrap());
    }
}
#[test]
fn retry_window_is_inclusive_and_expired_evictions_are_not_reintroduced() {
    let original = record(1, "key");
    let tomb = evicted_retry("s", &original, now()).unwrap().unwrap();
    let accepted = parse_time("2026-11-08T00:00:00Z").unwrap();
    let boundary = accepted + chrono::Duration::seconds(IDEMPOTENCY_WINDOW_SECS);
    for retained in [&original, &tomb] {
        assert!(
            resolve_retry(std::slice::from_ref(retained), &record(2, "key"), boundary)
                .unwrap()
                .is_some()
        );
        assert!(resolve_retry(
            std::slice::from_ref(retained),
            &record(2, "key"),
            boundary + chrono::Duration::nanoseconds(1)
        )
        .unwrap()
        .is_none());
        assert!(evicted_retry("s", retained, boundary).unwrap().is_some());
        assert!(
            evicted_retry("s", retained, boundary + chrono::Duration::nanoseconds(1))
                .unwrap()
                .is_none()
        );
    }
}
#[test]
fn confidence_keeps_existing_retry_semantics_but_the_evicted_value_digest_stays_original() {
    let original = record(1, "key");
    let changed = change(record(2, "key"), |v| v.observation.confidence = 0.7);
    assert_ne!(original.origin(), changed.origin());
    assert!(
        resolve_retry(std::slice::from_ref(&original), &changed, now())
            .unwrap()
            .is_some()
    );
    assert_eq!(
        evicted_retry("s", &original, now())
            .unwrap()
            .unwrap()
            .origin(),
        original.origin()
    );
}
#[test]
fn ambiguous_retry_identity_is_corrupt_even_when_a_matching_receipt_is_found_first() {
    let original = record(1, "key");
    let second = record(2, "key");
    assert!(matches!(
        resolve_retry(&[original.clone(), second], &record(3, "key"), now()),
        Err(StoreError::Corrupt)
    ));
    assert!(matches!(
        resolve_retry(
            &[original.clone(), legacy(&original)],
            &record(3, "key"),
            now()
        ),
        Err(StoreError::Conflict)
    ));
}
#[test]
fn unknown_incoming_origin_and_malformed_values_cannot_be_planned() {
    let original = record(1, "key");
    assert!(matches!(
        resolve_retry(&[], &legacy(&original), now()),
        Err(StoreError::Conflict)
    ));
    let mut invalid = original.clone();
    let Record::Observation(v) = &mut invalid.0 else {
        unreachable!()
    };
    v.origin.0.observation_sha256 = "0".repeat(64);
    assert!(matches!(
        resolve_retry(&[], &invalid, now()),
        Err(StoreError::Corrupt)
    ));
    assert!(matches!(
        evicted_retry("s", &invalid, now()),
        Err(StoreError::Corrupt)
    ));
}

#[test]
fn complete_segment_counts_are_bounded_before_a_retry_receipt() {
    for tombstones in [false, true] {
        let cap = if tombstones {
            MAX_TOMBSTONES
        } else {
            MAX_SEGMENT_OBSERVATIONS
        };
        let mut records: Vec<_> = (1..=cap)
            .map(|i| {
                let v = record(i as u64, &format!("key-{i}"));
                if tombstones {
                    evicted_retry("s", &v, now()).unwrap().unwrap()
                } else {
                    v
                }
            })
            .collect();
        let incoming = record(9999, "key-1");
        assert!(resolve_retry(&records, &incoming, now()).unwrap().is_some());
        records.push(if tombstones {
            evicted_retry("s", &record(9998, "extra"), now())
                .unwrap()
                .unwrap()
        } else {
            record(9998, "extra")
        });
        assert!(matches!(
            resolve_retry(&records, &incoming, now()),
            Err(StoreError::TooLarge)
        ));
    }
}
#[test]
fn complete_segment_byte_count_includes_origin_tag_and_newline_at_exact_limit() {
    let mut records: Vec<_> = (1..=MAX_SEGMENT_OBSERVATIONS)
        .map(|i| record(i as u64, &format!("key-{i}")))
        .collect();
    let base: usize = records.iter().map(|r| r.encode().unwrap().len() + 1).sum();
    let mut remaining = MAX_SEGMENT_BYTES as usize - base;
    for v in &mut records {
        let extra = remaining.min(MAX_EVIDENCE_CHARS - "error".len());
        if extra == 0 {
            break;
        }
        *v = change(v.clone(), |r| {
            r.observation.evidence.push_str(&"e".repeat(extra))
        });
        remaining -= extra;
    }
    assert_eq!(remaining, 0);
    assert_eq!(
        records
            .iter()
            .map(|r| r.encode().unwrap().len() + 1)
            .sum::<usize>(),
        MAX_SEGMENT_BYTES as usize
    );
    // Padding changes the payload, so retry that exact padded value.
    let incoming = change(records[0].clone(), |v| {
        v.observation.id = uuid::Uuid::from_u128(9999).to_string();
        v.observation.sequence = 9999;
    });
    assert!(resolve_retry(&records, &incoming, now()).unwrap().is_some());
    let last = records.last_mut().unwrap();
    *last = change(last.clone(), |r| r.observation.evidence.push('e'));
    assert!(matches!(
        resolve_retry(&records, &incoming, now()),
        Err(StoreError::TooLarge)
    ));
}
#[test]
fn eviction_of_existing_tombstones_preserves_their_exact_encoded_representation() {
    for original in [record(1, "key"), legacy(&record(2, "other"))] {
        let tomb = evicted_retry("s", &original, now()).unwrap().unwrap();
        let retained = evicted_retry("s", &tomb, now()).unwrap().unwrap();
        assert_eq!(tomb.encode().unwrap(), retained.encode().unwrap());
    }
}
