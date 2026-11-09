//! Pure retry/eviction rules for complete records. No original claim or I/O authority.
use super::*;

/// Inspect a complete bounded session segment, including retry tombstones.
/// A selected observation snapshot is insufficient: it omits evicted retry keys.
/// The caller still owns inventory, counter, source and admission validation.
/// `incoming` is proposed data, never proof of the right to persist that data.
pub(super) fn resolve_retry(
    records: &[RecordData],
    incoming: &RecordData,
    now: DateTime<Utc>,
) -> Result<Option<RecordOutcome>> {
    let Record::Observation(proposed) = &incoming.0 else {
        return Err(StoreError::Conflict);
    };
    incoming.validate(proposed.origin.session_id())?;
    incoming.encode()?;
    if records.len() > MAX_SEGMENT_OBSERVATIONS + MAX_TOMBSTONES {
        return Err(StoreError::TooLarge);
    }
    let (mut observations, mut tombstones, mut bytes) = (0, 0, 0u64);
    let mut receipt = None;
    for record in records {
        record.validate(proposed.origin.session_id())?;
        bytes += record.encode()?.len() as u64 + 1;
        if record.observation().is_some() {
            observations += 1;
        } else {
            tombstones += 1;
        }
        if observations > MAX_SEGMENT_OBSERVATIONS
            || tombstones > MAX_TOMBSTONES
            || bytes > MAX_SEGMENT_BYTES
        {
            return Err(StoreError::TooLarge);
        }
        if let Some(found) = matching_retry(
            record,
            &proposed.idempotency_key_hash,
            &proposed.payload_hash,
            now,
            |origin| same_scope(origin, &proposed.origin),
        )? {
            if receipt.is_some() {
                return Err(StoreError::Corrupt);
            }
            receipt = Some(found);
        }
    }
    Ok(receipt)
}

pub(super) fn retry_for_source(
    record: &RecordData,
    source: &crate::session_store::ObservationSource,
    key: &str,
    payload: &str,
    now: DateTime<Utc>,
) -> Result<Option<RecordOutcome>> {
    if !matches_source(record, source, key, payload, now)? {
        return Ok(None);
    }
    Ok(Some(receipt(record)))
}
pub(super) fn matches_source(
    record: &RecordData,
    source: &crate::session_store::ObservationSource,
    key: &str,
    payload: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    matches_retry(record, key, payload, now, |origin| {
        origin.database_epoch() == source.database_epoch()
            && origin.workspace_sha256() == source.workspace_sha256()
            && origin.session_id() == source.session_id()
            && origin.created_at() == source.created_at()
    })
}
fn matching_retry(
    record: &RecordData,
    key: &str,
    payload: &str,
    now: DateTime<Utc>,
    same_scope: impl FnOnce(&OriginData) -> bool,
) -> Result<Option<RecordOutcome>> {
    Ok(matches_retry(record, key, payload, now, same_scope)?.then(|| receipt(record)))
}
fn receipt(record: &RecordData) -> RecordOutcome {
    RecordOutcome::Duplicate {
        observation_id: record.identity().0.into(),
        sequence: record.sequence(),
        accepted_at: retry_fields(record).2.into(),
    }
}
fn matches_retry(
    record: &RecordData,
    expected_key: &str,
    expected_payload: &str,
    now: DateTime<Utc>,
    same_scope: impl FnOnce(&OriginData) -> bool,
) -> Result<bool> {
    let (key, payload, accepted) = retry_fields(record);
    if key != expected_key || !within_window(now, accepted) {
        return Ok(false);
    }
    let Some(origin) = record.origin() else {
        return Err(StoreError::Conflict);
    };
    if !same_scope(origin) || payload != expected_payload {
        return Err(StoreError::Conflict);
    }
    Ok(true)
}

fn same_scope(a: &OriginData, b: &OriginData) -> bool {
    a.database_epoch() == b.database_epoch()
        && a.workspace_sha256() == b.workspace_sha256()
        && a.session_id() == b.session_id()
        && a.created_at() == b.created_at()
}

pub(super) fn retry_fields(record: &RecordData) -> (&str, &str, &str) {
    match &record.0 {
        Record::Observation(v) => (
            &v.idempotency_key_hash,
            &v.payload_hash,
            &v.observation.observed_at,
        ),
        Record::LegacyObservation(v) => (
            &v.idempotency_key_hash,
            &v.payload_hash,
            &v.observation.observed_at,
        ),
        Record::Tombstone(v) => (&v.idempotency_key_hash, &v.payload_hash, &v.accepted_at),
        Record::LegacyTombstone(v) => (&v.idempotency_key_hash, &v.payload_hash, &v.accepted_at),
    }
}

/// Plan only the retry record for an eviction. Borrowed input is unchanged.
/// Qualified origins are copied from their own value; legacy records stay legacy.
/// Existing tombstones survive unchanged only while their retry window is live.
pub(super) fn evicted_retry(
    session: &str,
    record: &RecordData,
    now: DateTime<Utc>,
) -> Result<Option<RecordData>> {
    record.validate(session)?;
    record.encode()?;
    if !within_window(now, retry_fields(record).2) {
        return Ok(None);
    }
    let retained = RecordData(match &record.0 {
        Record::Observation(v) => Record::Tombstone(TombstoneData {
            idempotency_key_hash: v.idempotency_key_hash.clone(),
            payload_hash: v.payload_hash.clone(),
            observation_id: v.observation.id.clone(),
            sequence: v.observation.sequence,
            accepted_at: v.observation.observed_at.clone(),
            origin: v.origin.clone(),
        }),
        Record::LegacyObservation(v) => Record::LegacyTombstone(Tombstone {
            idempotency_key_hash: v.idempotency_key_hash.clone(),
            payload_hash: v.payload_hash.clone(),
            observation_id: v.observation.id.clone(),
            sequence: v.observation.sequence,
            accepted_at: v.observation.observed_at.clone(),
        }),
        Record::Tombstone(_) | Record::LegacyTombstone(_) => return Ok(Some(record.clone())),
    });
    retained.validate(session)?;
    retained.encode()?;
    Ok(Some(retained))
}

#[cfg(test)]
mod tests;
