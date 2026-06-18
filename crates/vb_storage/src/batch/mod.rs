#![forbid(unsafe_code)]
//! Atomic cross-keyspace write batch backed by Fjall.
//!
//! Accumulates writes across multiple keyspaces and commits them
//! atomically with a single WAL fsync.

// ── Domain type submodules ──────────────────────────────────────────────────

mod types;
mod write;

// ── Public re-exports ───────────────────────────────────────────────────────

pub use self::types::{BatchByteLimit, BatchState, DEFAULT_JOURNAL_BATCH_BYTE_LIMIT};
pub use self::write::JournalWriteBatch;

#[cfg(test)]
use crate::{
    constants::MAX_BATCH_COUNT,
    error::JournalError,
    records::{RunHeaderRecord, WorkflowSourceRecord},
};

// ── Test module declarations ────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "byte_accounting_tests.rs"]
mod byte_accounting_tests;
