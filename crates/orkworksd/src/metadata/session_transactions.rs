//! Durable revisions and the storage boundary for the versioned metadata API.
//! Legacy snapshot writers remain unconditional; they still advance the revision.
//! This module does not enable native clearing or coordinate external file writes.

use super::{normalize_session_metadata, write_atomic, MetadataStore, SessionMetadata};
use serde::{Deserialize, Serialize};
use std::{fs, io};

/// Opaque to callers; only this module interprets or advances the counter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct MetadataRevision(u64);

impl TryFrom<String> for MetadataRevision {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let revision = value
            .parse::<u64>()
            .map_err(|_| "invalid metadata revision")?;
        if revision.to_string() != value {
            return Err("invalid metadata revision");
        }
        Ok(Self(revision))
    }
}

impl From<MetadataRevision> for String {
    fn from(value: MetadataRevision) -> Self {
        value.0.to_string()
    }
}

impl MetadataRevision {
    fn next(&self) -> io::Result<Self> {
        self.0.checked_add(1).map(Self).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "metadata revision exhausted")
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RevisionedSession {
    #[serde(flatten)]
    pub(crate) metadata: SessionMetadata,
    #[serde(rename = "metadataRevision", default)]
    pub(crate) revision: MetadataRevision,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WriteScope {
    pub(crate) attention: bool,
    pub(crate) work: bool,
}

impl WriteScope {
    pub(crate) const ALL: Self = Self {
        attention: true,
        work: true,
    };
    pub(crate) const ATTENTION: Self = Self {
        attention: true,
        work: false,
    };
    pub(crate) const WORK: Self = Self {
        attention: false,
        work: true,
    };
    pub(crate) const NONE: Self = Self {
        attention: false,
        work: false,
    };
}

/// Storage errors remain distinct so the future HTTP adapter can distinguish
/// revision conflicts from missing/unreadable records without guessing.
#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))] // The authenticated API follows in #788.
pub(crate) enum SessionTransactionError {
    Conflict,
    Io(io::Error),
}

impl From<io::Error> for SessionTransactionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for SessionTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => f.write_str("metadata revision conflict"),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SessionTransactionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Conflict => None,
            Self::Io(error) => Some(error),
        }
    }
}

impl MetadataStore {
    /// Read without quarantining or rewriting. Missing, malformed, and unreadable
    /// records are distinct outcomes; legacy records have revision zero.
    pub(crate) fn try_read_revisioned_session(
        &self,
        id: &str,
    ) -> io::Result<Option<RevisionedSession>> {
        let path = self.sessions_dir().join(format!("{id}.json"));
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut record: RevisionedSession = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if record.metadata.id != id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "session identity mismatch",
            ));
        }
        record.metadata = normalize_session_metadata(record.metadata);
        Ok(Some(record))
    }

    /// Check and mutate the latest record while holding the same per-session
    /// lock as owned native clears, from read through atomic replacement.
    ///
    /// The mutation must not reenter this store for the same session. It may
    /// modify fields but cannot change session identity. Any error leaves the
    /// durable record, revision, and ownership unchanged. External filesystem
    /// replacements and legacy read/modify/write callers are not transactions.
    #[cfg_attr(not(test), allow(dead_code))] // Consumed by the API in #788.
    pub(crate) fn try_update_session(
        &self,
        id: &str,
        expected_revision: &MetadataRevision,
        mutate: impl FnOnce(&mut SessionMetadata) -> io::Result<()>,
    ) -> Result<RevisionedSession, SessionTransactionError> {
        self.try_update_session_scoped(id, expected_revision, WriteScope::ALL, mutate)
    }

    pub(crate) fn try_update_session_scoped(
        &self,
        id: &str,
        expected_revision: &MetadataRevision,
        scope: WriteScope,
        mutate: impl FnOnce(&mut SessionMetadata) -> io::Result<()>,
    ) -> Result<RevisionedSession, SessionTransactionError> {
        self.update_session_locked(id, Some(expected_revision), scope, mutate)
    }

    /// Internal producers read and arbitrate the latest record inside this
    /// closure. It shares the same lane as versioned API and native writes.
    pub(crate) fn mutate_session(
        &self,
        id: &str,
        scope: WriteScope,
        mutate: impl FnOnce(&mut SessionMetadata) -> io::Result<()>,
    ) -> Result<RevisionedSession, SessionTransactionError> {
        self.update_session_locked(id, None, scope, mutate)
    }

    fn update_session_locked(
        &self,
        id: &str,
        expected_revision: Option<&MetadataRevision>,
        scope: WriteScope,
        mutate: impl FnOnce(&mut SessionMetadata) -> io::Result<()>,
    ) -> Result<RevisionedSession, SessionTransactionError> {
        self.with_session_write_lock(id, || {
            let mut record = self
                .try_read_revisioned_session(id)?
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "session record missing"))?;
            if expected_revision.is_some_and(|expected| &record.revision != expected) {
                return Err(SessionTransactionError::Conflict);
            }
            // Check exhaustion before invoking a mutation with no chance of commit.
            let next_revision = record.revision.next()?;
            mutate(&mut record.metadata)?;
            if record.metadata.id != id {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "session identity changed",
                )
                .into());
            }
            let now = crate::workspace_runtime::iso_now();
            if scope.attention {
                record.metadata.provenance.attention_updated_at = Some(now.clone());
            }
            if scope.work {
                record.metadata.provenance.work_metadata_updated_at = Some(now);
            }
            record.revision = next_revision;
            let json = serde_json::to_string_pretty(&record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            let path = self.sessions_dir().join(format!("{id}.json"));
            write_atomic(&path, &json)?;
            if scope.attention {
                self.accept_session_write(id);
            }
            Ok(record)
        })
    }

    // Called only while the session's write lock is held, including native
    // ownership checks. Never derive a revision from the caller's snapshot.
    pub(super) fn prepare_session_replacement(&self, meta: &SessionMetadata) -> io::Result<String> {
        let revision = self
            .try_read_revisioned_session(&meta.id)?
            .map(|record| record.revision)
            .unwrap_or_default()
            .next()?;
        serde_json::to_string_pretty(&RevisionedSession {
            metadata: meta.clone(),
            revision,
        })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}
