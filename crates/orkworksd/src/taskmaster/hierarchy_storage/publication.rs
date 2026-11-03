//! Non-source-valid storage primitives: journal one retained child-source
//! object, one child metadata row, one provenance dependency edge or one
//! quota delta — one transaction per command with its complete change
//! manifest + proof entry. This unit publishes NO accepted source-owned
//! transition: the attachment transition needs its exact accepted effect set
//! (initial summary state, the five fresh assignment quota rows, retry/
//! interrupted dispositions) which this mechanism does not supply, so
//! attachment publication stays gated.

use super::connection::{Connection, RowChange, RowMode};
use super::rows::{
    self, AdmittedMeta, MetaKind, ObjectKind, SourceContext, CHILD_SOURCE_PROVENANCE,
};
use super::{ensure, HierarchyStorageError, Result, JOURNAL_COMMAND};
use serde_json::{json, Value};

/// The outcome of a journal command: the CommitRef its rows (and proof) link
/// to. Exact retries return the unchanged current commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JournalOutcome {
    commit: rows::CommitRef,
}

impl JournalOutcome {
    pub(crate) fn commit(&self) -> rows::CommitRef {
        self.commit.clone()
    }
}

/// Journal one retained child-source object row, immutable on conflict; an
/// exact retry of identical bytes is a no-op.
pub(crate) fn journal_object(
    store: &Connection,
    kind: ObjectKind,
    context: &SourceContext,
    bytes: &[u8],
) -> Result<JournalOutcome> {
    let admitted = rows::admit_source_object(kind, context, bytes)?;
    let value = serde_json::from_slice::<Value>(&admitted.bytes)
        .map_err(|_| HierarchyStorageError("canonical wire"))?;
    let body = json!({"body": value, "digest": admitted.digest});
    let outcome = store.journal_rows(
        JOURNAL_COMMAND,
        vec![RowChange {
            table: "objects",
            mode: RowMode::Insert,
            key: admitted.key,
            value: body,
        }],
        "mutable object",
    )?;
    Ok(JournalOutcome {
        commit: outcome.commit,
    })
}

/// Journal one `child_*` metadata row; the approvals envelope carries the
/// source digest and receipt; reservation/allocation/attachment rows are
/// body-only. Exact retries are no-ops.
pub(crate) fn journal_child_metadata(
    store: &Connection,
    kind: MetaKind,
    admitted: &AdmittedMeta,
    receipt: Option<&Value>,
) -> Result<JournalOutcome> {
    let mut value = admitted.value_base.clone();
    match kind {
        MetaKind::PlanApproval => {
            value["digest"] = json!(admitted.digest);
            let receipt = receipt.ok_or(HierarchyStorageError("receipt"))?;
            value["receipt"] = receipt.clone();
        }
        _ => {
            ensure(receipt.is_none(), "unexpected receipt")?;
        }
    }
    // Envelope/cap validation against the longest possible CommitRef so the
    // actual write can never exceed the complete-pair wrap cap.
    let synthetic_commit = json!({
        "changeDigest": "0".repeat(64),
        "commitSequence": u64::MAX.to_string(),
        "storageEpoch": "0".repeat(64),
    });
    let mut full = value.clone();
    full["lastCommit"] = synthetic_commit;
    rows::check_metadata_row(kind, &admitted.key, &full)?;
    let outcome = store.journal_rows(
        JOURNAL_COMMAND,
        vec![RowChange {
            table: kind.table_name(),
            mode: RowMode::Insert,
            key: admitted.key.clone(),
            value,
        }],
        "metadata row",
    )?;
    Ok(JournalOutcome {
        commit: outcome.commit,
    })
}

/// Provenance dependency edge: `child-source-provenance` only; both endpoints
/// must resolve to retained object rows with the cited digests.
pub(crate) fn journal_dependency(
    store: &Connection,
    consumer: &Value,
    target: &Value,
) -> Result<JournalOutcome> {
    for endpoint in [consumer, target] {
        rows::object_ref(endpoint)?;
        let key = &endpoint["key"];
        let stored = store.read_row("objects", key, "join")?;
        let digest = stored["digest"]
            .as_str()
            .ok_or(HierarchyStorageError("join"))?;
        ensure(endpoint["digest"] == json!(digest), "join")?;
    }
    let key = json!({
        "consumer": consumer,
        "purpose": CHILD_SOURCE_PROVENANCE,
        "target": target,
    });
    let outcome = store.journal_rows(
        JOURNAL_COMMAND,
        vec![RowChange {
            table: "dependencies",
            mode: RowMode::Insert,
            key,
            value: json!({}),
        }],
        "dependency",
    )?;
    Ok(JournalOutcome {
        commit: outcome.commit,
    })
}
