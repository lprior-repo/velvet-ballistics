#![forbid(unsafe_code)]
mod action_index;
mod append_event;
mod artifact_sequence;
mod commit;
mod emergency_deny;
mod inputs;
mod putters;
mod types;
mod writer_queue;
pub use types::{DEFAULT_JOURNAL_BATCH_BYTE_LIMIT, JournalWriteBatch};
pub use writer_queue::{JournalWriterQueue, MAX_BATCH_COUNT};
#[cfg(test)]
#[path = "artifact_sequence_test_support.rs"]
mod artifact_sequence_test_support;
#[cfg(test)]
#[path = "artifact_sequence_tests.rs"]
mod artifact_sequence_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
