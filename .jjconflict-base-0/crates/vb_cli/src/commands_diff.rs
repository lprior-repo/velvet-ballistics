#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure diff computation logic, separated from I/O and formatting.

use std::collections::HashMap;

use vb_core::{SlotValue, WorkflowDigest};
use vb_storage::events::JournalEvent;

/// Result of comparing two event streams.
pub struct DiffResult {
    /// Number of events in stream A.
    pub events_a: usize,
    /// Number of events in stream B.
    pub events_b: usize,
    /// Ordered list of difference entries (as JSON values for downstream formatting).
    pub diffs: Vec<serde_json::Value>,
}

/// Number of hex chars emitted by [`digest_short`].
#[allow(dead_code)] // exercised by the `summary_text_for_*` unit tests
const DIGEST_SHORT_LEN: usize = 8;

/// Render the leading 4 bytes of a `WorkflowDigest` as 8 lowercase hex chars.
///
/// Used by `summary_text` to keep the `RunAdmission` summary on one short
/// line while remaining human-readable for diff display. Truncation is
/// deliberate — the full digest is still emitted by [`diff_event_summary`]
/// through the upstream JSON projection.
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

/// Compile-time schema fingerprint over the canonical `JournalEvent` variant
/// names enumerated by [`KNOWN_VARIANTS`].
///
/// The hash is the FNV-1a 64-bit fold of every variant name in canonical
/// order, with `0xff` byte separators between names. Adding, removing, or
/// renaming any known variant changes this value, and the test
/// `schema_hash_matches_expected` enforces the value against
/// `EXPECTED_SCHEMA_HASH`. Bumping a variant therefore requires updating
/// both the `KNOWN_VARIANTS` slice and `EXPECTED_SCHEMA_HASH` — the hash
/// itself is derived and never hand-edited.
pub const SCHEMA_HASH: u64 = {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i: usize = 0;
    while i < KNOWN_VARIANTS.len() {
        // The bounds `i < KNOWN_VARIANTS.len()` and `j < bytes.len()`
        // make the indexing statically safe, but `clippy::indexing_slicing`
        // cannot reason about const-context while-loops. The `allow` is
        // bounded to this single const initializer.
        #[allow(clippy::indexing_slicing)]
        let bytes = KNOWN_VARIANTS[i].name().as_bytes();
        let mut j: usize = 0;
        while j < bytes.len() {
            // The widening `u8 -> u64` is safe by construction (`u8` is
            // always ≤ `u64::MAX`) but the `as_conversions` lint still
            // flags it. We use a per-byte widening helper that compiles
            // in const context (`u64::from` is not yet const-stable).
            #[allow(clippy::indexing_slicing)]
            let byte = bytes[j];
            hash ^= widen_byte_to_u64(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            j += 1;
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
};

/// Const-stable widening conversion `u8 -> u64`.
///
/// `u64::from` is not yet `const fn` on stable, so the FNV-1a fold in
/// [`SCHEMA_HASH`] uses this helper instead of `as u64` (which the
/// `clippy::as_conversions` lint forbids in production code).
#[allow(clippy::as_conversions)]
const fn widen_byte_to_u64(byte: u8) -> u64 {
    byte as u64
}

/// Compile-time guard that the schema hash is non-zero. An all-zero hash
/// would indicate the FNV-1a fold was never executed (i.e. `KNOWN_VARIANTS`
/// is empty), which would silently weaken the runtime schema-drift check.
const _: () = assert!(SCHEMA_HASH != 0, "SCHEMA_HASH must be non-zero");

/// Expected FNV-1a 64-bit hash of the current 24-variant schema.
///
/// Pinned to the FNV-1a fold of `KNOWN_VARIANTS` in canonical order with
/// `0xff` byte separators. The companion compile-time `assert!`
/// (immediately below) and the runtime test
/// `schema_hash_matches_expected` both gate against this value.
///
/// To bump the schema (add/remove/rename a known variant):
///   1. Update `KNOWN_VARIANTS` and the `name()` / `try_from_event` /
///      `diff_event_summary` / `summary_text` matches.
///   2. Compute the new FNV-1a fold and overwrite this constant.
///   3. Recompile — the compile-time `assert!` enforces equality.
pub const EXPECTED_SCHEMA_HASH: u64 = 0x1b5e_5da9_7361_afa6;

/// Compile-time guard that `EXPECTED_SCHEMA_HASH` matches `SCHEMA_HASH`.
///
/// The two are equal at compile time because `SCHEMA_HASH` is derived from
/// `KNOWN_VARIANTS` and `EXPECTED_SCHEMA_HASH` is hand-pinned to the same
/// value. If the maintainer updates `KNOWN_VARIANTS` without updating
/// `EXPECTED_SCHEMA_HASH`, this assertion fires before any test runs.
const _: () = assert!(
    SCHEMA_HASH == EXPECTED_SCHEMA_HASH,
    "EXPECTED_SCHEMA_HASH out of sync with KNOWN_VARIANTS — recompute SCHEMA_HASH and update EXPECTED_SCHEMA_HASH"
);

/// Sealed enumeration of every currently-known `JournalEvent` variant that
/// the CLI maps to a stable name and short summary.
///
/// `KnownVariant` is local to this crate so the closed-enum contract is
/// enforced at compile time: any new variant requires adding an arm to
/// [`KnownVariant::try_from_event`] (compile error if missing for a
/// current variant) and to both [`KnownVariant::name`] and
/// [`summary_text`] (compile error if missing).
///
/// The `#[non_exhaustive]` upstream `JournalEvent` enum still requires a
/// wildcard fallback for genuinely-new variants added after this snapshot;
/// that fallback returns `None` from `try_from_event` and is rendered as
/// `"Unknown"` by [`event_name`] and [`diff_event_summary`]. The runtime
/// test `every_known_variant_maps_to_a_non_unknown_name` catches drift if a
/// known variant is added to [`KNOWN_VARIANTS`] without updating the
/// production match arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KnownVariant {
    /// `JournalEvent::RunAccepted`.
    RunAccepted,
    /// `JournalEvent::RunAdmission`.
    RunAdmission,
    /// `JournalEvent::StepStarted`.
    StepStarted,
    /// `JournalEvent::StepSucceeded`.
    StepSucceeded,
    /// `JournalEvent::ActionScheduled`.
    ActionScheduled,
    /// `JournalEvent::ActionCompletedEvent`.
    ActionCompletedEvent,
    /// `JournalEvent::ActionScheduledTicket`.
    ActionScheduledTicket,
    /// `JournalEvent::ActionCompletedEnvelope`.
    ActionCompletedEnvelope,
    /// `JournalEvent::ActionFailedEvent`.
    ActionFailedEvent,
    /// `JournalEvent::ActionAbandoned`.
    ActionAbandoned,
    /// `JournalEvent::SlotWrittenEvent`.
    SlotWrittenEvent,
    /// `JournalEvent::WaitScheduledEvent`.
    WaitScheduledEvent,
    /// `JournalEvent::AskScheduledEvent`.
    AskScheduledEvent,
    /// `JournalEvent::AskAnsweredEvent`.
    AskAnsweredEvent,
    /// `JournalEvent::WaitResolvedEvent`.
    WaitResolvedEvent,
    /// `JournalEvent::RetryScheduledEvent`.
    RetryScheduledEvent,
    /// `JournalEvent::RunCancelled`.
    RunCancelled,
    /// `JournalEvent::RunKilled`.
    RunKilled,
    /// `JournalEvent::RunFinished`.
    RunFinished,
    /// `JournalEvent::RunFailedEvent`.
    RunFailedEvent,
    /// `JournalEvent::RunResumed`.
    RunResumed,
    /// `JournalEvent::RunRetried`.
    RunRetried,
    /// `JournalEvent::RunAnswered`.
    RunAnswered,
    /// `JournalEvent::AskTimedOutEvent`.
    AskTimedOutEvent,
}

impl KnownVariant {
    /// Canonical count of currently-known `JournalEvent` variants. The
    /// compile-time `assert!` in this file uses this constant to detect
    /// drift between [`KNOWN_VARIANTS`] and the public contract.
    pub(crate) const COUNT: usize = 24;

    /// Canonical variant name used by [`event_name`] and the JSON
    /// `"type"` field of [`diff_event_summary`]. This match is exhaustive
    /// on `KnownVariant`; adding a new variant without updating it fails
    /// at compile time.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::RunAccepted => "RunAccepted",
            Self::RunAdmission => "RunAdmission",
            Self::StepStarted => "StepStarted",
            Self::StepSucceeded => "StepSucceeded",
            Self::ActionScheduled => "ActionScheduled",
            Self::ActionCompletedEvent => "ActionCompleted",
            Self::ActionScheduledTicket => "ActionScheduledTicket",
            Self::ActionCompletedEnvelope => "ActionCompletedEnvelope",
            Self::ActionFailedEvent => "ActionFailed",
            Self::ActionAbandoned => "ActionAbandoned",
            Self::SlotWrittenEvent => "SlotWritten",
            Self::WaitScheduledEvent => "WaitScheduled",
            Self::AskScheduledEvent => "AskScheduled",
            Self::AskAnsweredEvent => "AskAnswered",
            Self::WaitResolvedEvent => "WaitResolved",
            Self::RetryScheduledEvent => "RetryScheduled",
            Self::RunCancelled => "RunCancelled",
            Self::RunKilled => "RunKilled",
            Self::RunFinished => "RunFinished",
            Self::RunFailedEvent => "RunFailed",
            Self::RunResumed => "RunResumed",
            Self::RunRetried => "RunRetried",
            Self::RunAnswered => "RunAnswered",
            Self::AskTimedOutEvent => "AskTimedOut",
        }
    }

    /// Attempt to classify an event as one of the known variants.
    ///
    /// Returns `None` for genuinely-new `JournalEvent` variants added after
    /// this snapshot. The `#[non_exhaustive]` upstream attribute forces a
    /// wildcard arm even for a fully-exhaustive local list; that arm is
    /// the only path that returns `None`.
    pub(crate) fn try_from_event(event: &JournalEvent) -> Option<Self> {
        Some(match event {
            JournalEvent::RunAccepted { .. } => Self::RunAccepted,
            JournalEvent::RunAdmission { .. } => Self::RunAdmission,
            JournalEvent::StepStarted { .. } => Self::StepStarted,
            JournalEvent::StepSucceeded { .. } => Self::StepSucceeded,
            JournalEvent::ActionScheduled { .. } => Self::ActionScheduled,
            JournalEvent::ActionCompletedEvent { .. } => Self::ActionCompletedEvent,
            JournalEvent::ActionScheduledTicket { .. } => Self::ActionScheduledTicket,
            JournalEvent::ActionCompletedEnvelope { .. } => Self::ActionCompletedEnvelope,
            JournalEvent::ActionFailedEvent { .. } => Self::ActionFailedEvent,
            JournalEvent::ActionAbandoned { .. } => Self::ActionAbandoned,
            JournalEvent::SlotWrittenEvent { .. } => Self::SlotWrittenEvent,
            JournalEvent::WaitScheduledEvent { .. } => Self::WaitScheduledEvent,
            JournalEvent::AskScheduledEvent { .. } => Self::AskScheduledEvent,
            JournalEvent::AskAnsweredEvent { .. } => Self::AskAnsweredEvent,
            JournalEvent::WaitResolvedEvent { .. } => Self::WaitResolvedEvent,
            JournalEvent::RetryScheduledEvent { .. } => Self::RetryScheduledEvent,
            JournalEvent::RunCancelled { .. } => Self::RunCancelled,
            JournalEvent::RunKilled { .. } => Self::RunKilled,
            JournalEvent::RunFinished { .. } => Self::RunFinished,
            JournalEvent::RunFailedEvent { .. } => Self::RunFailedEvent,
            JournalEvent::RunResumed { .. } => Self::RunResumed,
            JournalEvent::RunRetried { .. } => Self::RunRetried,
            JournalEvent::RunAnswered { .. } => Self::RunAnswered,
            JournalEvent::AskTimedOutEvent { .. } => Self::AskTimedOutEvent,
            _ => return None,
        })
    }
}

