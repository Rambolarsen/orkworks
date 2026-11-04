//! Acceptance tests for the dormant hierarchy storage kernel. Golden inputs
//! are the merged child-source component vectors; every rejection case mirrors
//! the owning contracts. Wired only here — the module has no production caller.

use super::connection::Connection;
use serde_json::{json, Value};
use tempfile::tempdir;

/// Golden fixture of record: merged child-source component vectors.
const VECTORS: &str = include_str!("../../../../../docs/validation/child-source-vectors.json");

fn fixture() -> &'static Value {
    use std::sync::OnceLock;
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| serde_json::from_str(VECTORS).expect("fixture parses"))
}

/// Canonical bytes of one pinned vector, e.g. (`"worker-plan"`).
fn golden(id: &str) -> Vec<u8> {
    let vectors = fixture()["vectors"].as_array().expect("vectors array");
    let chosen = vectors
        .iter()
        .find(|v| v["id"] == id)
        .unwrap_or_else(|| panic!("vector {id}"));
    let canonical = chosen["canonical"].as_str().expect("canonical bytes");
    assert_eq!(canonical.len(), chosen["bytes"].as_u64().unwrap() as usize);
    canonical.as_bytes().to_vec()
}

#[test]
fn complete_catalog_column_shape_verifies() {
    let root = tempdir().unwrap();
    let path = root.path().join("catalog.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None);
    drop(store);
    super::schema::verify_catalog_shape(&path).expect("fresh catalog shape is complete");
}

#[test]
fn foreign_column_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("snooping.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("ALTER TABLE objects ADD COLUMN snoop TEXT;")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("catalog"),
        "unknown column fences rather than repairs"
    );
}

// The following three cases pin scope-1 refusal behavior that shipped in
// Task 1; they are acceptance pins against regression, not new behavior.

#[test]
fn foreign_page_size_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("foreign_page.sqlite");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.pragma_update(None, "page_size", 512).unwrap();
        raw.execute_batch("CREATE TABLE foreign_store (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("page size"),
        "non-4096-byte page configuration is unrepairable state"
    );
}

#[test]
fn foreign_wal_mode_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("foreign_wal.sqlite");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.pragma_update(None, "journal_mode", "WAL").unwrap();
        raw.execute_batch("CREATE TABLE foreign_store (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("journal"),
        "the accepted store never runs in WAL mode, and never migrates one"
    );
}

#[test]
fn unknown_schema_version_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("unknown_version.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("UPDATE store_header SET schema_version = 5;")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("schema version"),
        "unknown format versions fence the store; nothing is migrated"
    );
}

// Metadata rows: source-record admission plus the complete 16 KiB wrapper.

// Row admission (scope 3): the nine golden row kinds from the packet.
// ---------------------------------------------------------------------------

use super::rows::{admit_meta_record, check_metadata_row, MetaKind};

#[test]
fn golden_metadata_records_admit_with_reconstructed_keys() {
    let approval = admit_meta_record(MetaKind::PlanApproval, &golden("worker-approval")).unwrap();
    assert_eq!(approval.key["approvalId"], "worker-approval");
    assert_eq!(
        approval.digest, "49159f1dc4b63168f2440b43ba890f9c746de7455681b1817b456b07bb1f21a8",
        "approval source digest is the child-source-plan-approval domain"
    );
    let reservation =
        admit_meta_record(MetaKind::Reservation, &golden("worker-reservation")).unwrap();
    assert_eq!(reservation.key["taskId"], "worker-task");
    let allocation = admit_meta_record(MetaKind::Allocation, &golden("worker-allocation")).unwrap();
    assert_eq!(
        allocation.key["reservationId"], "worker-reservation",
        "the allocation row keys by the exact ReservationRef"
    );
    let attachment = admit_meta_record(MetaKind::Attachment, &golden("worker-attachment")).unwrap();
    assert_eq!(
        attachment.key,
        json!("worker-session"),
        "attachment key is the reserved childSessionId"
    );
}

/// The packet's exact metadata wrapper rows: approved source + receipt, then
/// the reservation/allocation/attachment records with the frozen commit.
#[test]
fn golden_metadata_wrapper_rows_verify_and_fit_the_cap() {
    let approval = admit_meta_record(MetaKind::PlanApproval, &golden("worker-approval")).unwrap();
    let receipt: Value = serde_json::from_slice(&golden("worker-receipt")).unwrap();
    let mut approval_value = approval.value_base.clone();
    approval_value["digest"] = json!(approval.digest);
    approval_value["receipt"] = receipt.clone();
    approval_value["lastCommit"] = receipt["commit"].clone();
    let row = check_metadata_row(MetaKind::PlanApproval, &approval.key, &approval_value).unwrap();
    assert_eq!(row.table, "child_plan_approvals");
    for kind in [
        MetaKind::Reservation,
        MetaKind::Allocation,
        MetaKind::Attachment,
    ] {
        let record = match kind {
            MetaKind::Reservation => "worker-reservation",
            MetaKind::Allocation => "worker-allocation",
            _ => "worker-attachment",
        };
        let admitted = admit_meta_record(kind, &golden(record)).unwrap();
        let mut value = admitted.value_base.clone();
        value["lastCommit"] = receipt["commit"].clone();
        check_metadata_row(kind, &admitted.key, &value).unwrap();
    }
}

