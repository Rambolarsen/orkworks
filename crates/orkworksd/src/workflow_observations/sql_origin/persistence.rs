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
pub(in crate::workflow_observations) fn persist_batch(
    directory: &File,
    admission: &crate::session_application::sql_writes::ObservationWriteAdmission,
    request: ScanRequest,
    generation: u64,
    candidates: Vec<(usize, ObservationCandidate)>,
) -> WriteResult<Vec<(usize, WriteResult<RecordOutcome>)>> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        persist_batch_files(
            directory,
            request,
            admission.source(),
            generation,
            candidates,
            Utc::now,
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (directory, admission, request, generation, candidates);
        Err(StoreError::Unsupported.into())
    }
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
    let mut results = persist_intents(
        directory,
        request,
        source,
        vec![(0, origin, key.to_owned(), candidate)],
        || now,
    )?;
    results.remove(0).1
}
struct Prepared {
    candidate: ObservationCandidate,
    public_source: ObservationSource,
    confidence: f64,
    area: Option<String>,
    fingerprint: String,
    key_hash: String,
    payload_hash: String,
}
fn prepare(
    source: &SqlSource,
    origin: ObservationOrigin,
    key: &str,
    candidate: ObservationCandidate,
) -> WriteResult<Prepared> {
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
    Ok(Prepared {
        candidate,
        public_source,
        confidence,
        area,
        fingerprint,
        key_hash,
        payload_hash,
    })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn persist_batch_files(
    directory: &File,
    request: ScanRequest,
    source: &SqlSource,
    generation: u64,
    candidates: Vec<(usize, ObservationCandidate)>,
    clock: impl FnMut() -> DateTime<Utc>,
) -> WriteResult<Vec<(usize, WriteResult<RecordOutcome>)>> {
    if candidates.len() > 5 {
        return Err(StoreError::TooLarge.into());
    }
    let mut seen = [false; 5];
    for (index, _) in &candidates {
        if *index >= seen.len() || seen[*index] {
            return Err(StoreError::Invalid("invalid final-scan candidate index".into()).into());
        }
        seen[*index] = true;
    }
    persist_intents(
        directory,
        request,
        source,
        candidates
            .into_iter()
            .map(|(index, candidate)| {
                (
                    index,
                    ObservationOrigin::Peon,
                    format!("final-scan:{generation}:{index}"),
                    candidate,
                )
            })
            .collect(),
        clock,
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn persist_intents(
    directory: &File,
    request: ScanRequest,
    source: &SqlSource,
    intents: Vec<(usize, ObservationOrigin, String, ObservationCandidate)>,
    mut clock: impl FnMut() -> DateTime<Utc>,
) -> WriteResult<Vec<(usize, WriteResult<RecordOutcome>)>> {
    let deadline = request.deadline();
    check_deadline(deadline)?;
    // One aggregate allowance, before normalization or escaped allocations.
    let raw = intents.iter().try_fold(0usize, |sum, (_, _, key, c)| {
        [
            key.len(),
            c.description.len(),
            c.evidence.len(),
            c.problem_area.as_ref().map_or(0, String::len),
        ]
        .into_iter()
        .try_fold(sum, usize::checked_add)
    });
    if raw.is_none_or(|n| n > MAX_INTENT) {
        return Err(StoreError::TooLarge.into());
    }
    // Charge original escaped fields and every key/source before normalization;
    // canonicalized/truncated problem areas cannot shrink the admitted intent.
    let encoded = intents
        .iter()
        .try_fold(0usize, |sum, (_, _, key, candidate)| {
            let bytes = serde_json::to_vec(&(
                source,
                key,
                &candidate.description,
                &candidate.evidence,
                &candidate.problem_area,
                candidate.kind,
                candidate.reported_impact,
                candidate.confidence,
            ))?;
            sum.checked_add(bytes.len())
                .filter(|n| *n <= MAX_INTENT)
                .ok_or(StoreError::TooLarge)
        })?;
    debug_assert!(encoded <= MAX_INTENT);
    let mut slots: Vec<_> = intents
        .into_iter()
        .map(|(index, origin, key, candidate)| (index, prepare(source, origin, &key, candidate)))
        .collect();
    let scan_now = clock();
    let name = native::component(directory, source.session_id())?;
    let mut segment = native::Pin::capture(directory, &name)?;
    let counter = native::Pin::capture(directory, c"sequence")?;
    let mut indices = Vec::new();
    let mut retries: Vec<WriteResult<Option<usize>>> = (0..slots.len()).map(|_| Ok(None)).collect();
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
            for ((_, prepared), retry) in slots.iter().zip(&mut retries) {
                if let (Ok(p), Ok(found)) = (prepared, &mut *retry) {
                    match mutation::matches_source(
                        record,
                        source,
                        &p.key_hash,
                        &p.payload_hash,
                        scan_now,
                    ) {
                        Ok(true) if found.is_some() => *retry = Err(StoreError::Corrupt.into()),
                        Ok(true) => *found = Some(indices.len()),
                        Ok(false) => (),
                        Err(e) => *retry = Err(e.into()),
                    }
                }
            }
            let accepted =
                parse_time(mutation::retry_fields(record).2).ok_or(StoreError::Corrupt)?;
            indices.push(Index {
                offset,
                len: len as u32,
                sequence: record.sequence(),
                output_len: len as u32 + 1,
                evicted: false,
                accepted,
                mode: if record.observation().is_some() {
                    Mode::Observation
                } else {
                    Mode::Tombstone
                },
            });
            if indices.len() * std::mem::size_of::<Index>() > 128 * 1024 {
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
    let mut budget = ReadBudget {
        used: inventory.bytes,
    };
    // Receipt materialization occurs after the global duplicate inventory has
    // been released. Every reread consumes the same aggregate read allowance.
    let mut receipts = Vec::with_capacity(slots.len());
    for retry in retries {
        receipts.push(match retry {
            Err(e) => Err(e),
            Ok(None) => Ok(None),
            Ok(Some(index)) => {
                let record = decode_line(
                    &segment.read(directory, &indices[index], &mut budget, Some(deadline))?,
                    source.session_id(),
                )?;
                Ok(Some(RecordOutcome::Duplicate {
                    observation_id: record.identity().0.into(),
                    sequence: record.sequence(),
                    accepted_at: mutation::retry_fields(&record).2.into(),
                }))
            }
        });
    }
    let mut editor = Editor {
        directory,
        source,
        deadline,
        segment,
        counter,
        indices,
        budget,
        inventory,
        effects_started: false,
        poisoned: false,
    };
    let mut results = Vec::with_capacity(slots.len());
    for ((index, prepared), receipt) in slots.drain(..).zip(receipts) {
        let result = if editor.poisoned {
            Err(StoreError::Unavailable.into())
        } else {
            match (prepared, receipt) {
                (Err(e), _) | (_, Err(e)) => Err(e),
                (Ok(_), Ok(Some(receipt))) => (|| {
                    if !editor.effects_started {
                        check_deadline(deadline)?;
                    }
                    editor.poisoned = true;
                    editor.segment.verify(directory)?;
                    editor.counter.verify(directory)?;
                    editor.poisoned = false;
                    Ok(receipt)
                })(),
                (Ok(p), Ok(None)) if !editor.poisoned => editor.write(p, clock()),
                _ => Err(StoreError::Unavailable.into()),
            }
        };
        results.push((index, result));
    }
    Ok(results)
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
struct Editor<'a> {
    directory: &'a File,
    source: &'a SqlSource,
    deadline: Instant,
    segment: native::Pin,
    counter: native::Pin,
    indices: Vec<Index>,
    budget: ReadBudget,
    inventory: reader::Inventory,
    effects_started: bool,
    poisoned: bool,
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Editor<'_> {
    fn write(&mut self, prepared: Prepared, now: DateTime<Utc>) -> WriteResult<RecordOutcome> {
        let Prepared {
            candidate,
            public_source,
            confidence,
            area,
            fingerprint,
            key_hash,
            payload_hash,
        } = prepared;
        let directory = self.directory;
        let source = self.source;
        let deadline = (!self.effects_started).then_some(self.deadline);
        if let Some(deadline) = deadline {
            check_deadline(deadline)?;
        }
        let mut indices = self.indices.clone();
        for index in &mut indices {
            if index.mode == Mode::Tombstone
                && now.signed_duration_since(index.accepted)
                    > chrono::Duration::seconds(IDEMPOTENCY_WINDOW_SECS)
            {
                index.mode = Mode::Drop;
            }
        }
        let recent = indices
            .iter()
            .filter(|i| {
                now.signed_duration_since(i.accepted)
                    <= chrono::Duration::seconds(RATE_LIMIT_WINDOW_SECS)
            })
            .count();
        let segment = &mut self.segment;
        let counter = &self.counter;
        let inventory = &self.inventory;
        let budget = &mut self.budget;
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
                &segment.read(directory, index, budget, deadline)?,
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
        let old_segment_bytes = usize::try_from(segment.original.map_or(0, |stamp| stamp.2))
            .map_err(|_| StoreError::TooLarge)?;
        let old_counter_bytes = usize::try_from(counter.original.map_or(0, |stamp| stamp.2))
            .map_err(|_| StoreError::TooLarge)?;
        let new_counter_bytes = sequence.to_string().len();
        let new_segment_bytes = if rewrite {
            usize::try_from(bytes).map_err(|_| StoreError::TooLarge)?
        } else {
            old_segment_bytes
                .checked_add(line.len() + 1)
                .ok_or(StoreError::TooLarge)?
        };
        let new_segment_lines = indices
            .iter()
            .filter(|index| index.mode != Mode::Drop)
            .count()
            + 1;
        check_projected(
            inventory.lines,
            indices.len(),
            new_segment_lines,
            reader::MAX_LINES,
        )?;
        // The counter can survive a failed append. Its intermediate durable state
        // must remain readable even when a later rewrite would reclaim bytes.
        check_projected(
            inventory.bytes,
            old_counter_bytes,
            new_counter_bytes,
            reader::MAX_TOTAL_BYTES,
        )?;
        check_projected(
            inventory.bytes,
            old_segment_bytes + old_counter_bytes,
            new_segment_bytes + new_counter_bytes,
            reader::MAX_TOTAL_BYTES,
        )?;
        if let Some(deadline) = deadline {
            check_deadline(deadline)?;
        }
        segment.verify(directory)?;
        counter.verify(directory)?;
        self.effects_started = true;
        self.poisoned = true;
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
        let written = if rewrite {
            native::rewrite(
                directory,
                segment,
                &indices,
                &line,
                source.session_id(),
                now,
                budget,
            )?
        } else {
            native::append(directory, segment, &line)?
        };
        advanced.verify(directory)?;
        if rewrite {
            let mut offset = 0;
            for tombstones in [true, false] {
                for index in &mut indices {
                    if index.mode == Mode::Drop || (index.mode == Mode::Tombstone) != tombstones {
                        continue;
                    }
                    index.offset = offset;
                    index.len = index.output_len - 1;
                    index.evicted = false;
                    offset += u64::from(index.output_len);
                }
            }
            indices.retain(|i| i.mode != Mode::Drop);
        }
        indices.push(Index {
            offset: (new_segment_bytes - line.len() - 1) as u64,
            sequence,
            len: line.len() as u32,
            output_len: line.len() as u32 + 1,
            mode: Mode::Observation,
            evicted: false,
            accepted: now,
        });
        self.inventory.lines = self.inventory.lines - self.indices.len() + new_segment_lines;
        self.inventory.bytes = self.inventory.bytes - old_segment_bytes - old_counter_bytes
            + new_segment_bytes
            + new_counter_bytes;
        self.inventory.segments += usize::from(self.segment.original.is_none());
        self.inventory.sequence = Some(sequence);
        self.segment = written;
        self.counter = advanced;
        self.indices = indices;
        self.poisoned = false;
        // The original full file-job reservation spans all actual effects.
        Ok(RecordOutcome::Accepted(observation))
    }
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
#[derive(Clone)]
struct Index {
    offset: u64,
    sequence: u64,
    len: u32,
    output_len: u32,
    mode: Mode,
    evicted: bool,
    accepted: DateTime<Utc>,
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
fn check_projected(total: usize, removed: usize, added: usize, limit: usize) -> Result<()> {
    let retained = total.checked_sub(removed).ok_or(StoreError::Corrupt)?;
    retained
        .checked_add(added)
        .filter(|projected| *projected <= limit)
        .map(|_| ())
        .ok_or(StoreError::TooLarge)
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

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod batch_tests;
