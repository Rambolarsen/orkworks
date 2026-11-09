//! Private descriptor-based file effects; checked producer composition remains gated.
use super::*;
use crate::session_store::ObservationSource as SqlSource;
use std::{fs::File, time::Instant};

#[derive(Debug)]
pub(crate) enum WriteError {
    Storage(StoreError),
    Candidate(RecordError),
}
impl From<StoreError> for WriteError {
    fn from(error: StoreError) -> Self {
        Self::Storage(error)
    }
}
impl From<RecordError> for WriteError {
    fn from(error: RecordError) -> Self {
        Self::Candidate(error)
    }
}
impl From<std::io::Error> for WriteError {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error.into())
    }
}
const MAX_INTENT: usize = 64 * 1024;
type WriteResult<T> = std::result::Result<T, WriteError>;

pub(in crate::workflow_observations) fn persist(
    directory: &File,
    admission: crate::session_application::sql_writes::ObservationWriteAdmission,
    request: ScanRequest,
    origin: ObservationOrigin,
    key: &str,
    candidate: ObservationCandidate,
) -> WriteResult<RecordOutcome> {
    persist_files(
        directory,
        request,
        admission.source(),
        origin,
        key,
        candidate,
        Utc::now(),
    )
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn persist_files(
    directory: &File,
    request: ScanRequest,
    source: &SqlSource,
    origin: ObservationOrigin,
    key: &str,
    candidate: ObservationCandidate,
    now: DateTime<Utc>,
) -> WriteResult<RecordOutcome> {
    let deadline = request.deadline();
    check_deadline(deadline)?;
    if key.is_empty() {
        return Err(RecordError::EmptyIdempotencyKey.into());
    }
    // Bound raw input before NFKC/canonicalization or escaped serialization can
    // allocate in proportion to it. The encoded check below remains independent.
    if [
        key.len(),
        candidate.description.len(),
        candidate.evidence.len(),
        candidate.problem_area.as_ref().map_or(0, String::len),
    ]
    .into_iter()
    .try_fold(0usize, |sum, n| sum.checked_add(n))
    .is_none_or(|sum| sum > MAX_INTENT)
    {
        return Err(StoreError::TooLarge.into());
    }
    let (public_source, confidence) = validate_candidate(origin, &candidate)?;
    // Leave room in the original 1 MiB slot for compact inventory/target indices
    // and the single line buffer. Charge escaped intent and original provenance.
    if serde_json::to_vec(&(
        source,
        key,
        &candidate.description,
        &candidate.evidence,
        &candidate.problem_area,
        candidate.kind,
        candidate.reported_impact,
        candidate.confidence,
    ))
    .map_err(StoreError::from)?
    .len()
        > MAX_INTENT
    {
        return Err(StoreError::TooLarge.into());
    }
    let area = candidate
        .problem_area
        .as_deref()
        .map(canonicalize_problem_area)
        .transpose()?;
    let fingerprint = match area.as_deref() {
        Some(v) => fingerprint_v2(candidate.kind, v),
        None => format!(
            "v1:{}:{}",
            candidate.kind.as_str(),
            normalize_description(&candidate.description)
        ),
    };
    let key_hash = hash_key(source.session_id(), key);
    let payload_hash = hash_payload(
        candidate.kind,
        &candidate.description,
        &candidate.evidence,
        candidate.reported_impact,
        area.as_deref(),
    );
    let name = native::component(directory, source.session_id())?;
    let mut segment = native::Pin::capture(directory, &name)?;
    let counter = native::Pin::capture(directory, c"sequence")?;
    let (mut indices, mut receipt, mut recent) = (Vec::new(), None, 0usize);
    let inventory = request.inspect_at(
        directory,
        source.database_epoch(),
        source.workspace_sha256(),
        |session, record, offset, len, stamp| {
            if session != source.session_id() {
                return Ok(());
            }
            if segment.original != Some(stamp) {
                return Err(StoreError::Conflict);
            }
            if let Some(found) =
                mutation::retry_for_source(record, source, &key_hash, &payload_hash, now)?
            {
                if receipt.is_some() {
                    return Err(StoreError::Corrupt);
                }
                receipt = Some(found);
            }
            let accepted = mutation::retry_fields(record).2;
            recent += usize::from(within_rate_window(now, accepted));
            let observation = record.observation().is_some();
            indices.push(Index {
                offset,
                len: len as u32,
                sequence: record.sequence(),
                output_len: len as u32 + 1,
                evicted: false,
                mode: if observation {
                    Mode::Observation
                } else if within_window(now, accepted) {
                    Mode::Tombstone
                } else {
                    Mode::Drop
                },
            });
            // At most 2,024 compact 32-byte indices, independently of payload size.
            if indices.len() * 32 > 64 * 1024 {
                return Err(StoreError::TooLarge);
            }
            Ok(())
        },
    )?;
    segment.verify(directory)?;
    counter.verify(directory)?;
    #[cfg(test)]
    AFTER_SCAN.with(|slot| {
        if let Some(callback) = slot.borrow_mut().take() {
            callback();
        }
    });
    if let Some(receipt) = receipt {
        check_deadline(deadline)?;
        segment.verify(directory)?;
        counter.verify(directory)?;
        return Ok(receipt);
    }
    if recent >= MAX_ACCEPTED_PER_SESSION_MINUTE {
        return Err(RecordError::RateLimited.into());
    }
    if segment.original.is_none() && inventory.segments >= 4096 {
        return Err(StoreError::TooLarge.into());
    }
    let sequence = inventory
        .sequence
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(StoreError::TooLarge)?;
    let observation = WorkflowObservation {
        id: uuid::Uuid::new_v4().to_string(),
        sequence,
        session_id: source.session_id().into(),
        observed_at: now.to_rfc3339(),
        kind: candidate.kind,
        description: candidate.description,
        evidence: candidate.evidence,
        problem_area: area,
        reported_impact: candidate.reported_impact,
        source: public_source,
        confidence,
        fingerprint,
    };
    let record = RecordData(Record::Observation(ObservationData {
        origin: OriginData(OriginWire {
            schema_version: 1,
            database_epoch: source.database_epoch().into(),
            workspace_sha256: source.workspace_sha256().into(),
            session_id: source.session_id().into(),
            created_at: source.created_at().into(),
            observation_sha256: value_digest(&observation)?,
        }),
        observation: observation.clone(),
        idempotency_key_hash: key_hash,
        payload_hash,
    }));
    record.validate(source.session_id())?;
    let line = record.encode()?;
    indices.sort_by_key(|index| index.sequence);
    let mut budget = ReadBudget {
        used: inventory.bytes,
    };
    let mut bytes = line.len() as u64
        + 1
        + indices
            .iter()
            .filter(|i| i.mode != Mode::Drop)
            .map(|i| u64::from(i.output_len))
            .sum::<u64>();
    let mut observations = 1 + indices
        .iter()
        .filter(|i| i.mode == Mode::Observation)
        .count();
    let mut tombstones = indices.iter().filter(|i| i.mode == Mode::Tombstone).count();
    while observations > MAX_SEGMENT_OBSERVATIONS || bytes > MAX_SEGMENT_BYTES {
        let index = indices
            .iter_mut()
            .find(|i| i.mode == Mode::Observation)
            .ok_or(StoreError::TooLarge)?;
        let old = decode_line(
            &segment.read(directory, index, &mut budget, Some(deadline))?,
            source.session_id(),
        )?;
        bytes -= u64::from(index.output_len);
        observations -= 1;
        if let Some(tombstone) = mutation::evicted_retry(source.session_id(), &old, now)? {
            index.mode = Mode::Tombstone;
            index.evicted = true;
            index.output_len = tombstone.encode()?.len() as u32 + 1;
            bytes += u64::from(index.output_len);
            tombstones += 1;
        } else {
            index.mode = Mode::Drop;
        }
        while tombstones > MAX_TOMBSTONES {
            let oldest = indices
                .iter_mut()
                .find(|i| i.mode == Mode::Tombstone)
                .ok_or(StoreError::Corrupt)?;
            oldest.mode = Mode::Drop;
            bytes -= u64::from(oldest.output_len);
            tombstones -= 1;
        }
    }
    let rewrite = indices.iter().any(|i| i.mode == Mode::Drop || i.evicted)
        || segment
            .original
            .is_some_and(|stamp| stamp.2 + line.len() as u64 + 1 > MAX_SEGMENT_BYTES);
    if rewrite {
        // Preflight every later source read before counter effects. Copies are streamed.
        let future_reads: usize = indices
            .iter()
            .filter(|i| i.mode != Mode::Drop)
            .map(|i| i.len as usize)
            .sum();
        budget.check(future_reads)?;
    }
    check_deadline(deadline)?;
    segment.verify(directory)?;
    counter.verify(directory)?;
    let advanced = native::counter(directory, &counter, sequence)?;
    #[cfg(test)]
    native::fail(native::Fault::AfterCounter)?;
    #[cfg(test)]
    AFTER_COUNTER.with(|slot| {
        if let Some(callback) = slot.borrow_mut().take() {
            callback();
        }
    });
    advanced.verify(directory)?;
    // Effects have started. Finish this finite original operation; do not reset
    // its deadline or abandon resources in favour of a replacement write.
    if rewrite {
        native::rewrite(
            directory,
            &mut segment,
            &indices,
            &line,
            source.session_id(),
            now,
            &mut budget,
        )?;
    } else {
        native::append(directory, &segment, &line)?;
    }
    advanced.verify(directory)?;
    drop(inventory); // Original full file-job reservation spans all actual effects.
                     // Definitive file receipt for the original outer job. Caller/detached
                     // publication belongs to that owner; do not conceal a durable commit with
                     // a post-effect Expired result.
    Ok(RecordOutcome::Accepted(observation))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn persist_files(
    _: &File,
    _: ScanRequest,
    _: &SqlSource,
    _: ObservationOrigin,
    _: &str,
    _: ObservationCandidate,
    _: DateTime<Utc>,
) -> WriteResult<RecordOutcome> {
    Err(StoreError::Unsupported.into())
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Observation,
    Tombstone,
    Drop,
}
struct Index {
    offset: u64,
    sequence: u64,
    len: u32,
    output_len: u32,
    mode: Mode,
    evicted: bool,
}
struct ReadBudget {
    used: usize,
}
impl ReadBudget {
    fn check(&self, n: usize) -> Result<()> {
        self.used
            .checked_add(n)
            .filter(|v| *v <= reader::MAX_TOTAL_BYTES)
            .map(|_| ())
            .ok_or(StoreError::TooLarge)
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        self.check(n)?;
        self.used += n;
        Ok(())
    }
}
fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(StoreError::Expired)
    } else {
        Ok(())
    }
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod native;
#[cfg(test)]
thread_local! {static AFTER_COUNTER:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}
#[cfg(test)]
pub(crate) fn test_after_counter(callback: impl FnOnce() + 'static) {
    AFTER_COUNTER.with(|slot| *slot.borrow_mut() = Some(Box::new(callback)));
}
#[cfg(test)]
thread_local! {static AFTER_SCAN:std::cell::RefCell<Option<Box<dyn FnOnce()>>>=std::cell::RefCell::new(None);}
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests;
