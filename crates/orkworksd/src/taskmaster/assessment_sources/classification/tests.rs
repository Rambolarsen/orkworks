use super::*;
use serde_json::json;
const EPOCH: &str = "12345678-1234-4234-8234-123456789abc";
const HASH: &str = "c52ddf65534b7b46035084358ab7902be4bfef220bdb503ac7039cc861905b05";
fn scope() -> Scope {
    Scope {
        epoch: EPOCH.into(),
        workspace: HASH.into(),
    }
}
fn known(value: Value, pairs: &[(&str, &str)]) -> Input {
    Input {
        value,
        origin: Origin::Qualified(Qualified {
            scope: scope(),
            sources: pairs
                .iter()
                .map(|(id, created)| SourceCreation {
                    session_id: (*id).into(),
                    created_at: (*created).into(),
                })
                .collect(),
        }),
    }
}
fn unknown(value: Value) -> Input {
    Input {
        value,
        origin: Origin::Unknown,
    }
}
fn set(
    observations: Vec<Input>,
    recommendations: Vec<Input>,
    readiness: Option<Input>,
) -> CapturedInputSet {
    CapturedInputSet {
        scope: SelectedScope::Qualified(scope()),
        observations,
        recommendations,
        readiness,
    }
}
fn envelope(observations: Value, recommendations: Value, readiness: Value) -> Value {
    json!({"schemaVersion":1,"databaseEpoch":EPOCH,"workspaceSha256":HASH,"observations":observations,"recommendations":recommendations,"readinessFocus":readiness})
}
#[test]
fn unknown_in_each_complete_input_position_cannot_be_filtered_or_called_source_free() {
    for position in 0..3 {
        // Well-formed source data is still an Unknown input, not a parse failure.
        let value = json!({"value":{"id":"same"},"sources":[]});
        let a = if position == 0 {
            unknown(value.clone())
        } else {
            known(value.clone(), &[])
        };
        let b = if position == 1 {
            unknown(value.clone())
        } else {
            known(value.clone(), &[])
        };
        let c = if position == 2 {
            unknown(value)
        } else {
            known(value, &[])
        };
        assert_eq!(
            set(vec![a], vec![b], Some(c)).classify("assessment-1"),
            Err(ClassificationError::Unqualified)
        );
    }
    let value = json!({"value":{"id":"same"},"sources":[]});
    let qualified = set(
        vec![known(value.clone(), &[])],
        vec![known(value.clone(), &[])],
        Some(known(value, &[])),
    )
    .classify("assessment-1")
    .unwrap();
    let absent = set(vec![], vec![], None).classify("assessment-1").unwrap();
    assert_ne!(qualified.input_digest(), absent.input_digest());
    set(
        vec![known(json!({"value":{"id":"same"},"sources":[]}), &[])],
        vec![],
        None,
    )
    .classify("assessment-1")
    .unwrap();
}
#[test]
fn empty_set_requires_qualified_scope_and_serialized_data_cannot_supply_a_claim() {
    let mut empty = set(vec![], vec![], None);
    empty.scope = SelectedScope::Unknown;
    assert_eq!(
        empty.classify("assessment-1"),
        Err(ClassificationError::Unqualified)
    );
    let packet = set(vec![], vec![], None).classify("assessment-1").unwrap();
    assert_eq!(
        packet.input_digest(),
        "1297869707c0641d0165eeab56a4d3b5b39b985806f796c3374cadb2195a9148"
    );
    assert_eq!(
        set(
            vec![unknown(serde_json::to_value(packet).unwrap())],
            vec![],
            None
        )
        .classify("assessment-1"),
        Err(ClassificationError::Unqualified)
    );
}
#[test]
fn exact_values_order_and_source_union_match_the_existing_codec() {
    let make = || {
        set(
            vec![
                known(json!({"id":"one","nested":{"z":2,"a":1}}), &[("a", "old")]),
                known(json!({"id":"two"}), &[("a", "old"), ("b", "created")]),
            ],
            vec![known(json!({"id":"rec"}), &[])],
            Some(known(json!({"snapshotId":"focus"}), &[("b", "created")])),
        )
    };
    let wire = envelope(
        json!([{"value":{"id":"one","nested":{"z":2,"a":1}},"sources":[{"sessionId":"a","createdAt":"old"}]},{"value":{"id":"two"},"sources":[{"sessionId":"a","createdAt":"old"},{"sessionId":"b","createdAt":"created"}]}]),
        json!([{"value":{"id":"rec"},"sources":[]}]),
        json!({"value":{"snapshotId":"focus"},"sources":[{"sessionId":"b","createdAt":"created"}]}),
    );
    let expected = super::super::OriginalInputs::fixture("assessment-1", wire)
        .unwrap()
        .into_sources();
    let packet = make().classify("assessment-1").unwrap();
    assert_eq!(packet, expected);
    assert_eq!(
        serde_json::to_value(packet.sources()).unwrap(),
        json!([{"sessionId":"a","createdAt":"old"},{"sessionId":"b","createdAt":"created"}])
    );
    let mut reversed = make();
    reversed.observations.reverse();
    assert_ne!(
        reversed.classify("assessment-1").unwrap().input_digest(),
        packet.input_digest()
    );
    let mut changed = make();
    changed.observations[0].value["nested"]["z"] = json!(3);
    assert_ne!(
        changed.classify("assessment-1").unwrap().input_digest(),
        packet.input_digest()
    );
    let mut changed_source = make();
    for input in &mut changed_source.observations {
        if let Origin::Qualified(claim) = &mut input.origin {
            for source in &mut claim.sources {
                if source.session_id == "a" {
                    source.created_at = "different original creation".into();
                }
            }
        }
    }
    assert_ne!(
        changed_source
            .classify("assessment-1")
            .unwrap()
            .input_digest(),
        packet.input_digest()
    );
}
#[test]
fn foreign_scope_conflicting_creations_and_noncanonical_pairs_refuse() {
    for part in ["epoch", "workspace"] {
        let mut input = known(json!(1), &[]);
        if let Origin::Qualified(ref mut q) = input.origin {
            if part == "epoch" {
                q.scope.epoch = "00000000-0000-4000-8000-000000000001".into()
            } else {
                q.scope.workspace = "a".repeat(64)
            }
        }
        assert_eq!(
            set(vec![input], vec![], None).classify("assessment-1"),
            Err(ClassificationError::Conflict)
        );
    }
    assert_eq!(
        set(
            vec![known(json!(1), &[("s", "old")])],
            vec![known(json!(2), &[("s", "new")])],
            None
        )
        .classify("assessment-1"),
        Err(ClassificationError::Conflict)
    );
    for pairs in [
        vec![("b", "c"), ("a", "c")],
        vec![("a", "c"), ("a", "c")],
        vec![("bad/id", "c")],
        vec![("s", "")],
    ] {
        assert!(matches!(
            set(vec![known(json!(1), &pairs)], vec![], None).classify("assessment-1"),
            Err(ClassificationError::Invalid(_))
        ));
    }
    assert!(matches!(
        set(vec![], vec![], None).classify("bad/id"),
        Err(ClassificationError::Invalid(_))
    ));
    let mut bad = set(vec![], vec![], None);
    if let SelectedScope::Qualified(ref mut s) = bad.scope {
        s.epoch = EPOCH.to_uppercase()
    };
    assert!(matches!(
        bad.classify("assessment-1"),
        Err(ClassificationError::Invalid(_))
    ));
}
#[test]
fn aggregate_source_limit_accepts_128_and_refuses_the_129th_across_inputs() {
    let ids: Vec<_> = (0..128).map(|i| format!("s{i:03}")).collect();
    let pairs: Vec<_> = ids.iter().map(|id| (id.as_str(), "c")).collect();
    assert_eq!(
        set(vec![known(json!(1), &pairs)], vec![], None)
            .classify("assessment-1")
            .unwrap()
            .sources()
            .len(),
        128
    );
    assert_eq!(
        set(
            vec![known(json!(1), &pairs)],
            vec![known(json!(2), &[("z", "c")])],
            None
        )
        .classify("assessment-1"),
        Err(ClassificationError::TooLarge)
    );
}
#[test]
fn complete_source_packet_limit_accepts_exact_16_kib_and_refuses_one_extra_byte() {
    let mut pairs: Vec<(String, String)> =
        (0..128).map(|i| (format!("s{i:03}"), "c".into())).collect();
    let wire = |pairs: &[(String, String)]| json!({"schemaVersion":1,"databaseEpoch":EPOCH,"workspaceSha256":HASH,"assessmentId":"assessment-1","inputDependenciesSha256":"a".repeat(64),"sources":pairs.iter().map(|(id,c)|json!({"sessionId":id,"createdAt":c})).collect::<Vec<_>>()});
    let mut remaining = 16384 - serde_json::to_vec(&wire(&pairs)).unwrap().len();
    for (_, created) in &mut pairs {
        let add = remaining.min(255);
        created.push_str(&"x".repeat(add));
        remaining -= add;
    }
    assert_eq!(remaining, 0);
    assert_eq!(serde_json::to_vec(&wire(&pairs)).unwrap().len(), 16384);
    let run = |pairs: &[(String, String)]| {
        let refs: Vec<_> = pairs
            .iter()
            .map(|(id, c)| (id.as_str(), c.as_str()))
            .collect();
        set(vec![known(json!("value"), &refs)], vec![], None).classify("assessment-1")
    };
    assert_eq!(
        serde_json::to_vec(&run(&pairs).unwrap()).unwrap().len(),
        16384
    );
    pairs
        .iter_mut()
        .find(|(_, c)| c.len() < 256)
        .unwrap()
        .1
        .push('x');
    assert_eq!(run(&pairs), Err(ClassificationError::TooLarge));
}
#[test]
fn full_envelope_limit_counts_structure_escaping_and_late_members_without_trimming() {
    let wire = |value: Value| {
        envelope(
            json!([{"value":value,"sources":[]}]),
            json!([]),
            Value::Null,
        )
    };
    let overhead = serde_json::to_vec(&wire(json!(""))).unwrap().len();
    let payload = "x".repeat(1048576 - overhead);
    assert_eq!(
        serde_json::to_vec(&wire(json!(payload))).unwrap().len(),
        1048576
    );
    set(vec![known(json!(payload), &[])], vec![], None)
        .classify("assessment-1")
        .unwrap();
    assert_eq!(
        set(vec![known(json!(format!("{payload}x")), &[])], vec![], None).classify("assessment-1"),
        Err(ClassificationError::TooLarge)
    );
    let escaped = "\n".repeat((1048576 - overhead) / 2 + 1);
    assert!(serde_json::to_vec(&wire(json!(escaped))).unwrap().len() > 1048576);
    assert_eq!(
        set(vec![known(json!(escaped), &[])], vec![], None).classify("assessment-1"),
        Err(ClassificationError::TooLarge)
    );
    assert_eq!(
        set(
            vec![known(json!(payload), &[])],
            vec![known(json!("late"), &[])],
            None
        )
        .classify("assessment-1"),
        Err(ClassificationError::TooLarge)
    );
}
