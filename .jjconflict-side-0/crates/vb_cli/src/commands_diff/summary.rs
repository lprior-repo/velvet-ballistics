#![forbid(unsafe_code)]
//! Human-readable per-event summary text.
//!
//! Owns [`DIGEST_SHORT_LEN`], [`digest_short`], and [`summary_text`].
//! The output format of [`summary_text`] is pinned by exact-string tests
//! in `commands_diff/tests.rs`.

use vb_core::WorkflowDigest;
use vb_storage::events::JournalEvent;

/// Number of hex chars emitted by [`digest_short`].
#[allow(dead_code)] // exercised by the `summary_text_for_*` unit tests
const DIGEST_SHORT_LEN: usize = 8;

/// Render the leading 4 bytes of a `WorkflowDigest` as 8 lowercase hex chars.
///
/// Used by `summary_text` to keep the `RunAdmission` summary on one short
/// line while remaining human-readable for diff display. Truncation is
/// deliberate — the full digest is still emitted by
/// [`super::diff::diff_event_summary`] through the upstream JSON projection.
#[allow(dead_code)] // exercised by the `summary_text_for_*` unit tests
#[must_use]
fn digest_short(digest: &WorkflowDigest) -> String {
    let bytes = digest.as_bytes();
    let mut out = String::with_capacity(DIGEST_SHORT_LEN);
    // Hex-encode the first 4 bytes by hand. `write!` to a `String` only
    // fails on allocator failure, which we cannot recover from, and the
    // manual loop is easier to audit than a `format_args!` macro that
    // produces a `Result` we would otherwise have to discard.
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes.iter().take(4) {
        // `byte & 0x0f` and `byte >> 4` are both in `0..16` because `byte: u8`,
        // so the `HEX[hi as usize]` / `HEX[lo as usize]` index is statically
        // bounded. `usize::from(u8)` is the canonical widening conversion and
        // is not flagged by `clippy::as_conversions`.
        let hi = usize::from((byte >> 4) & 0x0f);
        let lo = usize::from(byte & 0x0f);
        #[allow(clippy::indexing_slicing)]
        let hi_ch = HEX[hi];
        #[allow(clippy::indexing_slicing)]
        let lo_ch = HEX[lo];
        out.push(char::from(hi_ch));
        out.push(char::from(lo_ch));
    }
    out
}

/// Produce a short, human-readable summary for a single event.
///
/// The output format is pinned by the per-variant exact-string tests in
/// `commands_diff/tests.rs`; if the format changes for any of the modern
/// variants (`ActionScheduledTicket`, `ActionAbandoned`, `WaitResolved`,
/// `AskTimedOut`, `RunKilled`, `RunAdmission`) those tests will fail with
/// an exact-equal mismatch. New `KnownVariant` arms must be added here in
/// lockstep with [`super::diff::diff_event_summary`].
#[allow(clippy::too_many_lines, dead_code)] // exercised by the `summary_text_for_*` unit tests
pub fn summary_text(event: &JournalEvent) -> String {
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            format!("run accepted at seq {}", seq.get())
        }
        JournalEvent::RunAdmission {
            artifact_digest, ..
        } => {
            format!("run admitted (artifact={})", digest_short(artifact_digest))
        }
        JournalEvent::StepStarted { step, .. } => {
            format!("step started at step {}", step.get())
        }
        JournalEvent::StepSucceeded { step, .. } => {
            format!("step succeeded at step {}", step.get())
        }
        JournalEvent::ActionScheduled { action, .. } => {
            format!("action scheduled for action {}", action.get())
        }
        JournalEvent::ActionCompletedEvent { action, .. } => {
            format!("action completed for action {}", action.get())
        }
        JournalEvent::ActionScheduledTicket { ticket, .. } => {
            format!("scheduled ticket for action {}", ticket.action.get())
        }
        JournalEvent::ActionCompletedEnvelope { ticket, .. } => {
            format!(
                "action completed envelope for action {}",
                ticket.action.get()
            )
        }
        JournalEvent::ActionFailedEvent { action, .. } => {
            format!("action failed for action {}", action.get())
        }
        JournalEvent::ActionAbandoned { ticket, .. } => {
            format!("action abandoned (capacity={})", ticket.capacity)
        }
        JournalEvent::SlotWrittenEvent { slot, .. } => {
            format!("slot written at slot {}", slot.get())
        }
        JournalEvent::WaitScheduledEvent { step, .. } => {
            format!("wait scheduled at step {}", step.get())
        }
        JournalEvent::AskScheduledEvent { step, .. } => {
            format!("ask scheduled at step {}", step.get())
        }
        JournalEvent::AskAnsweredEvent { step, .. } => {
            format!("ask answered at step {}", step.get())
        }
        JournalEvent::WaitResolvedEvent { step, .. } => {
            format!("wait resolved at step {}", step.get())
        }
        JournalEvent::RetryScheduledEvent { step, .. } => {
            format!("retry scheduled at step {}", step.get())
        }
        JournalEvent::RunCancelled { .. } => String::from("run cancelled"),
        JournalEvent::RunKilled { seq, .. } => {
            format!("run killed (seq={})", seq.get())
        }
        JournalEvent::RunFinished { .. } => String::from("run finished"),
        JournalEvent::RunFailedEvent { .. } => String::from("run failed"),
        JournalEvent::RunResumed { .. } => String::from("run resumed"),
        JournalEvent::RunRetried { .. } => String::from("run retried"),
        JournalEvent::RunAnswered { slot_idx, .. } => {
            format!("run answered at slot {}", slot_idx.get())
        }
        JournalEvent::AskTimedOutEvent { step, .. } => {
            format!("ask timed out at step {}", step.get())
        }
        _ => String::from("unknown variant"),
    }
}