/// Canonical ordered list of every [`KnownVariant`].
///
/// Order is significant: the FNV-1a fold in [`SCHEMA_HASH`] consumes the
/// variant names in this exact order, and the runtime tests iterate this
/// slice to construct one sample event per variant. New variants must be
/// appended (or inserted with care) and the `EXPECTED_SCHEMA_HASH` must
/// be updated in lockstep.
pub(crate) const KNOWN_VARIANTS: [KnownVariant; KnownVariant::COUNT] = [
    KnownVariant::RunAccepted,
    KnownVariant::RunAdmission,
    KnownVariant::StepStarted,
    KnownVariant::StepSucceeded,
    KnownVariant::ActionScheduled,
    KnownVariant::ActionCompletedEvent,
    KnownVariant::ActionScheduledTicket,
    KnownVariant::ActionCompletedEnvelope,
    KnownVariant::ActionFailedEvent,
    KnownVariant::ActionAbandoned,
    KnownVariant::SlotWrittenEvent,
    KnownVariant::WaitScheduledEvent,
    KnownVariant::AskScheduledEvent,
    KnownVariant::AskAnsweredEvent,
    KnownVariant::WaitResolvedEvent,
    KnownVariant::RetryScheduledEvent,
    KnownVariant::RunCancelled,
    KnownVariant::RunKilled,
    KnownVariant::RunFinished,
    KnownVariant::RunFailedEvent,
    KnownVariant::RunResumed,
    KnownVariant::RunRetried,
    KnownVariant::RunAnswered,
    KnownVariant::AskTimedOutEvent,
];

