#![forbid(unsafe_code)]
//! Journal event types and record kind identifiers.
//!
//! The event domain is split across focused submodules:
//!
//! - **`outcome`** — terminal action outcome enum
//! - **`variant`** — the `JournalEvent` sum type
//! - **`kind`** — MRWE5 proof-seam kind classification
//! - **`access`** — field extractors (`run_id`, `seq`, `record_kind`, `attempt`, etc.)
//! - **`valid`** — structural validity checks and slot-value decoding

mod outcome;
pub mod variant;
mod kind;
mod access;
mod valid;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use outcome::DurableActionOutcome;
pub use variant::JournalEvent;
pub use kind::JournalEventKindClass;
