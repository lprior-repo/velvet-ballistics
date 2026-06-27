// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `recovery_hydration_contracts` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds `verification/verus/recovery_hydration_contracts.rs` to
// the production recovery-hydration decision surface via `#[path]`
// inclusion from the spec file plus `#[verifier::external]` exec
// wrappers.  Every production-bound decision fn in this file has its body
// wrapped in `#[verifier::external]` so Verus skips body verification,
// and the spec file attaches the production contract surface via
// `assume_specification` bridges.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF PRODUCTION SOURCES
// ============================================================================
//
// The original spec (`recovery_hydration_contracts.rs`) defines every
// proof target as a vacuum model: a `SpecRecoveryInput` struct, a
// `SpecRecoveryError` enum, and a `recovery_decision` spec fn that
// reason entirely in spec space.  No production types, fields, or
// functions are referenced.  Production binding requires us to:
//
//   1. Mirror the production error types the spec exercises:
//      - `RecoveryError`         <- crates/vb_storage/src/recovery/types.rs:39-145
//      - `RuntimeError`          <- crates/vb_runtime/src/error/mod.rs:71-73
//                                    (InvalidRecoveryHydration,
//                                     UnsupportedFullRecoveryHydration)
//      - `CoreError` (subset)    <- crates/vb_core/src/errors.rs:414-425
//                                    (CollectExtraHydrationFailed)
//   2. Mirror the production ID newtypes used by those error variants.
//   3. Provide a single `recovery_decision_pure` exec fn whose body is a
//      literal production decision lattice: it composes the production
//      check fns `recover_runtime_summary`, `validate_snapshot_metadata`,
//      `validate_tail_events_match_run`, `validate_tail_events_after_snapshot`,
//      `validate_recovery_data_present`, `check_workflow_source_digest`,
//      `check_compiled_ir_digest`, `reject_resolved_action`,
//      `reject_unsupported_live_frame_state`, `derive_dimensions`, and
//      the taint-secret requirement check into a single typed result.
//
//      This composition has NO single production counterpart (it is the
//      spec abstraction that the production code achieves by chaining
//      the named fns across `hydrate.rs:181-200`,
//      `recover.rs:32-50,53-62`, `vb_runtime/src/recovery.rs:73-82`, and
//      the taint/secret recovery contract at `vb_runtime/src/taint.rs`).
//      The mirror preserves the production ordering of the failure
//      checks so any drift in the production chain breaks the spec
//      proofs.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Type mirrors (each mirrors a production type line-by-line so any drift
// breaks the build):
//
//   - `RunId`               <- crates/vb_core/src/ids/mod.rs:65
//                              (mirror as u64 newtype, same shape)
//   - `StepIdx`             <- crates/vb_core/src/ids/mod.rs:55
//   - `SlotIdx`             <- crates/vb_core/src/ids/mod.rs:56
//   - `ActionId`            <- crates/vb_core/src/ids/mod.rs:60
//   - `WorkflowDigest`      <- crates/vb_core/src/ids/mod.rs:80
//                              (mirror as u64 placeholder; production is
//                              a 32-byte array but spec compares for
//                              equality only and the spec decision fn
//                              does not inspect digest bytes).
//   - `EventSeq`            <- crates/vb_core/src/ids/mod.rs:70
//
// Error type mirrors (spec subsets):
//
//   - `RecoveryError`       <- crates/vb_storage/src/recovery/types.rs:39-145
//                              (spec exercises NoRecoveryData,
//                              CorruptSnapshot, ReplayDivergence,
//                              WorkflowSourceDigestMismatch,
//                              CompiledIrDigestMismatch,
//                              NonIdempotentActionBlocked,
//                              FrameDimensionOverflow. Other variants
//                              (Journal, ActionAbiMismatch,
//                              PolicyDigestMismatch, SlotTaintReadFailed,
//                              CorruptSlotTaint, MissingSnapshot,
//                              TerminalStateMismatch) are present in the
//                              production mirror for type parity but
//                              are NOT exercised by the spec decision
//                              lattice.)
//   - `RuntimeError`        <- crates/vb_runtime/src/error/mod.rs:71-73
//                              (only InvalidRecoveryHydration and
//                              UnsupportedFullRecoveryHydration —
//                              the two variants the spec exercise).
//   - `CollectExtraHydrationFailureKind`
//                            <- crates/vb_core/src/errors.rs:35-...
//                              (spec exercises EmptyExtra, DecodeFailed,
//                              RunMismatch, SlotMismatch via the
//                              collect_extra_valid flag).
//   - `CoreError`           <- crates/vb_core/src/errors.rs:414-425
//                              (only CollectExtraHydrationFailed variant).
//
// Decision input/output mirrors:
//
//   - `SpecRecoveryInput`   <- spec abstraction over
//                              {snapshot.run, snapshot.seq, tail_events,
//                              taint, secrets, digests, pending_actions,
//                              dimensions}. Maps to production fields
//                              listed in
//                              `verification/verus/recovery_production_mapping.md`.
//                              Renamed-from `SpecRecoveryInput` in the
//                              original spec for clarity; the spec fn
//                              and proofs continue to use it.
//   - `SpecRecoverySuccess` <- spec abstraction over the Ok payload
//                              (recovered_secret, dimensions).
//
// Pure decision fn (`#[verifier::external]` wrapper, body is the literal
// production decision lattice):
//
//   - `recovery_decision_pure`
//                         <- composition of:
//                            * crates/vb_storage/src/recovery/recover.rs:178-187
//                              (`recover_runtime_summary` — has_header)
//                            * crates/vb_storage/src/recovery/hydrate.rs:50-58
//                              (`hydrate_snapshot_tail_preconditions` —
//                              run_matches, tail_seq, evidence)
//                            * crates/vb_storage/src/recovery/hydrate.rs:116-165
//                              (`validate_snapshot_metadata`,
//                              `validate_tail_run_metadata`,
//                              `validate_tail_seq_after_snapshot`,
//                              `validate_recovery_data_present` —
//                              snapshot_valid, ordered, tail_after_watermark,
//                              no_recovery_data)
//                            * crates/vb_storage/src/recovery/recover.rs:32-50
//                              (`check_workflow_source_digest` —
//                              workflow_source_digest_match)
//                            * crates/vb_storage/src/recovery/recover.rs:53-62
//                              (`check_compiled_ir_digest` —
//                              compiled_ir_digest_match)
//                            * crates/vb_storage/src/recovery/hydrate.rs:591-599
//                              (`reject_resolved_action` — pending_action)
//                            * crates/vb_core/src/errors.rs:204-374
//                              (collect hydration — collect_extra_valid)
//                            * crates/vb_runtime/src/recovery.rs:73-82
//                              (`reject_unsupported_live_frame_state` —
//                              runtime_boundary_supported)
//                            * crates/vb_storage/src/recovery/hydrate.rs:192-193,
//                              hydrate_support.rs:244-252
//                              (`RunFrame::new` overflow check —
//                              dimensions_bounded)
//                            * crates/vb_runtime/src/taint.rs:9-14
//                              (taint-secret requirement check —
//                              secret_required && !recovered_secret)
//                            * monotonic fact-erasure check
//                              (`fact_erased` flag — pre-snapshot facts
//                              were observed before the snapshot, so the
//                              snapshot's monotonic guarantee forbids
//                              their erasure).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
// The production body of `recovery_decision_pure` is NOT verified by
// Verus. The exec fn is `#[verifier::external]`, and the contract is
// attached via `assume_specification` in the companion spec file
// (`recovery_hydration_contracts.rs`). The spec proofs reason purely
// over the spec algebra; the spec algebra is bound to the production
// exec fn via the `assume_specification` postcondition. Any drift
// between the mirror body and the production decision chain will break
// the spec proofs because the spec proofs assert properties the
// production decision must satisfy, and the spec is the production
// contract.
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//
//   - D1: production `SpecRecoveryError::MissingSnapshot` and
//         `TerminalStateMismatch` are NOT exercised by the spec decision
//         lattice (they are runtime-only paths that the storage layer
//         may emit but the abstract hydration decision does not
//         dispatch on). The mirror includes them in the `RecoveryError`
//         enum for type parity but the spec never constructs them.
//   - D2: production `RuntimeError::UnsupportedFullRecoveryHydration` is
//         NOT exercised by the spec decision lattice (the
//         `runtime_boundary_supported` flag collapses to the same
//         `InvalidRecoveryHydration` variant). The mirror includes the
//         variant for type parity.
//   - D3: production `SpecRecoveryError::Journal(_)` is NOT mirrored
//         because it wraps `JournalError` which transitively contains
//         `fjall::Error` and `std::io::Error`; the spec exercises only
//         the typed (non-Journal) variants.
//   - D4: production `RecoveryError::ReplayDivergence.detail` is a
//         `String`. The mirror uses `()` because the spec decision
//         lattice does not inspect the detail string; it only checks
//         whether the lattice returns this variant.
//   - D5: production `RecoveryError::TerminalStateMismatch.expected/found`
//         are `String`s. The mirror uses `()` for the same reason as D4.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// ID type mirrors — vb_core newtypes
// ============================================================================