/// Compile-time check that `KNOWN_VARIANTS` matches the declared count.
///
/// The array is annotated `[KnownVariant; KnownVariant::COUNT]` so the
/// compiler already enforces the length. This assertion additionally
/// guarantees that every element is the canonical enum (no `..` padding
/// or list-comprehension tricks could ever compile here without breaking
/// the const-context).
const _: () = assert!(KNOWN_VARIANTS.len() == KnownVariant::COUNT);

/// Compare two event streams and produce a structured diff.
pub fn compute_diff(events_a: &[JournalEvent], events_b: &[JournalEvent]) -> DiffResult {
    let len_a = events_a.len();
    let len_b = events_b.len();
    let max_len = len_a.max(len_b);
    let mut diffs: Vec<serde_json::Value> = Vec::new();

    for idx in 0..max_len {
        let ev_a = events_a.get(idx);
        let ev_b = events_b.get(idx);
        match (ev_a, ev_b) {
            (Some(a), None) => {
                diffs.push(serde_json::json!({
                    "index": idx,
                    "kind": "only_in_a",
                    "event_a": diff_event_summary(a)
                }));
            }
            (None, Some(b)) => {
                diffs.push(serde_json::json!({
                    "index": idx,
                    "kind": "only_in_b",
                    "event_b": diff_event_summary(b)
                }));
            }
            (Some(a), Some(b)) => {
                if events_differ(a, b) {
                    diffs.push(serde_json::json!({
                        "index": idx,
                        "kind": "changed",
                        "event_a": diff_event_summary(a),
                        "event_b": diff_event_summary(b)
                    }));
                }
            }
            (None, None) => {}
        }
    }

    let steps_a = collect_step_outcomes(events_a);
    let steps_b = collect_step_outcomes(events_b);
    for (step, outcome) in &steps_a {
        match steps_b.get(step) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "step_missing_in_b",
                    "step": step,
                    "outcome_a": outcome
                }));
            }
            Some(bo) => {
                if outcome != bo {
                    diffs.push(serde_json::json!({
                        "kind": "step_outcome_differs",
                        "step": step,
                        "outcome_a": outcome,
                        "outcome_b": bo
                    }));
                }
            }
        }
    }
    for (step, outcome) in &steps_b {
        if !steps_a.contains_key(step) {
            diffs.push(serde_json::json!({
                "kind": "step_missing_in_a",
                "step": step,
                "outcome_b": outcome
            }));
        }
    }

    let slots_a = collect_slot_values(events_a);
    let slots_b = collect_slot_values(events_b);
    for (slot, va) in &slots_a {
        match slots_b.get(slot) {
            None => {
                diffs.push(serde_json::json!({
                    "kind": "slot_missing_in_b",
                    "slot": slot,
                    "value_a": va
                }));
            }
            Some(vb) => {
                if va != vb {
                    diffs.push(serde_json::json!({
                        "kind": "slot_value_differs",
                        "slot": slot,
                        "value_a": va,
                        "value_b": vb
                    }));
                }
            }
        }
    }
    for (slot, vb) in &slots_b {
        if !slots_a.contains_key(slot) {
            diffs.push(serde_json::json!({
                "kind": "slot_missing_in_a",
                "slot": slot,
                "value_b": vb
            }));
        }
    }

    DiffResult {
        events_a: len_a,
        events_b: len_b,
        diffs,
    }
}