/// The metadata intake cap refuses over-16 KiB records before any shape
/// check; the four accepted golden wrappers each measure under the cap.
#[test]
fn corrupted_reserved_field_refuses_the_delta() {
    let root = tempdir().unwrap();
    let path = root.path().join("reserved.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    drop(store);
    // Corrupt one quota row's reserved field directly; the primitive path
    // must refuse before any reload.
    let raw = rusqlite::Connection::open(&path).unwrap();
    let value: Vec<u8> = raw
        .query_row("SELECT value FROM quota_usage LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut corrupted: Value = serde_json::from_slice(&value).unwrap();
    corrupted["reserved"] = json!("64");
    raw.execute(
        "UPDATE quota_usage SET value = ?1",
        rusqlite::params![super::canonical(&corrupted).unwrap()],
    )
    .unwrap();
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        quota::apply_delta(&store, &run_owner(), 4096)
            .err()
            .map(|e| e.0),
        Some("unavailable"),
        "ambiguous rows fence reload, and fenced stores refuse writes"
    );
}

#[test]
fn large_valid_source_body_over_16kib_is_an_acceptance_case() {
    // The 16 KiB wrapper cap applies to child metadata rows only; a valid
    // >16 KiB child configuration body (from its 32 KiB retained rule
    // content) is admitted and journaled, never rejected at the object row.
    let bytes = modified("worker-configuration", |value| {
        let content = format!(
            "{}{}",
            "Synthetic retained rule content. ",
            "x".repeat(32 * 1024)
        );
        let rule_id = value["rules"][0]["id"].clone();
        let rule = &mut value["rules"][0];
        rule["content"] = json!(content);
        rule["contentDigest"] = json!(super::sha256_hex(content.as_bytes()));
        // The requirement manifest binds that named source digest.
        for manifest in value["requirementManifests"].as_array_mut().unwrap() {
            if manifest["sourceId"] == rule_id {
                manifest["sourceDigest"] = json!(super::sha256_hex(content.as_bytes()));
            }
        }
        // The framing contract requires the rendered byte payload to track
        // the mutated content; the test mirrors the owning codec.
        let re_rendered = super::rows::render_child_instructions(value).unwrap();
        value["renderedInstructions"] = json!(re_rendered);
        value["renderedInstructionsDigest"] = json!(super::sha256_hex(
            value["renderedInstructions"].as_str().unwrap().as_bytes()
        ));
        let mut body = value.clone();
        body.as_object_mut()
            .expect("configuration object")
            .remove("configurationDigest");
        let resealed = super::domain_digest(
            "orkworks.assignment-configuration.v3",
            &super::canonical(&body).unwrap(),
        );
        value["configurationDigest"] = json!(resealed);
    });
    assert!(
        bytes.len() > 16 * 1024,
        "the acceptance case is above 16 KiB"
    );
    let root = tempdir().unwrap();
    let path = root.path().join("large_configuration.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(&store, ObjectKind::Configuration, &source_context(), &bytes)
        .unwrap();
    drop(store);
    assert_eq!(
        Connection::open(&path, "workspace-1")
            .unwrap()
            .unavailable_reason(),
        None,
        "a valid >16 KiB source body is not a rejection"
    );
}

#[test]
fn deleted_change_entry_row_fences_reopen() {
    // Reload sweep: every change entry across the chain must still have its
    // stored row; deleting an established quota row behind the journal fences.
    let root = tempdir().unwrap();
    let path = root.path().join("deleted_row.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    drop(store);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("DELETE FROM quota_usage;").unwrap();
    }
    let mutated = store_bytes(&path);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("rows"),
        "a missing committed change row cannot be repaired"
    );
    assert_eq!(store_bytes(&path), mutated, "bytes stay untouched");
}

#[test]
fn review_snapshot_binds_the_retained_result() {
    // The snapshot's embedded result is validated: identity, revision and
    // the source-domain digest bind it to the target.
    let bytes = modified("review-snapshot", |value| {
        value["result"]["resultRevision"] = json!(2);
    });
    let rejection = rows::admit_source_object(ObjectKind::ReviewInput, &source_context(), &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(
        rejection,
        Some("result revision binding"),
        "a result that does not bind to the target must refuse"
    );
}

#[test]
fn approval_request_digest_reconstruction_binds_the_record() {
    let bytes = modified("worker-approval", |value| {
        value["requestDigest"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
    });
    let rejection = rows::admit_meta_record(MetaKind::PlanApproval, &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(
        rejection,
        Some("approval request digest"),
        "the request digest must reconstruct from the record's own joins"
    );
}

#[test]
fn manifest_positions_follow_source_id_order() {
    // The accepted checker sorts rules + role template by source ID before
    // matching requirement manifests positionally. Rename the rule to sort
    // AFTER the role ("worker-role" < "zz-late-rule") and place the manifests
    // in ID-sorted order (role first): valid config, accepted although the
    // rule's own precedence is 0 and the rule is not the first source.
    // Fragments of the rename appear in all three joined places; every other
    // byte is the accepted fixture.
    let bytes = modified("worker-configuration", |value| {
        let renamed_rule_id = json!("zz-late-rule");
        let role_manifest = value["requirementManifests"][1].clone();
        value["rules"][0]["id"] = renamed_rule_id.clone();
        value["requirementManifests"][0]["sourceId"] = renamed_rule_id.clone();
        value["requirementManifests"] = json!([role_manifest, value["requirementManifests"][0]]);
        // The framing contract tracks the renamed block identity; the test
        // mirrors the owning codec, reseals, and re-digests.
        let re_rendered = super::rows::render_child_instructions(value).unwrap();
        value["renderedInstructions"] = json!(re_rendered);
        value["renderedInstructionsDigest"] = json!(super::sha256_hex(
            value["renderedInstructions"].as_str().unwrap().as_bytes()
        ));
        let mut body = value.clone();
        body.as_object_mut()
            .expect("configuration object")
            .remove("configurationDigest");
        let resealed = super::domain_digest(
            "orkworks.assignment-configuration.v3",
            &super::canonical(&body).unwrap(),
        );
        value["configurationDigest"] = json!(resealed);
    });
    let rejection = rows::admit_source_object(ObjectKind::Configuration, &source_context(), &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(
        rejection, None,
        "manifest positions follow the ID-sorted source order, not rule precedence"
    );
}

#[test]
fn oversized_metadata_intake_is_rejected() {
    let filler = vec![b'x'; 16 * 1024 + 1];
    let rejection = admit_meta_record(MetaKind::Attachment, &filler)
        .err()
        .map(|e| e.0);
    assert_eq!(rejection, Some("bytes"), "over-cap intake refuses first");
}

// Reload/recovery (scope 5): reopen revalidates; corruption fences.

fn store_bytes(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).expect("store bytes")
}

#[test]
fn reopen_after_full_chain_verifies_and_continues() {
    let root = tempdir().unwrap();
    let path = root.path().join("chain.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    publication::journal_object(
        &store,
        ObjectKind::Configuration,
        &source_context(),
        &golden("worker-configuration"),
    )
    .unwrap();
    let consumer_ref = {
        let admitted = super::rows::admit_source_object(
            ObjectKind::Configuration,
            &source_context(),
            &golden("worker-configuration"),
        )
        .unwrap();
        json!({"digest": admitted.digest, "key": admitted.key})
    };
    let target_ref = {
        // The evidence target must itself be retained before the edge can
        // be journaled (broken joins refuse).
        let admitted = super::rows::admit_source_object(
            ObjectKind::Evidence,
            &source_context(),
            &golden("worker-evidence-0"),
        )
        .unwrap();
        publication::journal_object(
            &store,
            ObjectKind::Evidence,
            &source_context(),
            &golden("worker-evidence-0"),
        )
        .unwrap();
        json!({"digest": admitted.digest, "key": admitted.key})
    };
    publication::journal_dependency(&store, &consumer_ref, &target_ref).unwrap();
    drop(store);
    let reopened = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(reopened.unavailable_reason(), None);
    assert_eq!(reopened.workspace_id(), "workspace-1");
    assert!(!reopened.epoch().is_empty());
}

#[test]
fn deleted_commit_proof_opens_unavailable_and_leaves_bytes() {
    let root = tempdir().unwrap();
    let path = root.path().join("missing_proof.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    drop(store);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("DELETE FROM commit_proof WHERE sequence = 2;")
            .unwrap();
    }
    let before = store_bytes(&path);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("chain"),
        "a missing proof cannot be repaired or re-derived"
    );
    assert_eq!(store_bytes(&path), before, "recovery never rewrites bytes");
}

#[test]
fn mutated_proof_body_opens_unavailable_and_leaves_bytes() {
    let root = tempdir().unwrap();
    let path = root.path().join("mutated_manifest.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    drop(store);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        let body: Vec<u8> = raw
            .query_row(
                "SELECT body FROM commit_proof WHERE sequence = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut mutated = body.clone();
        let last = mutated.last_mut().expect("non-empty manifest body");
        *last = if *last == b'\n' { b'x' } else { b'\n' };
        raw.execute(
            "UPDATE commit_proof SET body = ?1 WHERE sequence = 2",
            rusqlite::params![mutated],
        )
        .unwrap();
    }
    let before = store_bytes(&path);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("chain"),
        "a mutated manifest body fails its digest"
    );
    assert_eq!(store_bytes(&path), before, "bytes stay untouched");
}

#[test]
fn mutated_row_body_opens_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("mutated_row.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    drop(store);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        let body: Vec<u8> = raw
            .query_row("SELECT value FROM quota_usage", [], |row| row.get(0))
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        let mut corrupted = value.clone();
        corrupted["used"] = json!("4096");
        raw.execute(
            "UPDATE quota_usage SET value = ?1",
            rusqlite::params![super::canonical(&corrupted).unwrap()],
        )
        .unwrap();
    }
    let before = store_bytes(&path);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("rows"),
        "a stored row body must match its committed row digest"
    );
    assert_eq!(store_bytes(&path), before, "bytes stay untouched");
}

#[test]
fn mismatched_epoch_opens_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("epoch.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    drop(store);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "UPDATE store_header SET storage_epoch = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff';",
        )
        .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), Some("epoch"));
}