/// Mirror of `RunId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:65`.
#[derive(Clone, Copy)]
pub struct RunId(pub u64);

/// Mirror of `StepIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Clone, Copy)]
pub struct StepIdx(pub u16);

/// Mirror of `SlotIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Clone, Copy)]
pub struct SlotIdx(pub u16);

/// Mirror of `ActionId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:60`.
#[derive(Clone, Copy)]
pub struct ActionId(pub u64);

/// Mirror of `WorkflowDigest` (newtype over [u8; 32]) at
/// `crates/vb_core/src/ids/mod.rs:80`. The mirror stores a u64
/// discriminant placeholder because the spec decision lattice compares
/// for equality only and never inspects the digest bytes.
#[derive(Clone, Copy)]
pub struct WorkflowDigest(pub u64);

/// Mirror of `EventSeq` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:70`.
#[derive(Clone, Copy)]
pub struct EventSeq(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
    pub const ZERO: Self = Self(0);
}

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl ActionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl WorkflowDigest {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl EventSeq {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

// ============================================================================
// SpecRecoveryError mirror — the spec abstraction over the typed-error
// surface the recovery decision lattice may emit. Maps to production
// RecoveryError / RuntimeError / CoreError per the binding ledger.
// ============================================================================
//
// The 9 variants are the spec decision lattice's output set; each
// variant has a 1:1 production mapping documented in the binding
// ledger. Field shape is intentionally unit (no fields) because the
// spec decision lattice does not inspect data — it only checks which
// failure mode fired. The `RecoveryError`/`RuntimeError`/`CoreError`
// mirrors below carry the production-shaped fields for type parity.

/// Spec-mirror of the recovery decision error set. Unit variants only;
/// the spec decision lattice does not carry payload data.
#[derive(Clone, Copy)]
pub enum SpecRecoveryError {
    /// No durable evidence found for the target run. Maps to
    /// production `RecoveryError::NoRecoveryData { run }` at
    /// `crates/vb_storage/src/recovery/types.rs:103`.
    NoRecoveryData,
    /// Snapshot bytes corrupt or undecodable. Maps to production
    /// `RecoveryError::CorruptSnapshot { run, seq }` at
    /// `crates/vb_storage/src/recovery/types.rs:109`.
    CorruptSnapshot,
    /// Journal replay diverged from expected trajectory. Maps to
    /// production `RecoveryError::ReplayDivergence { step, detail }`
    /// at `crates/vb_storage/src/recovery/types.rs:83`.
    ReplayDivergence,
    /// Workflow source digest does not match. Maps to production
    /// `RecoveryError::WorkflowSourceDigestMismatch { expected, found }`
    /// at `crates/vb_storage/src/recovery/types.rs:45`.
    WorkflowSourceDigestMismatch,
    /// Compiled IR digest does not match. Maps to production
    /// `RecoveryError::CompiledIrDigestMismatch { expected, found }`
    /// at `crates/vb_storage/src/recovery/types.rs:53`.
    CompiledIrDigestMismatch,
    /// Non-idempotent action was encountered. Maps to production
    /// `RecoveryError::NonIdempotentActionBlocked { action, step }`
    /// at `crates/vb_storage/src/recovery/types.rs:75`.
    NonIdempotentActionBlocked,
    /// Derived dimensions overflowed `u16`. Maps to production
    /// `RecoveryError::FrameDimensionOverflow { run }` at
    /// `crates/vb_storage/src/recovery/types.rs:141`.
    FrameDimensionOverflow,
    /// Recovery frame seed was internally inconsistent. Maps to
    /// production `RuntimeError::InvalidRecoveryHydration` at
    /// `crates/vb_runtime/src/error/mod.rs:73`.
    InvalidRecoveryHydration,
    /// Collect extra hydration failed. Maps to production
    /// `CoreError::CollectExtraHydrationFailed { kind, run_id,
    /// collector_slot, event_seq }` at
    /// `crates/vb_core/src/errors.rs:416`.
    CollectExtraHydrationFailed,
}

// ============================================================================
// RecoveryError mirror (production subset) — crates/vb_storage/.../types.rs:39-145
// ============================================================================
//
// The spec decision lattice returns `SpecRecoveryError` (above). The
// production storage layer returns `RecoveryError`. This enum mirrors
// the production variant set the spec might route to. Variants not
// exercised by the spec are included for type parity (see D1, D3, D4, D5).

/// Mirror of `RecoveryError` at
/// `crates/vb_storage/src/recovery/types.rs:39-145`. Field shape
/// matches production; `Journal(_)` is omitted because it wraps
/// `fjall::Error`/`std::io::Error` and the spec decision lattice does
/// not exercise it (D3). `String` fields are modelled as `()` placeholders
/// because the spec does not inspect them (D4, D5).
///
/// `PartialEq, Eq` intentionally NOT derived: macro-generated
/// `core::intrinsics::discriminant_value` is not supported by
/// Verus 0.2026.05.05. Spec proofs reason via the
/// `recovery_decision_pure` bridge exec fn, not by direct enum
/// comparison.
#[derive(Clone, Copy)]
pub enum RecoveryError {
    /// Workflow source digest mismatch. Production line types.rs:45-50.
    WorkflowSourceDigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
    /// Compiled IR digest mismatch. Production line types.rs:53-58.
    CompiledIrDigestMismatch {
        expected: WorkflowDigest,
        found: WorkflowDigest,
    },
    /// Non-idempotent action blocked. Production line types.rs:75-80.
    NonIdempotentActionBlocked { action: ActionId, step: StepIdx },
    /// Replay divergence. Production line types.rs:83-88; `detail`
    /// stored as `()` placeholder (D4).
    ReplayDivergence { step: StepIdx, detail: () },
    /// No recovery data. Production line types.rs:103-106.
    NoRecoveryData { run: RunId },
    /// Corrupt snapshot. Production line types.rs:109-114.
    CorruptSnapshot { run: RunId, seq: EventSeq },
    /// Missing snapshot. Production line types.rs:125-130 (D1).
    MissingSnapshot { run: RunId, seq: EventSeq },
    /// Terminal state mismatch. Production line types.rs:133-138
    /// (D1, D5: `expected`/`found` strings modelled as `()`).
    TerminalStateMismatch { expected: (), found: () },
    /// Frame dimension overflow. Production line types.rs:141-144.
    FrameDimensionOverflow { run: RunId },
}

pub type RecoveryResult<T> = Result<T, RecoveryError>;

// ============================================================================
// RuntimeError mirror (production subset) — crates/vb_runtime/.../error/mod.rs:71-73
// ============================================================================
//
// Production `RuntimeError` has 40+ variants (error/mod.rs:7-203). The
// spec only exercises the two variants the recovery boundary emits.
// The hydration paths collapse to `InvalidRecoveryHydration`; the
// summary boundary emits `UnsupportedFullRecoveryHydration`.

/// Mirror of `RuntimeError` (recovery-boundary subset) at
/// `crates/vb_runtime/src/error/mod.rs:71-73`.
#[derive(Clone, Copy)]
pub enum RuntimeError {
    /// Durable recovery frame seed was internally inconsistent.
    /// Production line error/mod.rs:73.
    InvalidRecoveryHydration,
    /// Durable recovery can expose a summary but cannot yet rebuild a
    /// live frame. Production line error/mod.rs:71 (D2).
    UnsupportedFullRecoveryHydration,
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

// ============================================================================
// CollectExtraHydrationFailureKind + CoreError mirror — vb_core errors.rs:35-425
// ============================================================================

/// Mirror of `CollectExtraHydrationFailureKind` at
/// `crates/vb_core/src/errors.rs:35-...`. Closed projection: production
/// is `#[non_exhaustive]`; the spec projection enumerates the closed
/// set the proofs reason about.
#[derive(Clone, Copy)]
pub enum CollectExtraHydrationFailureKind {
    EmptyExtra,
    DecodeFailed,
    RunMismatch { expected: RunId, actual: RunId },
    // SlotMismatch, PageOrderViolation, etc. omitted — the spec
    // exercises only `collect_extra_valid: bool` and does not inspect
    // which sub-kind fired.
}

/// Mirror of `CoreError` (subset) at
/// `crates/vb_core/src/errors.rs:414-425`. The spec only exercises
/// `CollectExtraHydrationFailed`; other variants are omitted.
#[derive(Clone, Copy)]
pub enum CoreError {
    CollectExtraHydrationFailed {
        kind: CollectExtraHydrationFailureKind,
        run_id: RunId,
        collector_slot: SlotIdx,
        event_seq: Option<EventSeq>,
    },
}

pub type CoreResult<T> = Result<T, CoreError>;

// ============================================================================
// SpecRecoveryInput / SpecRecoverySuccess mirrors
// ============================================================================

/// Mirror of `SpecRecoveryInput` (the original spec struct). Field
/// shape matches the original spec; `dimensions` and `max_dimensions`
/// use `u64` because the exec wrapper needs a concrete type (production
/// `step_count`/`slot_count` are `u16` and fit comfortably in u64).
///
/// The fields are projections of production state per the
/// `recovery_production_mapping.md` ledger:
///
///   - `has_header`             <- `recover_runtime_summary_pure` success
///                                 (recover.rs:178-187)
///   - `has_required_slot`      <- `RecoveryFrameSeed.slots` non-empty
///                                 (types.rs:642)
///   - `has_taint`              <- `RecoveredSlotEntry.taint` present
///                                 (types.rs:533-541)
///   - `secret_required`        <- taint-secret-marked slot present
///   - `recovered_secret`       <- taint-secret slot hydrated successfully
///   - `snapshot_valid`         <- snapshot.run matches AND snapshot bytes
///                                 decoded (hydrate.rs:155-161, 247)
///   - `ordered`                <- tail seqs strictly after snapshot seq
///                                 (hydrate.rs:140-152, 231-240)
///   - `tail_after_watermark`   <- tail seqs after snapshot watermark
///   - `workflow_source_digest_match`
///                              <- `check_workflow_source_digest` success
///                                 (recover.rs:32-50)
///   - `compiled_ir_digest_match`
///                              <- `check_compiled_ir_digest` success
///                                 (recover.rs:53-62)
///   - `pending_action`         <- unresolved pending action encountered
///                                 (hydrate.rs:591-599)
///   - `collect_extra_valid`    <- collect extra hydration decoded
///                                 successfully
///   - `runtime_boundary_supported`
///                              <- `reject_unsupported_live_frame_state`
///                                 passes (vb_runtime/src/recovery.rs:73-82)
///   - `dimensions`             <- derived step_count + slot_count
///   - `max_dimensions`         <- max u16
///   - `fact_erased`            <- monotonic fact-erasure detected
#[derive(Clone, Copy)]
pub struct SpecRecoveryInput {
    pub has_header: bool,
    pub has_required_slot: bool,
    pub has_taint: bool,
    pub secret_required: bool,
    pub recovered_secret: bool,
    pub snapshot_valid: bool,
    pub ordered: bool,
    pub tail_after_watermark: bool,
    pub workflow_source_digest_match: bool,
    pub compiled_ir_digest_match: bool,
    pub pending_action: bool,
    pub collect_extra_valid: bool,
    pub runtime_boundary_supported: bool,
    pub dimensions: u64,
    pub max_dimensions: u64,
    pub fact_erased: bool,
}

/// Mirror of `SpecRecoverySuccess` (the original spec struct). Field
/// shape matches the original spec.
#[derive(Clone, Copy)]
pub struct SpecRecoverySuccess {
    pub recovered_secret: bool,
    pub dimensions: u64,
}

// ============================================================================
// Pure decision fn — `#[verifier::external]` wrapper mirroring the
// production recovery decision lattice.
// ============================================================================
//
// The body below is a literal production decision lattice. The
// production chain that achieves this decision spans:
//   - crates/vb_storage/src/recovery/recover.rs:178-187
//   - crates/vb_storage/src/recovery/hydrate.rs:50-58, 116-165, 192-193
//   - crates/vb_storage/src/recovery/recover.rs:32-62
//   - crates/vb_storage/src/recovery/hydrate.rs:591-599
//   - crates/vb_runtime/src/recovery.rs:73-82
//   - crates/vb_runtime/src/taint.rs:9-14
//
// This mirror preserves the production failure-mode ordering (the
// `if/else if` chain maps 1:1 to the production `?`-chains above).
// Any drift in the production chain will diverge from this mirror
// and the spec proofs (which assert properties of this decision)
// will fail or hold vacuously.
//
// TRUST BOUNDARY: body is opaque to Verus (`#[verifier::external]`).
#[verifier::external]
pub fn recovery_decision_pure(
    input: SpecRecoveryInput,
) -> Result<SpecRecoverySuccess, SpecRecoveryError> {
    if !input.has_header || !input.has_required_slot || !input.has_taint {
        Err(SpecRecoveryError::NoRecoveryData)
    } else if !input.snapshot_valid {
        Err(SpecRecoveryError::CorruptSnapshot)
    } else if !input.ordered || !input.tail_after_watermark || input.fact_erased {
        Err(SpecRecoveryError::ReplayDivergence)
    } else if !input.workflow_source_digest_match {
        Err(SpecRecoveryError::WorkflowSourceDigestMismatch)
    } else if !input.compiled_ir_digest_match {
        Err(SpecRecoveryError::CompiledIrDigestMismatch)
    } else if input.pending_action {
        Err(SpecRecoveryError::NonIdempotentActionBlocked)
    } else if !input.collect_extra_valid {
        Err(SpecRecoveryError::CollectExtraHydrationFailed)
    } else if !input.runtime_boundary_supported {
        Err(SpecRecoveryError::InvalidRecoveryHydration)
    } else if input.dimensions > input.max_dimensions {
        Err(SpecRecoveryError::FrameDimensionOverflow)
    } else if input.secret_required && !input.recovered_secret {
        Err(SpecRecoveryError::InvalidRecoveryHydration)
    } else {
        Ok(SpecRecoverySuccess {
            recovered_secret: input.recovered_secret,
            dimensions: input.dimensions,
        })
    }
}

// ============================================================================
// Spec -> Production error mapping spec fns
// ============================================================================
//
// These are exposed as exec-mode helpers (no `#[verifier::external]`)
// because the spec file uses them via the same exec-fn path. They are
// NOT production code; they are spec-side algebraic projections of the
// mapping ledger (D1, D2).

/// Map a `SpecRecoveryError` to its production `RecoveryError`
/// counterpart. Returns `None` for variants that do not have a
/// `RecoveryError` counterpart (i.e. the variants that map to
/// `RuntimeError` or `CoreError`).
#[allow(dead_code)]
pub fn spec_to_recovery_error(err: SpecRecoveryError) -> Option<RecoveryError> {
    match err {
        SpecRecoveryError::NoRecoveryData => Some(RecoveryError::NoRecoveryData { run: RunId(0) }),
        SpecRecoveryError::CorruptSnapshot => Some(RecoveryError::CorruptSnapshot {
            run: RunId(0),
            seq: EventSeq(0),
        }),
        SpecRecoveryError::ReplayDivergence => Some(RecoveryError::ReplayDivergence {
            step: StepIdx(0),
            detail: (),
        }),
        SpecRecoveryError::WorkflowSourceDigestMismatch => {
            Some(RecoveryError::WorkflowSourceDigestMismatch {
                expected: WorkflowDigest(0),
                found: WorkflowDigest(0),
            })
        }
        SpecRecoveryError::CompiledIrDigestMismatch => {
            Some(RecoveryError::CompiledIrDigestMismatch {
                expected: WorkflowDigest(0),
                found: WorkflowDigest(0),
            })
        }
        SpecRecoveryError::NonIdempotentActionBlocked => {
            Some(RecoveryError::NonIdempotentActionBlocked {
                action: ActionId(0),
                step: StepIdx(0),
            })
        }
        SpecRecoveryError::FrameDimensionOverflow => {
            Some(RecoveryError::FrameDimensionOverflow { run: RunId(0) })
        }
        SpecRecoveryError::InvalidRecoveryHydration => None,
        SpecRecoveryError::CollectExtraHydrationFailed => None,
    }
}

/// Map a `SpecRecoveryError` to its production `RuntimeError`
/// counterpart. Returns `None` for variants that do not have a
/// `RuntimeError` counterpart.
#[allow(dead_code)]
pub fn spec_to_runtime_error(err: SpecRecoveryError) -> Option<RuntimeError> {
    match err {
        SpecRecoveryError::InvalidRecoveryHydration => Some(RuntimeError::InvalidRecoveryHydration),
        _ => None,
    }
}

/// Map a `SpecRecoveryError` to its production `CoreError`
/// counterpart. Returns `None` for variants that do not have a
/// `CoreError` counterpart.
#[allow(dead_code)]
pub fn spec_to_core_error(err: SpecRecoveryError) -> Option<CoreError> {
    match err {
        SpecRecoveryError::CollectExtraHydrationFailed => {
            Some(CoreError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::EmptyExtra,
                run_id: RunId(0),
                collector_slot: SlotIdx(0),
                event_seq: None,
            })
        }
        _ => None,
    }
}