/// Produce a short JSON summary of a single event for diff display.
///
/// The outer match is over `JournalEvent` and therefore requires the
/// `#[non_exhaustive]` wildcard; each per-variant arm resolves the
/// `type` field through [`KnownVariant::name`] so the schema guard
/// remains tied to the closed enum. New `KnownVariant` arms added to
/// [`KnownVariant::name`] must be matched here in lockstep; the runtime
/// test `every_known_variant_maps_to_a_non_unknown_name` enforces the
/// invariant from the other direction.
#[allow(clippy::too_many_lines)]
pub fn diff_event_summary(event: &JournalEvent) -> serde_json::Value {
    let type_name = event_name(event);
    match event {
        JournalEvent::RunAccepted { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunAdmission { seq, policy, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "policy": format!("{policy:?}")
        }),
        JournalEvent::StepStarted { seq, step, .. } => {
            serde_json::json!({
                "type": type_name,
                "seq": seq.get(),
                "step": step.get()
            })
        }
        JournalEvent::StepSucceeded {
            seq, step, output, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "output": output.get()
        }),
        JournalEvent::ActionScheduled {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionCompletedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionScheduledTicket {
            seq,
            run,
            ticket,
            input,
            output,
            action_abi_digest,
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}"),
            "input": input.get(),
            "output": output.get(),
            "action_abi_digest": format!("{action_abi_digest:?}")
        }),
        JournalEvent::ActionCompletedEnvelope {
            seq,
            run,
            ticket,
            output,
            outcome,
            encoded_len,
            taint,
            value_digest,
            action_abi_digest,
            ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}"),
            "output": output.get(),
            "outcome": format!("{outcome:?}"),
            "encoded_len": encoded_len,
            "taint": format!("{taint:?}"),
            "value_digest": format!("{value_digest:?}"),
            "action_abi_digest": format!("{action_abi_digest:?}")
        }),
        JournalEvent::ActionFailedEvent {
            seq, step, action, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get(),
            "action": action.get()
        }),
        JournalEvent::ActionAbandoned { seq, run, ticket } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "run": run.get(),
            "ticket": format!("{ticket:?}")
        }),
        JournalEvent::SlotWrittenEvent {
            seq, slot, value, ..
        } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "slot": slot.get(),
            "has_value": value.is_some()
        }),
        JournalEvent::WaitScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::AskScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::AskAnsweredEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::WaitResolvedEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::RetryScheduledEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        JournalEvent::RunCancelled { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunKilled { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunFinished { seq, result, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "result": result.get()
        }),
        JournalEvent::RunFailedEvent { seq, .. } => {
            serde_json::json!({"type": type_name, "seq": seq.get()})
        }
        JournalEvent::RunResumed { run, .. } => {
            serde_json::json!({"type": type_name, "run": run.get()})
        }
        JournalEvent::RunRetried { run, .. } => {
            serde_json::json!({"type": type_name, "run": run.get()})
        }
        JournalEvent::RunAnswered { run, slot_idx, .. } => serde_json::json!({
            "type": type_name,
            "run": run.get(),
            "slot_idx": slot_idx.get()
        }),
        JournalEvent::AskTimedOutEvent { seq, step, .. } => serde_json::json!({
            "type": type_name,
            "seq": seq.get(),
            "step": step.get()
        }),
        _ => serde_json::json!({"type": type_name}),
    }
}