/// Canonical bytes of a modified golden record.
fn modified(record: &str, mutate: fn(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(&golden(record)).unwrap();
    mutate(&mut value);
    super::canonical(&value).expect("canonical modification")
}

#[test]
fn extra_field_rejects_plan_admission() {
    let bytes = {
        let mut value: Value = serde_json::from_slice(&golden("worker-plan")).unwrap();
        value["extra"] = json!(1);
        super::canonical(&value).unwrap()
    };
    let rejection = rows::admit_source_object(ObjectKind::Plan, &source_context(), &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(rejection, Some("closed fields"));
}

#[test]
fn duplicate_object_fields_reject_decode() {
    let bytes = b"{\"a\":1,\"a\":1}";
    let rejection = rows::admit_source_object(ObjectKind::Plan, &source_context(), bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(
        rejection,
        Some("canonical wire"),
        "duplicate keys are noncanonical stored bytes"
    );
}

#[test]
fn wrong_kind_root_substitution_rejects() {
    let bytes = modified("worker-source", |value| {
        value["assignmentIdentity"]["kind"] = json!("root");
    });
    let rejection =
        rows::admit_source_object(ObjectKind::AssignmentSource, &source_context(), &bytes)
            .err()
            .map(|e| e.0);
    assert_eq!(rejection, Some("child identity"));
}

#[test]
fn invalid_counter_text_rejects() {
    for bad in ["00", "+1", "1.0", "1e0", " 1", "18446744073709551616"] {
        let rejection = rows::counter(&json!(bad), true).err().map(|e| e.0);
        assert_eq!(rejection, Some("counter"), "{bad} is not a Counter");
    }
    let top = json!("18446744073709551615");
    assert_eq!(rows::counter(&top, true).unwrap(), u64::MAX);
    assert_eq!(rows::counter(&json!("0"), false).unwrap(), 0);
    assert_eq!(
        rows::counter(&json!("0"), true).err().map(|e| e.0),
        Some("counter")
    );
}

#[test]
fn counter_must_be_text_not_number() {
    let rejection = rows::counter(&json!(1_u64), true).err().map(|e| e.0);
    assert_eq!(rejection, Some("counter"), "storage Counters are TEXT");
}

#[test]
fn digest_mismatch_rejects_sealed_plan() {
    let bytes = modified("worker-plan", |value| {
        value["scopeText"] = json!("Changed without re-signing.");
    });
    let rejection = rows::admit_source_object(ObjectKind::Plan, &source_context(), &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(rejection, Some("sealed digest"));
}

#[test]
fn broken_evidence_content_join_rejects() {
    let bytes = modified("worker-evidence-0", |value| {
        let content_digest = value["contentDigest"].as_str().unwrap().to_string();
        let flipped = format!(
            "{}{}",
            if content_digest.starts_with('a') {
                'b'
            } else {
                'a'
            },
            &content_digest[1..]
        );
        value["contentDigest"] = json!(flipped);
    });
    let rejection = rows::admit_source_object(ObjectKind::Evidence, &source_context(), &bytes)
        .err()
        .map(|e| e.0);
    assert_eq!(rejection, Some("evidence content digest"));
}

#[test]
fn broken_source_plan_join_rejects() {
    let bytes = modified("worker-source", |value| {
        value["plan"]["runId"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-1");
    });
    let rejection =
        rows::admit_source_object(ObjectKind::AssignmentSource, &source_context(), &bytes)
            .err()
            .map(|e| e.0);
    assert_eq!(rejection, Some("source plan projection"));
}

#[test]
fn noncanonical_stored_bytes_reject() {
    let canonical_bytes = golden("worker-plan");
    let mut spaced = b"{ \"schemaVersion\"".to_vec();
    spaced.extend_from_slice(&canonical_bytes["{\"schemaVersion\"".len()..]);
    let rejection = rows::admit_source_object(ObjectKind::Plan, &source_context(), &spaced)
        .err()
        .map(|e| e.0);
    assert_eq!(rejection, Some("canonical wire"));
}

// Quota (scope 6): child-source-retained-bytes counters.

use super::quota;

// Publication primitives (scope 4): one transaction per command.

use super::publication;

#[test]
fn golden_worker_source_chain_journals_row_by_row() {
    let root = tempdir().unwrap();
    let path = root.path().join("journals.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    // The plan first: the run identity comes from the plan binding itself.
    let plan = publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    // Establishes land as commands first; the plan's row journal carries its
    // two quota charges in the same transaction (init, 2 establishes, row).
    assert_eq!(plan.commit().sequence, 4);
    // The same body journals as an exact retry with no new commit.
    let retry = publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    assert_eq!(
        retry.commit(),
        plan.commit(),
        "exact retries leave links unchanged"
    );
    // The configuration joins the frozen run id through the same context.
    publication::journal_object(
        &store,
        ObjectKind::Configuration,
        &source_context(),
        &golden("worker-configuration"),
    )
    .unwrap();
    publication::journal_object(
        &store,
        ObjectKind::AssignmentSource,
        &source_context(),
        &golden("worker-source"),
    )
    .unwrap();
    for id in [
        "worker-evidence-0",
        "worker-evidence-1",
        "worker-evidence-2",
        "worker-evidence-3",
    ] {
        publication::journal_object(&store, ObjectKind::Evidence, &source_context(), &golden(id))
            .unwrap();
    }
    // The out-of-scope catalog tables stay untouched by producer commands.
    assert_eq!(
        super::schema::empty_out_of_scope_tables(&path).unwrap(),
        super::schema::OUT_OF_SCOPE
    );
}

#[test]
fn established_child_metadata_conflicts_on_same_key_change() {
    let root = tempdir().unwrap();
    let path = root.path().join("immutable_row.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    let admitted = super::rows::admit_meta_record(
        super::rows::MetaKind::Attachment,
        &golden("worker-attachment"),
    )
    .unwrap();
    publication::journal_child_metadata(&store, super::rows::MetaKind::Attachment, &admitted, None)
        .unwrap();
    // A second record bound to the same childSessionId with different bytes
    // changes the attachment body: re-sealed admission passes, but the row
    // conflict refuses the rewrite.
    let bytes = modified("worker-attachment", |value| {
        value["assignmentIdentity"]["taskVersion"] = json!(3);
        let mut body = value.clone();
        if let Some(map) = body.as_object_mut() {
            map.remove("attachmentDigest");
        }
        let resealed = super::sha256_hex(
            &[
                super::rows::ATTACHMENT_DOMAIN.as_bytes(),
                b"\n",
                &super::canonical(&body).unwrap(),
            ]
            .concat(),
        );
        value["attachmentDigest"] = json!(resealed);
    });
    let re_admitted = super::rows::admit_meta_record(super::rows::MetaKind::Attachment, &bytes)
        .expect("re-sealed attachment record");
    let rejection = publication::journal_child_metadata(
        &store,
        super::rows::MetaKind::Attachment,
        &re_admitted,
        None,
    )
    .err()
    .map(|e| e.0);
    assert_eq!(
        rejection,
        Some("metadata row"),
        "established child metadata rows are immutable"
    );
}

#[test]
fn mid_command_failure_rolls_back_every_row() {
    let root = tempdir().unwrap();
    let path = root.path().join("rollback.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    let _ = publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    super::connection::arm_test_seam();
    let rejection = publication::journal_object(
        &store,
        ObjectKind::Configuration,
        &source_context(),
        &golden("worker-configuration"),
    )
    .err()
    .map(|e| e.0);
    super::connection::disarm_test_seam();
    assert_eq!(rejection, Some("seam"), "injected mid-command failure");
    // The header/latest stays unchanged and the rolled-back proof is absent.
    let connection = store.raw();
    let proof_head: i64 = connection
        .query_row("SELECT MAX(sequence) FROM commit_proof", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(proof_head, 4, "no proof past the pre-failure head");
}

#[test]
fn provenance_edges_journal_with_retained_join() {
    let root = tempdir().unwrap();
    let path = root.path().join("edges.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    let configuration = publication::journal_object(
        &store,
        ObjectKind::Configuration,
        &source_context(),
        &golden("worker-configuration"),
    )
    .unwrap();
    let _evidence = publication::journal_object(
        &store,
        ObjectKind::Evidence,
        &source_context(),
        &golden("worker-evidence-0"),
    )
    .unwrap();
    let consumer_ref = {
        let admitted = super::rows::admit_source_object(
            ObjectKind::Configuration,
            &source_context(),
            &golden("worker-configuration"),
        )
        .unwrap();
        json!({"digest": admitted.digest, "key": admitted.key})
    };
    let target_ref = {
        let admitted = super::rows::admit_source_object(
            ObjectKind::Evidence,
            &source_context(),
            &golden("worker-evidence-0"),
        )
        .unwrap();
        json!({"digest": admitted.digest, "key": admitted.key})
    };
    let edge = publication::journal_dependency(&store, &consumer_ref, &target_ref).unwrap();
    assert!(edge.commit().sequence > configuration.commit().sequence);
    // Exact edge retry: unchanged commit.
    let retry = publication::journal_dependency(&store, &consumer_ref, &target_ref).unwrap();
    assert_eq!(retry.commit(), edge.commit());
    // Cited digest drift breaks the join.
    let drifted = json!({"digest": "0000000000000000000000000000000000000000000000000000000000000000", "key": consumer_ref["key"].clone()});
    assert_eq!(
        publication::journal_dependency(&store, &drifted, &target_ref)
            .err()
            .map(|e| e.0),
        Some("join")
    );
}

#[test]
fn format_four_manifest_digest_recomputes_from_stored_proof() {
    let root = tempdir().unwrap();
    let path = root.path().join("manifest.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    establish_pool(&store);
    publication::journal_object(
        &store,
        ObjectKind::Plan,
        &source_context(),
        &golden("worker-plan"),
    )
    .unwrap();
    let connection = store.raw();
    let (sequence, body, change_digest): (i64, Vec<u8>, String) = connection
        .query_row(
            "SELECT sequence, body, change_digest FROM commit_proof ORDER BY sequence DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        sequence, 4,
        "init + two establishes + the row command carrying both charges"
    );
    // Exact recomputation: the change domain over the manifest bytes.
    let recomputed = super::domain_digest(super::CHANGE_DOMAIN, &body);
    assert_eq!(recomputed, change_digest);
    // Canonical manifest bytes: re-serializing the parsed value is identical.
    let manifest: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(super::canonical(&manifest).unwrap(), body);
}

/// The run/workspace quota rows must be established before any producer
/// journal: missing rows fence the accepted preparation order.
fn establish_pool(store: &Connection) {
    quota::establish(store, &run_owner()).unwrap();
    quota::establish(store, &workspace_owner()).unwrap();
}

/// The run-owner quota key from the accepted owner codecs.
fn run_owner() -> Value {
    json!({
        "kind": "run",
        "workspaceId": "workspace-1",
        "runId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1",
    })
}

fn workspace_owner() -> Value {
    json!({"kind": "workspace", "workspaceId": "workspace-1"})
}

#[test]
fn quota_delta_applies_checked_counters() {
    let root = tempdir().unwrap();
    let path = root.path().join("delta.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    quota::establish(&store, &run_owner()).unwrap();
    let used = quota::apply_delta(&store, &run_owner(), 4096).unwrap();
    assert_eq!(used["used"], "4096");
    // No-refund semantics: a delta below current used is refused.
    assert_eq!(
        quota::apply_delta(&store, &run_owner(), -4097)
            .err()
            .map(|e| e.0),
        Some("quota delta"),
        "negative deltas cannot underflow the counter"
    );
    let released = quota::apply_delta(&store, &run_owner(), -4096).unwrap();
    assert_eq!(released["used"], "0", "exact release reverts to zero");
}

#[test]
fn quota_run_cap_rejects_with_complete_old_state() {
    let root = tempdir().unwrap();
    let path = root.path().join("runcap.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    quota::establish(&store, &run_owner()).unwrap();
    let before = json!({"reserved": "0", "used": "0"});
    // Two-step: one increment to the cap boundary is accepted...
    quota::apply_delta(&store, &run_owner(), 8 * 1024 * 1024).unwrap();
    // ...a second increment rejects at the accepted 8 MiB run cap.
    let rejection = quota::apply_delta(&store, &run_owner(), 1)
        .err()
        .map(|e| e.0);
    assert_eq!(
        rejection,
        Some("quota capacity"),
        "8 MiB run cap is inclusive"
    );
    let after = quota::read_used(&store, &run_owner()).unwrap();
    assert_eq!(
        after["used"],
        json!((8 * 1024 * 1024).to_string()),
        "old state complete"
    );
    let _ = before;
}

#[test]
fn quota_workspace_cap_rejects_with_complete_old_state() {
    let root = tempdir().unwrap();
    let path = root.path().join("wscap.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    quota::establish(&store, &workspace_owner()).unwrap();
    quota::apply_delta(&store, &workspace_owner(), 64 * 1024 * 1024).unwrap();
    assert_eq!(
        quota::apply_delta(&store, &workspace_owner(), 1)
            .err()
            .map(|e| e.0),
        Some("quota capacity"),
        "64 MiB workspace cap is inclusive"
    );
}

#[test]
fn missing_quota_row_fences_the_delta() {
    let root = tempdir().unwrap();
    let path = root.path().join("fence.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        quota::apply_delta(&store, &run_owner(), 1)
            .err()
            .map(|e| e.0),
        Some("quota row"),
        "missing rows fence writes; nothing is autofixed"
    );
    // The refused write left no row behind.
    assert_eq!(
        quota::read_used(&store, &run_owner()).err().map(|e| e.0),
        Some("quota row")
    );
}

#[test]
fn assignment_scope_rows_are_not_part_of_the_retained_pool() {
    let root = tempdir().unwrap();
    let path = root.path().join("scope.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    let assignment_owner = json!({"assignmentIdentity": {"childSessionId": "worker-session", "configurationDigest": "08d017be21a687f290d084ac99eaca328ea8478fb477eedf82fde4170a04b00f", "configurationId": "worker-configuration", "kind": "child", "parentSessionId": "parent-1", "planId": "worker-plan", "planRevision": 1, "reservationId": "worker-reservation", "runId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1", "taskId": "worker-task", "taskVersion": 2, "workspaceId": "workspace-1"}, "kind": "assignment"});
    assert_eq!(
        quota::establish(&store, &assignment_owner)
            .err()
            .map(|e| e.0),
        Some("resource_scope"),
        "child-source-retained-bytes defines no assignment pool"
    );
}

#[test]
fn unknown_resource_rejects() {
    let root = tempdir().unwrap();
    let path = root.path().join("unknown_resource.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        quota::establish_resource(&store, &run_owner(), "reporting-receipt-bytes")
            .err()
            .map(|e| e.0),
        Some("resource"),
        "this kernel admits child-source-retained-bytes only"
    );
}

#[test]
fn fresh_quota_row_establishes_zero() {
    let root = tempdir().unwrap();
    let path = root.path().join("quota.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    let first = quota::establish(&store, &run_owner()).unwrap();
    assert_eq!(
        first["used"], "0",
        "fresh establishment initializes used to canonical zero"
    );
    assert_eq!(first["reserved"], "0");
    // A second establishment is refused, not idempotent-overwritten.
    assert_eq!(
        quota::establish(&store, &run_owner()).err().map(|e| e.0),
        Some("quota row"),
        "establishment never rewrites an existing row"
    );
}

/// The store/journal admission context from the packet's golden identifiers.
fn source_context() -> super::rows::SourceContext<'static> {
    super::rows::SourceContext {
        workspace: "workspace-1",
        run_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1",
    }
}

use super::rows::{self, ObjectKind};

#[test]
fn golden_plan_row_parses_reconstructs_key_and_sealed_digest() {
    let admitted =
        rows::admit_source_object(ObjectKind::Plan, &source_context(), &golden("worker-plan"))
            .unwrap();
    let identity = &admitted.key["identity"];
    assert_eq!(identity["planId"], "worker-plan");
    assert_eq!(identity["planRevision"], 1);
    assert_eq!(
        identity["planDefinitionDigest"],
        "c17753cc252897208b1288ced09bdd8c353c00988f32b49a660a36a3fc778008"
    );
    assert_eq!(identity["workspaceId"], "workspace-1");
    assert_eq!(
        identity["runId"],
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1"
    );
}

#[test]
fn golden_configuration_row_parses_reconstructs_key_and_sealed_digest() {
    let admitted = rows::admit_source_object(
        ObjectKind::Configuration,
        &source_context(),
        &golden("worker-configuration"),
    )
    .unwrap();
    let identity = &admitted.key["identity"];
    assert_eq!(identity["configurationId"], "worker-configuration");
    assert_eq!(
        identity["configurationDigest"],
        "08d017be21a687f290d084ac99eaca328ea8478fb477eedf82fde4170a04b00f"
    );
    assert_eq!(identity["task"]["taskId"], "worker-task");
}

#[test]
fn golden_review_input_row_parses_and_reconstructs_key() {
    let fixture_digest = fixture()["snapshot"]["inputDigest"]
        .as_str()
        .expect("snapshot inputDigest")
        .to_string();
    let admitted = rows::admit_source_object(
        ObjectKind::ReviewInput,
        &source_context(),
        &golden("review-snapshot"),
    )
    .unwrap();
    let identity = &admitted.key["identity"];
    assert_eq!(identity["inputDigest"], fixture_digest);
    assert_eq!(identity["task"]["taskId"], "reviewer-task");
    assert_eq!(identity["task"]["workspaceId"], "workspace-1");
}

#[test]
fn golden_evidence_rows_parse_and_verify_raw_content_digest() {
    let vectors = fixture()["vectors"].as_array().expect("vectors array");
    for id in [
        "worker-evidence-0",
        "worker-evidence-1",
        "worker-evidence-2",
        "worker-evidence-3",
    ] {
        let vector = vectors
            .iter()
            .find(|v| v["id"] == id)
            .expect("evidence vector");
        let admitted =
            rows::admit_source_object(ObjectKind::Evidence, &source_context(), &golden(id))
                .unwrap();
        let identity = &admitted.key["identity"];
        let body: Value = serde_json::from_str(vector["canonical"].as_str().unwrap()).unwrap();
        assert_eq!(
            identity["contentDigest"], body["contentDigest"],
            "evidence key carries the raw content digest"
        );
        assert_eq!(
            admitted.digest,
            vector["domainDigest"].as_str().expect("domainDigest"),
            "source digest is the child-source-evidence domain over the body"
        );
    }
}

#[test]
fn golden_assignment_source_row_parses_and_reconstructs_identity() {
    let admitted = rows::admit_source_object(
        ObjectKind::AssignmentSource,
        &source_context(),
        &golden("worker-source"),
    )
    .unwrap();
    let identity = &admitted.key["identity"];
    assert_eq!(identity["kind"], "child");
    assert_eq!(identity["taskId"], "worker-task");
    assert_eq!(identity["taskVersion"], 2);
    assert_eq!(identity["childSessionId"], "worker-session");
    assert_eq!(
        admitted.digest, "970838d0a405e008900b866d37ed0f0a31d7723288559eb85576ff9886cafbac",
        "source digest is the whole-body domain digest"
    );
}

#[test]
fn out_of_scope_tables_stay_empty() {
    let root = tempdir().unwrap();
    let path = root.path().join("empty_out_of_scope.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None);
    drop(store);
    assert_eq!(
        super::schema::empty_out_of_scope_tables(&path).unwrap(),
        super::schema::OUT_OF_SCOPE,
        "out-of-scope catalog tables exist and are untouched by this kernel"
    );
}

#[test]
fn fresh_open_creates_store() {
    let root = tempdir().unwrap();
    let path = root.path().join("hierarchy").join("v1.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None, "fresh store is available");
    assert_eq!(store.workspace_id(), "workspace-1");
    assert!(!store.epoch().is_empty(), "fresh store owns a random epoch");
    drop(store);
    assert!(path.exists(), "catalog must be created on disk");
}

#[test]
fn reopen_verifies_epoch_and_workspace() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    let epoch = {
        let store = Connection::open(&path, "workspace-1").unwrap();
        store.epoch().to_string()
    };
    let reopened = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(reopened.unavailable_reason(), None);
    assert_eq!(
        reopened.epoch(),
        &epoch,
        "reopen must not re-roll the epoch"
    );
    assert_eq!(reopened.workspace_id(), "workspace-1");
}

#[test]
fn workspace_mismatch_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    let store = Connection::open(&path, "other-workspace").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("workspace"),
        "workspace mismatch fences the store"
    );
}

#[test]
fn non_sqlite_file_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    std::fs::write(&path, b"definitely not a database").unwrap();
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("physical database"),
        "a foreign magic file must not be repaired"
    );
}

#[test]
fn foreign_table_is_unavailable() {
    let root = tempdir().unwrap();
    let path = root.path().join("store.sqlite");
    drop(Connection::open(&path, "workspace-1").unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch("CREATE TABLE snooping (x INTEGER);")
            .unwrap();
    }
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(
        store.unavailable_reason(),
        Some("catalog"),
        "unknown table fences rather than repairs"
    );
}

#[test]
fn superseded_quota_row_opens_unavailable_without_repair() {
    let root = tempdir().unwrap();
    let path = root.path().join("superseded_quota.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    quota::establish(&store, &run_owner()).unwrap();
    let original: (Vec<u8>, Vec<u8>, i64) = store
        .raw()
        .query_row(
            "SELECT value, last_commit, sequence FROM quota_usage",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    // The final byte total is zero again, so quota recomputation alone
    // cannot distinguish the original row from the committed current row.
    quota::apply_delta(&store, &run_owner(), 1).unwrap();
    quota::apply_delta(&store, &run_owner(), -1).unwrap();
    let latest = store.read_latest().unwrap();
    drop(store);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), None);
    assert_eq!(store.read_latest().unwrap(), latest);
    store
        .raw()
        .execute(
            "UPDATE quota_usage SET value = ?1, last_commit = ?2, sequence = ?3",
            rusqlite::params![original.0, original.1, original.2],
        )
        .unwrap();
    drop(store);
    let before = store_bytes(&path);
    let store = Connection::open(&path, "workspace-1").unwrap();
    assert_eq!(store.unavailable_reason(), Some("rows"));
    assert_eq!(
        store_bytes(&path),
        before,
        "recovery never repairs stale rows"
    );
    assert_eq!(
        quota::apply_delta(&store, &run_owner(), 0)
            .err()
            .map(|e| e.0),
        Some("unavailable")
    );
}

#[test]
fn extreme_quota_deltas_refuse_without_mutation_or_poisoning() {
    let root = tempdir().unwrap();
    let path = root.path().join("extreme_quota.sqlite");
    let store = Connection::open(&path, "workspace-1").unwrap();
    quota::establish(&store, &run_owner()).unwrap();
    quota::apply_delta(&store, &run_owner(), 1).unwrap();
    let before = store_bytes(&path);
    let latest = store.read_latest().unwrap();
    for delta in [i128::MAX, i128::MIN] {
        assert!(quota::apply_delta(&store, &run_owner(), delta).is_err());
        assert_eq!(quota::read_used(&store, &run_owner()).unwrap()["used"], "1");
        assert_eq!(store.read_latest().unwrap(), latest);
        assert_eq!(store_bytes(&path), before);
    }
    assert_eq!(
        quota::apply_delta(&store, &run_owner(), 1).unwrap()["used"],
        "2"
    );
}
