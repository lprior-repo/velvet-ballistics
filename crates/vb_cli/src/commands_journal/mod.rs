#![forbid(unsafe_code)]
//! Pure journal analysis logic for trace, retry, resume, and answer commands.
//!
//! All functions in this module are pure: they accept `&[JournalEvent]` and
//! return structured data. No I/O, no formatting, no side effects.
//!
//! Sub-modules are organized by command domain so each file stays under the
//! 300-line source cap.

// Trace domain lives in `trace.rs` to keep this file small. The `pub(crate)`
// items below are re-exported so callers use `crate::commands_journal::*`
//! exactly as before the refactor.
mod trace;
pub(crate) use self::trace::{
    TraceEntry, TraceFilters, TraceStatus, build_trace, event_status, filter_events, filter_trace,
};

// Retry domain: analysis of whether a run can be retried.
mod retry;
pub(crate) use self::retry::{RetryAnalysis, analyze_retry};

// Resume domain: analysis of whether a suspended run can be resumed.
mod resume;
pub(crate) use self::resume::{ResumeAnalysis, analyze_resume};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
