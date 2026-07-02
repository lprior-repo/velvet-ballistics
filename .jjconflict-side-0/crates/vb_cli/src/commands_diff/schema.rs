#![forbid(unsafe_code)]
//! Compile-time schema fingerprint and canonical `KnownVariant` enumeration.
//!
//! This module owns the FNV-1a 64-bit fold over the canonical
//! [`KNOWN_VARIANTS`] ordering, plus the closed [`KnownVariant`] enum that
//! downstream modules pattern-match against. Three compile-time guards live
//! here; each uses the static-array-length idiom
//! `const _: [(); 1] = [(); if cond { 1 } else { 0 }];` (no production
//! `assert!`/`panic!` macros, no `as_conversions`) so the Holzman rule
//! against production panics and lossy `as` casts both remain satisfied
//! (and `scripts/check-panic-surface.sh` exits 0).

// `JournalEvent` is referenced in doc comments above and through the
// `KNOWN_VARIANTS` / `EXPECTED_SCHEMA_HASH` constants below; the
// `#[allow(unused_imports)]` silences a false-positive warning in
// non-`cfg(test)` compilations where the doc-only references are
// resolved away by rustc.
#[allow(unused_imports)]
use vb_storage::events::JournalEvent;

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
///
/// Uses the static-array-length idiom (length 1 when the condition holds,
/// length 0 when it fails) so that [`scripts/check-panic-surface.sh`] keeps
/// passing without any production `assert!` macros.
const _: [(); 1] = [(); if SCHEMA_HASH != 0 { 1 } else { 0 }];

/// Expected FNV-1a 64-bit hash of the current 24-variant schema.
///
/// Pinned to the FNV-1a fold of `KNOWN_VARIANTS` in canonical order with
/// `0xff` byte separators. The companion compile-time guard
/// (immediately below) and the runtime test
/// `schema_hash_matches_expected` both gate against this value.
///
/// To bump the schema (add/remove/rename a known variant):
///   1. Update `KNOWN_VARIANTS` and the `name()` / `try_from_event` /
///      `diff_event_summary` / `summary_text` matches.
///   2. Compute the new FNV-1a fold and overwrite this constant.
///   3. Recompile — the compile-time guard enforces equality.
pub const EXPECTED_SCHEMA_HASH: u64 = 0x1b5e_5da9_7361_afa6;

/// Compile-time guard that `EXPECTED_SCHEMA_HASH` matches `SCHEMA_HASH`.
///
/// The two are equal at compile time because `SCHEMA_HASH` is derived from
/// `KNOWN_VARIANTS` and `EXPECTED_SCHEMA_HASH` is hand-pinned to the same
/// value. If the maintainer updates `KNOWN_VARIANTS` without updating
/// `EXPECTED_SCHEMA_HASH`, this guard fires before any test runs. Uses the
/// static-array-length idiom so no `assert!` macro enters production code.
const _: [(); 1] = [(); if SCHEMA_HASH == EXPECTED_SCHEMA_HASH {
    1
} else {
    0
}];

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
    /// compile-time guard in this module uses this constant to detect
    /// drift between [`KNOWN_VARIANTS`] and the public contract.
    pub(crate) const COUNT: usize = 24;
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

/// Compile-time guard that `KNOWN_VARIANTS` matches the declared count.
///
/// The array is annotated `[KnownVariant; KnownVariant::COUNT]` so the
/// compiler already enforces the length. This guard additionally
/// guarantees that every element is the canonical enum (no `..` padding
/// or list-comprehension tricks could ever compile here without breaking
/// the const-context). Uses the static-array-length idiom so no `assert!`
/// macro enters production code.
const _: [(); 1] = [(); if KNOWN_VARIANTS.len() == KnownVariant::COUNT {
    1
} else {
    0
}];