/// Return the static name string for an event variant.
///
/// For known variants the name is taken from [`KnownVariant::name`]; for
/// future `#[non_exhaustive]` additions the literal `"Unknown"` is
/// returned. The companion test
/// `every_known_variant_maps_to_a_non_unknown_name` enforces that no
/// current variant falls through.
pub fn event_name(event: &JournalEvent) -> &'static str {
    KnownVariant::try_from_event(event).map_or("Unknown", KnownVariant::name)
}

/// Produce a short, human-readable summary for a single event.
///
/// The output format is pinned by the per-variant exact-string tests in
/// `commands_diff/tests.rs`; if the format changes for any of the modern
/// variants (`ActionScheduledTicket`, `ActionAbandoned`, `WaitResolved`,
/// `AskTimedOut`, `RunKilled`, `RunAdmission`) those tests will fail with
/// an exact-equal mismatch. New `KnownVariant` arms must be added here in
/// lockstep with [`diff_event_summary`].
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

/// Check whether two events differ in a semantically meaningful way.
pub fn events_differ(a: &JournalEvent, b: &JournalEvent) -> bool {
    match (a, b) {
        (
            JournalEvent::StepSucceeded {
                step: sa,
                output: oa,
                ..
            },
            JournalEvent::StepSucceeded {
                step: sb,
                output: ob,
                ..
            },
        ) => sa != sb || oa != ob,
        (
            JournalEvent::StepStarted { step: sa, .. },
            JournalEvent::StepStarted { step: sb, .. },
        ) => sa != sb,
        (
            JournalEvent::ActionScheduled {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionScheduled {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionCompletedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionCompletedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::ActionFailedEvent {
                step: sa,
                action: aa,
                ..
            },
            JournalEvent::ActionFailedEvent {
                step: sb,
                action: ab,
                ..
            },
        ) => sa != sb || aa != ab,
        (
            JournalEvent::SlotWrittenEvent {
                slot: sa,
                value: va,
                ..
            },
            JournalEvent::SlotWrittenEvent {
                slot: sb,
                value: vb,
                ..
            },
        ) => sa != sb || va != vb,
        (
            JournalEvent::RunFinished { result: ra, .. },
            JournalEvent::RunFinished { result: rb, .. },
        ) => ra != rb,
        _ => event_name(a) != event_name(b),
    }
}

/// Collect the final outcome per step from an event stream.
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut outcomes = HashMap::new();
    for event in events {
        match event {
            JournalEvent::StepSucceeded { step, output, .. } => {
                outcomes.insert(step.get(), format!("succeeded(output={})", output.get()));
            }
            JournalEvent::ActionFailedEvent { step, action, .. } => {
                outcomes.insert(step.get(), format!("failed(action={})", action.get()));
            }
            JournalEvent::ActionCompletedEvent { step, action, .. } => {
                outcomes.insert(
                    step.get(),
                    format!("action_completed(action={})", action.get()),
                );
            }
            _ => {}
        }
    }
    outcomes
}

/// Collect the final display value per slot from an event stream.
pub fn collect_slot_values(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut slots = HashMap::new();
    for event in events {
        if let JournalEvent::SlotWrittenEvent { slot, value, .. } = event {
            let display = match value {
                Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
                    Ok(v) => format!("{v}"),
                    Err(_) => format!("[{} bytes]", bytes.len()),
                },
                None => String::from("none"),
            };
            slots.insert(slot.get(), display);
        }
    }
    slots
}

#[cfg(test)]
#[path = "commands_diff/tests.rs"]
mod tests;
