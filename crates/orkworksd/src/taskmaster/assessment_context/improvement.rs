//! Progressive range reads reuse the assessment owner's catalog and file policy.
use super::{digest, open_evidence_file, validate_evidence_text, RepositoryContext};
use crate::taskmaster::{improvement::model::MAX_RANGE_BYTES, runtime::ContextLevel};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};

const MAX_FILES: usize = 32;
const MAX_RANGES: usize = 32;
const MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RangeFact {
    pub path: String,
    pub offset: u64,
    pub file_size: u64,
    pub sha256: String,
    pub text: String,
}
impl RangeFact {
    pub(crate) fn complete(&self) -> bool {
        self.offset == 0 && self.text.len() as u64 == self.file_size
    }
    pub(crate) fn identity(&self) -> Result<String, String> {
        let value = (&self.path, self.offset, self.file_size, &self.text);
        Ok(digest(
            &serde_json::to_vec(&value).map_err(|_| "Invalid range identity")?,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RangeIdentity {
    pub path: String,
    pub offset: u64,
    pub bytes: usize,
    pub file_size: u64,
    pub sha256: String,
}
impl From<&RangeFact> for RangeIdentity {
    fn from(f: &RangeFact) -> Self {
        Self {
            path: f.path.clone(),
            offset: f.offset,
            bytes: f.text.len(),
            file_size: f.file_size,
            sha256: f.sha256.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProgressiveEvidence {
    context: RepositoryContext,
    facts: Vec<RangeFact>,
    check: Arc<dyn Fn() -> Result<(), String> + Send + Sync>,
}
impl ProgressiveEvidence {
    pub(crate) fn collect(
        root: &Path,
        level: ContextLevel,
        excluded: &[String],
        observed_at: &str,
    ) -> Result<Self, String> {
        Self::collect_controlled(root, level, excluded, observed_at, || Ok(()))
    }
    pub(crate) fn collect_controlled(
        root: &Path,
        level: ContextLevel,
        excluded: &[String],
        observed_at: &str,
        check: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) -> Result<Self, String> {
        // The same ephemeral guard follows all seed, range and revalidation reads.
        let check = Arc::new(check);
        let context = RepositoryContext::collect_for_readiness_controlled(
            root,
            level,
            excluded,
            observed_at,
            check.as_ref(),
        )?;
        let mut result = Self {
            context,
            facts: Vec::new(),
            check,
        };
        // Initial reads count in the same range/file/byte accounting as follow-ups.
        for seed in result.context.manifest.files.clone() {
            let fact = result.read_range(&seed.path, 0, MAX_RANGE_BYTES)?;
            result.insert(fact)?;
        }
        result.revalidate()?;
        Ok(result)
    }
    pub(crate) fn range_manifest(&self) -> Vec<RangeIdentity> {
        self.facts.iter().map(RangeIdentity::from).collect()
    }
    pub(crate) fn restore(
        root: &Path,
        level: ContextLevel,
        excluded: &[String],
        observed_at: &str,
        catalog_digest: &str,
        ranges: &[RangeIdentity],
    ) -> Result<Self, String> {
        if ranges.len() > MAX_RANGES
            || ranges.iter().any(|f| f.bytes > MAX_RANGE_BYTES)
            || ranges.iter().map(|f| f.bytes).sum::<usize>() > MAX_BYTES
        {
            return Err("Saved improvement evidence exceeds limits".into());
        }
        let mut result = Self::collect(root, level, excluded, observed_at)?;
        if result.catalog_digest() != catalog_digest {
            return Err("Improvement catalog changed".into());
        }
        for range in ranges {
            let id = result
                .catalog()
                .iter()
                .position(|p| p == &range.path)
                .ok_or("Saved improvement file is unavailable")?;
            let actual = if range.bytes == 0 && range.file_size == 0 && range.offset == 0 {
                result.read_range(&range.path, 0, 0)?
            } else {
                result.read(id, range.offset, range.bytes)?
            };
            if RangeIdentity::from(&actual) != *range {
                return Err("Saved improvement evidence changed".into());
            }
        }
        if result.range_manifest() != ranges {
            return Err("Saved improvement evidence set changed".into());
        }
        Ok(result)
    }
    pub(crate) fn catalog(&self) -> &[String] {
        &self.context.catalog
    }
    pub(crate) fn facts(&self) -> &[RangeFact] {
        &self.facts
    }
    pub(crate) fn catalog_complete(&self) -> bool {
        self.context.manifest.catalog_complete
    }
    pub(crate) fn catalog_digest(&self) -> &str {
        &self.context.manifest.catalog_digest
    }
    pub(crate) fn instruction_coverage_complete(&self) -> bool {
        self.context.manifest.instruction_coverage_complete
    }
    pub(crate) fn read(
        &mut self,
        file_id: usize,
        offset: u64,
        length: usize,
    ) -> Result<RangeFact, String> {
        if length == 0 || length > MAX_RANGE_BYTES || offset.checked_add(length as u64).is_none() {
            return Err("Invalid improvement range bounds".into());
        }
        self.revalidate()?;
        let target = self
            .context
            .catalog
            .get(file_id)
            .cloned()
            .ok_or("Unknown improvement file ID")?;
        let mut next = self.clone();
        // Admit a target together with all its governing instructions, never partially.
        for path in next
            .context
            .instruction_closure(std::slice::from_ref(&target))?
        {
            let (start, size) = if path == target {
                (offset, length)
            } else {
                (0, MAX_RANGE_BYTES)
            };
            let fact = next.read_range(&path, start, size)?;
            next.insert(fact)?;
        }
        let fact = next
            .facts
            .iter()
            .find(|f| {
                f.path == target
                    && f.offset == offset
                    && f.text.len() as u64
                        == (f.file_size.saturating_sub(offset)).min(length as u64)
            })
            .cloned()
            .ok_or("Improvement range was not admitted")?;
        next.revalidate()?;
        *self = next;
        Ok(fact)
    }
    fn insert(&mut self, fact: RangeFact) -> Result<(), String> {
        if self.facts.contains(&fact) {
            return Ok(());
        }
        let mut paths = self
            .facts
            .iter()
            .map(|f| f.path.as_str())
            .collect::<HashSet<_>>();
        paths.insert(&fact.path);
        if self.facts.len() >= MAX_RANGES
            || paths.len() > MAX_FILES
            || self.facts.iter().map(|f| f.text.len()).sum::<usize>() + fact.text.len() > MAX_BYTES
        {
            return Err("Improvement evidence limit reached".into());
        }
        self.facts.push(fact);
        Ok(())
    }
    fn read_range(&self, path: &str, offset: u64, length: usize) -> Result<RangeFact, String> {
        (self.check)()?;
        let mut file = open_evidence_file(
            &self.context.root,
            path,
            self.context.level,
            &self.context.excluded,
        )?;
        let metadata = file
            .metadata()
            .map_err(|_| "Improvement evidence is unavailable")?;
        if !metadata.is_file() || offset > metadata.len() {
            return Err("Invalid improvement range".into());
        }
        // Inspect boundary overlap privately so a credential marker split across requests cannot bypass the policy.
        let inspection_start = offset.saturating_sub(32);
        let inspection_length = length
            .checked_add((offset - inspection_start) as usize)
            .and_then(|n| n.checked_add(32))
            .ok_or("Invalid improvement range")?;
        (self.check)()?;
        file.seek(SeekFrom::Start(inspection_start))
            .map_err(|_| "Improvement evidence cannot be read")?;
        let mut inspected = Vec::new();
        (&mut file)
            .take(inspection_length as u64)
            .read_to_end(&mut inspected)
            .map_err(|_| "Improvement evidence cannot be read")?;
        // UTF-8 boundary overlap may start inside a character. Scan sensitive ASCII markers separately, then validate returned text strictly.
        let scan = String::from_utf8_lossy(&inspected);
        validate_evidence_text(scan.as_bytes())?;
        let start = (offset - inspection_start) as usize;
        let end = start.saturating_add(length).min(inspected.len());
        let bytes = inspected
            .get(start..end)
            .ok_or("Invalid improvement range")?;
        validate_evidence_text(bytes)?;
        let after = file
            .metadata()
            .map_err(|_| "Improvement evidence is unavailable")?;
        if metadata.len() != after.len() || metadata.modified().ok() != after.modified().ok() {
            return Err("Improvement evidence changed during its read".into());
        }
        let text = std::str::from_utf8(bytes)
            .map_err(|_| "Improvement range is not UTF-8 text")?
            .to_owned();
        let mut fact = RangeFact {
            path: path.into(),
            offset,
            file_size: metadata.len(),
            sha256: String::new(),
            text,
        };
        fact.sha256 = fact.identity()?;
        Ok(fact)
    }
    pub(crate) fn revalidate(&self) -> Result<(), String> {
        let current = RepositoryContext::collect_for_readiness_controlled(
            &self.context.root,
            self.context.level,
            &self.context.excluded,
            &self.context.observed_at,
            self.check.as_ref(),
        )?;
        if current.manifest != self.context.manifest {
            return Err("Improvement repository inputs changed".into());
        }
        for fact in &self.facts {
            let actual = self.read_range(&fact.path, fact.offset, fact.text.len())?;
            if actual != *fact {
                return Err("Improvement repository evidence changed".into());
            }
        }
        Ok(())
    }
}
