#![forbid(unsafe_code)]
mod append_event;
mod commit;
mod putters;
mod types;
mod writer_queue;
pub use types::{DEFAULT_JOURNAL_BATCH_BYTE_LIMIT, JournalWriteBatch};
pub use writer_queue::{JournalWriterQueue, MAX_BATCH_COUNT};
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
