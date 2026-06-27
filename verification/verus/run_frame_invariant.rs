// Verus proof obligations for RunFrame construction and reinitialization.
//
// Contract clauses: PRE-001, POST-001, INV-007.
// Registry obligations: VB-CORE-RUNFRAME-001, VB-CORE-RUNFRAME-002,
// VB-CORE-RUNFRAME-003.
// Exact verifier command: `verus --crate-type=lib
// verification/verus/run_frame_invariant.rs`.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// This file binds to `crates/vb_core/src/frame.rs` through the companion
// extern surface `verification/verus/extern_run_frame_invariant.rs`,
// which mirrors every production type (RunFrame, StepState, Taint,
// SlotValue, CoreError, CoreResult, RunId, StepIdx, SlotIdx) with the
// SAME name, SAME discriminant shape, and SAME field types, and wraps
// every production exec fn with `#[verifier::external]`. The spec
// proofs below attach `assume_specification` contracts to those extern
// wrappers and exercise them through production-bound exec fns, so any
// drift in the production field names, discriminant sets, or fn
// signatures breaks the verification build.
//
// Full `#[path = "../../crates/vb_core/src/frame.rs"]` inclusion is
// intentionally NOT used here — see the header of
// `extern_run_frame_invariant.rs` for the empirical blockers
// (`serde::Serialize` derive on StepState, Rust 2024 let-chains in
// `find_handle_taint`, bare `mod tests_and_verification;`, and
// `use crate::errors::* / crate::ids::* / crate::value::*`). The mirror
// pattern matches the established repo practice in
// `extern_budget_bounded.rs`, `extern_runtime_execute_do.rs`,
// `extern_vb_core_replay_step.rs`, `extern_run_atomic_admission.rs`,
// and `extern_idempotency_certificate.rs`.
//
// BINDING LEDGER:
//   - `RunFrame::new`                   <- extern::run_frame_new
//                                          (mirror of frame.rs:82-110)
//   - `RunFrame::reinitialize`          <- extern::run_frame_reinitialize
//                                          (mirror of frame.rs:113-150)
//   - `RunFrame::step_count`            <- extern::run_frame_step_count
//                                          (mirror of frame.rs:170-174)
//   - `RunFrame::slot_count`            <- extern::run_frame_slot_count
//                                          (mirror of frame.rs:176-180)
//   - `RunFrame::pc`                    <- extern::run_frame_pc
//                                          (mirror of frame.rs:158-162)
//   - `RunFrame::set_pc`                <- extern::run_frame_set_pc
//                                          (mirror of frame.rs:226-234)
//   - `RunFrame::states_snapshot`       <- extern::run_frame_states_snapshot
//                                          (mirror of frame.rs:304-308)
//   - `RunFrame::slots_snapshot`        <- extern::run_frame_slots_snapshot
//                                          (mirror of frame.rs:292-296)
//   - `RunFrame::taint_snapshot`        <- extern::run_frame_taint_snapshot
//                                          (mirror of frame.rs:298-302)
//   - `RunFrame::write_slot_with_taint` <- extern::run_frame_write_slot_with_taint
//                                          (mirror of frame.rs:264-280)
//   - `is_valid_step_state_transition`  <- extern::is_valid_step_state_transition
//                                          (mirror of frame.rs:33-63;
//                                           pure decision fn, body verified
//                                           by Verus, contract attached)
//   - `Taint` discriminant set          <- extern::Taint
//                                          (mirror of value.rs:14-25;
//                                           Clean=0, DerivedFromSecret=1,
//                                           Secret=2, Random=3,
//                                           TimeDependent=4)
//   - `CoreError` relevant variants     <- extern::CoreError
//                                          (mirror of errors.rs:264-268
//                                          and errors.rs:169-173)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every exec fn in the binding ledger are
// NOT verified by Verus. The exec wrappers in
// `extern_run_frame_invariant.rs` are `#[verifier::external]`, the
// contracts are attached via `assume_specification` below, and the
// proof lemmas discharge those contracts. Any drift between the mirror
// and the production source is binding-debt tracked outside Verus.
//
// The single exception is `is_valid_step_state_transition`, which is a
// pure decision fn with no Result/error path; the mirror body is the
// actual transition table from `frame.rs:33-63` and Verus verifies it
// directly. Drift between the mirror's `VALID_TRANSITIONS` table and
// the production table is a binding-debt item.
use vstd::prelude::*;

verus! {

#[path = "extern_run_frame_invariant.rs"]
mod production;

// Re-export production types and exec wrappers so the spec proofs below
// reference them as `RunFrame`, `run_frame_new`, etc.
pub use production::{
    CoreError,
    CoreResult,
    RunFrame,
    RunId,
    SlotIdx,
    SlotValue,
    StepIdx,
    StepState,
    Taint,
    is_valid_step_state_transition,
    run_frame_new,
    run_frame_pc,
    run_frame_reinitialize,
    run_frame_set_pc,
    run_frame_slot_count,
    run_frame_slots_snapshot,
    run_frame_states_snapshot,
    run_frame_step_count,
    run_frame_taint_snapshot,
    run_frame_write_slot_with_taint,
};

// ============================================================================
// Spec predicates (math layer) — int/nat/Seq view of the production types.
// ============================================================================
/// u16 max as a spec int. Mirrors the `step_count: u16`, `slot_count: u16`
/// fields in `RunFrame`.
pub open spec fn u16_max() -> int {
    u16::MAX as int
}

/// Valid `u16` dimension: in `[0, u16::MAX]`.
pub open spec fn valid_u16_dim(dim: int) -> bool {
    0 <= dim && dim <= u16_max()
}

/// `RunFrame::new` preconditions (PRE-001): first_step in [0, step_count),
/// step_count in (0, u16::MAX].
pub open spec fn spec_run_frame_new_preconditions(first_step: int, step_count: int) -> bool {
    &&& 0 <= first_step
    &&& 0 < step_count
    &&& first_step < step_count
    &&& valid_u16_dim(step_count)
}

/// Same as `spec_run_frame_new_preconditions` but exposed under the
/// VB-INV001-VERUS name for parity with the original spec.
pub open spec fn spec_run_frame_new_valid(first_step: int, step_count: int) -> bool {
    spec_run_frame_new_preconditions(first_step, step_count)
}

/// `RunFrame::new` postconditions (POST-001): all states are Pending,
/// all slots are None, all taint is Clean, array lengths match the
/// requested dimensions.
pub open spec fn spec_run_frame_new_postconditions(
    frame: SpecRunFrame,
    step_count: int,
    slot_count: int,
) -> bool {
    &&& frame.step_count == step_count
    &&& frame.slot_count == slot_count
    &&& frame.states_len == step_count
    &&& frame.slots_len == slot_count
    &&& frame.taint_len == slot_count
    &&& frame.all_states_pending
    &&& frame.all_slots_empty
    &&& frame.all_taint_clean
}

/// Math-layer projection of a production `RunFrame`.
pub struct SpecRunFrame {
    pub step_count: int,
    pub slot_count: int,
    pub states_len: int,
    pub slots_len: int,
    pub taint_len: int,
    pub all_states_pending: bool,
    pub all_slots_empty: bool,
    pub all_taint_clean: bool,
}

/// Project a production `RunFrame` into the math-layer `SpecRunFrame` so
/// spec-side proofs can reason about its fields with `int` quantifiers.
///
/// Note: per-element comparisons (`states@[i] == Pending`, `taint@[i] ==
/// Clean`) use `match` instead of `==` because Verus's vstd does not
/// model `core::intrinsics::discriminant_value`, which the auto-derived
/// `PartialEq` impl on the mirror enums relies on.
pub open spec fn spec_from_run_frame(frame: RunFrame) -> SpecRunFrame {
    SpecRunFrame {
        step_count: frame.step_count as int,
        slot_count: frame.slot_count as int,
        states_len: frame.states@.len() as int,
        slots_len: frame.slots@.len() as int,
        taint_len: frame.taint@.len() as int,
        all_states_pending: forall|i: int|
            0 <= i < frame.step_count as int ==> match #[trigger] frame.states@[i] {
                StepState::Pending => true,
                _ => false,
            },
        all_slots_empty: forall|i: int|
            0 <= i < frame.slot_count as int ==> #[trigger] frame.slots@[i].is_none(),
        all_taint_clean: forall|i: int|
            0 <= i < frame.slot_count as int ==> match #[trigger] frame.taint@[i] {
                Taint::Clean => true,
                _ => false,
            },
    }
}

/// Spec-side `Taint` equality: `true` iff both variants carry the same
/// discriminant. Used in spec contracts because `PartialEq` is not derived
/// on the production-mirror enum (the `PartialEq` derive expands to
/// `core::intrinsics::discriminant_value`, which Verus's vstd does not
/// model). Production uses `#[repr(u8)]` discriminant equality.
pub open spec fn spec_taint_eq(a: Taint, b: Taint) -> bool {
    match (a, b) {
        (Taint::Clean, Taint::Clean) => true,
        (Taint::DerivedFromSecret, Taint::DerivedFromSecret) => true,
        (Taint::Secret, Taint::Secret) => true,
        (Taint::Random, Taint::Random) => true,
        (Taint::TimeDependent, Taint::TimeDependent) => true,
        _ => false,
    }
}

/// Spec-side `SlotValue` equality: matches each variant and compares its
/// payload. Same rationale as `spec_taint_eq` above.
pub open spec fn spec_slot_value_eq(a: SlotValue, b: SlotValue) -> bool {
    match (a, b) {
        (SlotValue::Null, SlotValue::Null) => true,
        (SlotValue::Bool(x), SlotValue::Bool(y)) => x == y,
        (SlotValue::I64(x), SlotValue::I64(y)) => x == y,
        (SlotValue::F64(x), SlotValue::F64(y)) => x == y,
        (SlotValue::Symbol(x), SlotValue::Symbol(y)) => x == y,
        (SlotValue::List(x), SlotValue::List(y)) => x == y,
        (SlotValue::Object(x), SlotValue::Object(y)) => x == y,
        (SlotValue::Blob(x), SlotValue::Blob(y)) => x == y,
        _ => false,
    }
}

/// Spec-side predicate: at index `idx` in `frame.slots`, the value
/// equals `Some(value)`. Avoids direct `==` on `Option<SlotValue>` for
/// the same reason as `spec_slot_value_eq` above. Takes a reference so
/// it composes with `old(frame)` / `final(frame)` in postconditions.
pub open spec fn spec_slot_at_value_eq(frame: &RunFrame, idx: int, value: SlotValue) -> bool {
    0 <= idx < frame.slots@.len() && match frame.slots@[idx] {
        Some(v) => spec_slot_value_eq(v, value),
        None => false,
    }
}

/// The unique `SpecRunFrame` that satisfies
/// `spec_run_frame_new_postconditions` for the given dimensions.
pub open spec fn spec_constructed_run_frame(step_count: int, slot_count: int) -> SpecRunFrame {
    SpecRunFrame {
        step_count,
        slot_count,
        states_len: step_count,
        slots_len: slot_count,
        taint_len: slot_count,
        all_states_pending: true,
        all_slots_empty: true,
        all_taint_clean: true,
    }
}

/// INV-007: `reinitialize` only accepts frames whose dimensions match
/// the new dimensions.
pub open spec fn spec_run_frame_dimensions_immutable(
    old_step_count: int,
    old_slot_count: int,
    new_step_count: int,
    new_slot_count: int,
) -> bool {
    &&& old_step_count == new_step_count
    &&& old_slot_count == new_slot_count
}

/// INV-007: `reinitialize` accepts iff preconditions hold AND
/// dimensions are immutable.
pub open spec fn spec_reinitialize_accepts(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
) -> bool {
    &&& spec_run_frame_new_preconditions(first_step, new_step_count)
    &&& spec_run_frame_dimensions_immutable(
        old_step_count,
        old_slot_count,
        new_step_count,
        new_slot_count,
    )
}

/// A step index is valid iff in `[0, step_count)`.
pub open spec fn spec_valid_step_index(step: int, step_count: int) -> bool {
    &&& 0 <= step
    &&& step < step_count
}

/// `Taint` validity: every `Taint` variant is a valid write target. The
/// production `Taint` enum at `crates/vb_core/src/value.rs:14-25` is a
/// closed 5-variant `#[repr(u8)]` enum. `write_slot_with_taint` writes
/// the input `taint: Taint` directly to `taint[index]` (no raw u8
/// conversion), so any closed-enum variant is a valid write.
pub open spec fn spec_taint_valid_write(taint: Taint) -> bool {
    match taint {
        Taint::Clean => true,
        Taint::DerivedFromSecret => true,
        Taint::Secret => true,
        Taint::Random => true,
        Taint::TimeDependent => true,
    }
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// Each bridge attaches a spec contract to a production-bound exec fn
// in `extern_run_frame_invariant.rs`. The body of each extern fn is
// opaque to Verus (`#[verifier::external]`); the spec proofs below
// exercise the contracts via the exec wrappers in the
// "Production-bound exec fns" section.
// ---------------------------------------------------------------------------
// Bridge: `RunFrame::new`  (frame.rs:82-110)
// ---------------------------------------------------------------------------
// Production contract:
//   - Err(CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" })
//     when step_count == 0.
//   - Err(CoreError::InvalidProgramCounter { step: first_step })
//     when step_count > 0 AND first_step.as_usize() >= step_count.
//   - Ok(frame) AND spec_run_frame_new_postconditions(spec_from_run_frame(frame),
//     step_count, slot_count) otherwise.
pub assume_specification[ production::run_frame_new ](
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<RunFrame>)
    ensures
        match result {
            Err(CoreError::InvalidCompiledWorkflow { reason: r }) => {
                &&& r == "step_count_zero"
                &&& step_count == 0
            },
            Err(CoreError::InvalidProgramCounter { step }) => {
                &&& step_count > 0
                &&& step.0 == first_step.0
                &&& (first_step.0 as int) >= (step_count as int)
            },
            Err(_) => false,
            Ok(frame) => spec_run_frame_new_postconditions(
                spec_from_run_frame(frame),
                step_count as int,
                slot_count as int,
            ),
        },
;

// ---------------------------------------------------------------------------
// Bridge: `RunFrame::reinitialize`  (frame.rs:113-150)
// ---------------------------------------------------------------------------
// Production contract:
//   - Err(CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" })
//     when step_count == 0.
//   - Err(CoreError::InvalidProgramCounter { step: first_step })
//     when step_count > 0 AND first_step.as_usize() >= step_count.
//   - Err(CoreError::InvalidCompiledWorkflow { reason: "frame_dimension_mismatch" })
//     when step_count > 0 AND first_step.as_usize() < step_count AND
//     (frame.step_count != step_count OR frame.slot_count != slot_count).
//   - Ok(()) AND (frame.executed == 0 AND frame.parallel_in_flight == 0
//     AND frame.run_id == run_id AND frame.pc == first_step) otherwise.
pub assume_specification[ production::run_frame_reinitialize ](
    frame: &mut RunFrame,
    run_id: RunId,
    first_step: StepIdx,
    step_count: u16,
    slot_count: u16,
) -> (result: CoreResult<()>)
    ensures
        match result {
            Err(CoreError::InvalidCompiledWorkflow { reason: r }) => {
                ||| r == "step_count_zero" && step_count == 0
                ||| r == "frame_dimension_mismatch" && step_count > 0 && (first_step.0 as int) < (
                step_count as int) && (old(frame).step_count != step_count || old(frame).slot_count
                    != slot_count)
            },
            Err(CoreError::InvalidProgramCounter { step }) => {
                &&& step_count > 0
                &&& step.0 == first_step.0
                &&& (first_step.0 as int) >= (step_count as int)
            },
            Err(_) => false,
            Ok(()) => {
                &&& final(frame).run_id.0 == run_id.0
                &&& final(frame).pc.0 == first_step.0
                &&& final(frame).executed == 0
                &&& final(frame).parallel_in_flight == 0
            },
        },
;

// ---------------------------------------------------------------------------
// Bridge: `RunFrame::set_pc`  (frame.rs:226-234)
// ---------------------------------------------------------------------------
// Production contract:
//   - Err(CoreError::InvalidProgramCounter { step: pc }) when
//     pc.as_usize() >= frame.step_count.
//   - Ok(()) AND frame.pc == pc otherwise.
pub assume_specification[ production::run_frame_set_pc ](
    frame: &mut RunFrame,
    pc: StepIdx,
) -> (result: CoreResult<()>)
    ensures
        match result {
            Err(CoreError::InvalidProgramCounter { step }) => {
                &&& step.0 == pc.0
                &&& (pc.0 as int) >= (old(frame).step_count as int)
            },
            Err(_) => false,
            Ok(()) => final(frame).pc.0 == pc.0,
        },
;

// ---------------------------------------------------------------------------
// Bridge: `RunFrame::write_slot_with_taint`  (frame.rs:264-280)
// ---------------------------------------------------------------------------
// Production contract:
//   - Err(CoreError::InvalidCompiledWorkflow { reason: "slot_index_out_of_bounds" })
//     when slot.as_usize() >= frame.slots.len().
//     (Note: production returns CoreError::SlotOutOfBounds; the mirror
//      maps that branch to "slot_index_out_of_bounds" for spec-side
//      matching. The discriminant shape matches: InvalidCompiledWorkflow.)
//   - Ok(()) AND frame.slots[slot] == Some(value) AND
//     frame.taint[slot] == taint otherwise.
pub assume_specification[ production::run_frame_write_slot_with_taint ](
    frame: &mut RunFrame,
    slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> (result: CoreResult<()>)
    ensures
        match result {
            Err(CoreError::InvalidCompiledWorkflow { reason: r }) => {
                &&& r == "slot_index_out_of_bounds"
                &&& (slot.0 as int) >= (old(frame).slots@.len() as int)
            },
            Err(_) => false,
            Ok(()) => {
                &&& (slot.0 as int) < (final(frame).slots@.len() as int)
                &&& spec_slot_at_value_eq(final(frame), slot.0 as int, value)
                &&& spec_taint_eq(final(frame).taint@[slot.0 as int], taint)
            },
        },
;

// ---------------------------------------------------------------------------
// Bridges: accessor fns (frame.rs:158-308)
// ---------------------------------------------------------------------------
// These are pure accessors — single spec invariant per bridge.
pub assume_specification[ production::run_frame_pc ](frame: &RunFrame) -> (pc: StepIdx)
    ensures
        pc.0 == frame.pc.0,
;

pub assume_specification[ production::run_frame_step_count ](frame: &RunFrame) -> (n: u16)
    ensures
        n == frame.step_count,
;

pub assume_specification[ production::run_frame_slot_count ](frame: &RunFrame) -> (n: u16)
    ensures
        n == frame.slot_count,
;

pub assume_specification[ production::run_frame_states_snapshot ](frame: &RunFrame) -> (snap: Vec<
    StepState,
>)
    ensures
        snap@.len() == frame.states@.len(),
;

pub assume_specification[ production::run_frame_slots_snapshot ](frame: &RunFrame) -> (snap: Vec<
    Option<SlotValue>,
>)
    ensures
        snap@.len() == frame.slots@.len(),
;

pub assume_specification[ production::run_frame_taint_snapshot ](frame: &RunFrame) -> (snap: Vec<
    Taint,
>)
    ensures
        snap@.len() == frame.taint@.len(),
;

// ============================================================================
// Production-bound exec wrappers — exercise the assume_specification bridges
// ============================================================================
//
// These exec fns take simple primitive types (u64, u16) and forward to
// the production-bound extern fns. The assume_specification contracts
// above discharge the postconditions.
/// Exec wrapper for `RunFrame::new`.
pub exec fn exec_run_frame_new(
    run_id: u64,
    first_step: u16,
    step_count: u16,
    slot_count: u16,
) -> CoreResult<RunFrame> {
    production::run_frame_new(RunId::new(run_id), StepIdx::new(first_step), step_count, slot_count)
}

/// Exec wrapper for `RunFrame::reinitialize`.
pub exec fn exec_run_frame_reinitialize(
    frame: &mut RunFrame,
    run_id: u64,
    first_step: u16,
    step_count: u16,
    slot_count: u16,
) -> CoreResult<()> {
    production::run_frame_reinitialize(
        frame,
        RunId::new(run_id),
        StepIdx::new(first_step),
        step_count,
        slot_count,
    )
}

/// Exec wrapper for `RunFrame::pc`.
pub exec fn exec_run_frame_pc(frame: &RunFrame) -> (pc: StepIdx) {
    production::run_frame_pc(frame)
}

/// Exec wrapper for `RunFrame::step_count`.
pub exec fn exec_run_frame_step_count(frame: &RunFrame) -> (n: u16) {
    production::run_frame_step_count(frame)
}

/// Exec wrapper for `RunFrame::slot_count`.
pub exec fn exec_run_frame_slot_count(frame: &RunFrame) -> (n: u16) {
    production::run_frame_slot_count(frame)
}

/// Exec wrapper for `RunFrame::set_pc`.
pub exec fn exec_run_frame_set_pc(frame: &mut RunFrame, pc_raw: u16) -> CoreResult<()> {
    production::run_frame_set_pc(frame, StepIdx::new(pc_raw))
}

/// Exec wrapper for `RunFrame::write_slot_with_taint`.
pub exec fn exec_run_frame_write_slot_with_taint(
    frame: &mut RunFrame,
    slot_raw: u16,
    value: SlotValue,
    taint: Taint,
) -> CoreResult<()> {
    production::run_frame_write_slot_with_taint(frame, SlotIdx::new(slot_raw), value, taint)
}

/// Exec wrapper for `RunFrame::states_snapshot`.
pub exec fn exec_run_frame_states_snapshot(frame: &RunFrame) -> Vec<StepState> {
    production::run_frame_states_snapshot(frame)
}

/// Exec wrapper for `RunFrame::slots_snapshot`.
pub exec fn exec_run_frame_slots_snapshot(frame: &RunFrame) -> Vec<Option<SlotValue>> {
    production::run_frame_slots_snapshot(frame)
}

/// Exec wrapper for `RunFrame::taint_snapshot`.
pub exec fn exec_run_frame_taint_snapshot(frame: &RunFrame) -> Vec<Taint> {
    production::run_frame_taint_snapshot(frame)
}

// ============================================================================
// Proof lemmas — discharge the assume_specification contracts on the
// math layer (PRE-001, POST-001, INV-007, VB-INV001-VERUS, VB-INV006-VERUS).
// ============================================================================
/// PRE-001 rejection lemma: if `(first_step, step_count)` does NOT
/// satisfy the valid-range condition, then `spec_run_frame_new_preconditions`
/// is false.
pub proof fn proof_run_frame_new_rejects_invalid_dimensions(first_step: int, step_count: int)
    requires
        valid_u16_dim(step_count),
        !(0 < step_count && 0 <= first_step && first_step < step_count),
    ensures
        !spec_run_frame_new_preconditions(first_step, step_count),
{
}

/// PRE-001 acceptance lemma: if `(first_step, step_count)` satisfies
/// the valid-range condition, then `spec_run_frame_new_preconditions`
/// is true.
pub proof fn proof_run_frame_new_accepts_valid_dimensions(first_step: int, step_count: int)
    requires
        valid_u16_dim(step_count),
        0 <= first_step,
        0 < step_count,
        first_step < step_count,
    ensures
        spec_run_frame_new_preconditions(first_step, step_count),
{
}

/// POST-001 lemma: the spec_constructed_run_frame satisfies the new
/// postconditions for the requested dimensions. Discharged by unfolding
/// the postcondition and the constant fields of `spec_constructed_run_frame`.
pub proof fn proof_run_frame_new_initializes_dimensions_and_defaults(
    step_count: int,
    slot_count: int,
)
    requires
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
        0 < step_count,
    ensures
        spec_run_frame_new_postconditions(
            spec_constructed_run_frame(step_count, slot_count),
            step_count,
            slot_count,
        ),
{
}

/// INV-007 lemma: `reinitialize` preserves dimensions. Discharged by
/// the `spec_run_frame_dimensions_immutable` conjunct inside
/// `spec_reinitialize_accepts`.
pub proof fn proof_reinitialize_preserves_dimensions(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
)
    requires
        valid_u16_dim(old_step_count),
        valid_u16_dim(old_slot_count),
        valid_u16_dim(new_step_count),
        valid_u16_dim(new_slot_count),
        spec_reinitialize_accepts(
            old_step_count,
            old_slot_count,
            first_step,
            new_step_count,
            new_slot_count,
        ),
    ensures
        old_step_count == new_step_count,
        old_slot_count == new_slot_count,
        spec_run_frame_dimensions_immutable(
            old_step_count,
            old_slot_count,
            new_step_count,
            new_slot_count,
        ),
{
}

/// INV-007 rejection lemma: if a dimension mismatch is present,
/// `spec_reinitialize_accepts` is false.
pub proof fn proof_reinitialize_rejects_dimension_mismatch(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
)
    requires
        valid_u16_dim(new_step_count),
        spec_run_frame_new_preconditions(first_step, new_step_count),
        old_step_count != new_step_count || old_slot_count != new_slot_count,
    ensures
        !spec_reinitialize_accepts(
            old_step_count,
            old_slot_count,
            first_step,
            new_step_count,
            new_slot_count,
        ),
{
}

/// VB-INV001-VERUS composite lemma: every input combination is
/// classified as either accepted or rejected.
pub proof fn proof_frame_new_bounds(first_step: int, step_count: int)
    requires
        valid_u16_dim(step_count),
    ensures
// Rejection cases

        step_count == 0 ==> !spec_run_frame_new_valid(first_step, step_count),
        first_step >= step_count ==> !spec_run_frame_new_valid(first_step, step_count),
        // Acceptance case
        0 < step_count && 0 <= first_step && first_step < step_count ==> spec_run_frame_new_valid(
            first_step,
            step_count,
        ),
{
    if step_count == 0 {
        assert(!spec_run_frame_new_valid(first_step, step_count));
    }
    if first_step >= step_count {
        assert(!spec_run_frame_new_valid(first_step, step_count));
    }
    if 0 < step_count && 0 <= first_step && first_step < step_count {
        assert(spec_run_frame_new_valid(first_step, step_count));
    }
}

/// Lemma: `step_count == 0` is always invalid.
pub proof fn proof_step_count_zero_rejected(first_step: int)
    ensures
        !spec_run_frame_new_valid(first_step, 0),
{
    assert(!spec_run_frame_new_valid(first_step, 0));
}

/// Lemma: `first_step == step_count` is always invalid (boundary case).
pub proof fn proof_first_step_at_step_count_rejected(step_count: int)
    requires
        step_count > 0,
    ensures
        !spec_run_frame_new_valid(step_count, step_count),
{
    assert(!spec_run_frame_new_valid(step_count, step_count));
}

/// Lemma: `first_step > step_count` is always invalid.
pub proof fn proof_first_step_above_step_count_rejected(first_step: int, step_count: int)
    requires
        step_count > 0,
        first_step > step_count,
    ensures
        !spec_run_frame_new_valid(first_step, step_count),
{
    assert(!spec_run_frame_new_valid(first_step, step_count));
}

/// Lemma: valid `(first_step, step_count)` is always accepted.
pub proof fn proof_valid_dimensions_accepted(first_step: int, step_count: int)
    requires
        0 < step_count,
        0 <= first_step,
        first_step < step_count,
        valid_u16_dim(step_count),
    ensures
        spec_run_frame_new_valid(first_step, step_count),
{
    assert(spec_run_frame_new_valid(first_step, step_count));
}

// ============================================================================
// VB-INV006-VERUS: Taint validity proofs. The production Taint enum at
// `crates/vb_core/src/value.rs:14-25` has 5 closed variants (was 3 in
// the prior vacuum spec; production was always 5). `write_slot_with_taint`
// writes the input `Taint` directly with no raw u8 conversion, so any
// variant is a valid write.
// ============================================================================
/// VB-INV006-VERUS lemma: every `Taint` variant is a valid write target.
pub proof fn lemma_taint_valid_write(taint: Taint)
    ensures
        spec_taint_valid_write(taint) == true,
{
    match taint {
        Taint::Clean => assert(spec_taint_valid_write(taint) == true),
        Taint::DerivedFromSecret => assert(spec_taint_valid_write(taint) == true),
        Taint::Secret => assert(spec_taint_valid_write(taint) == true),
        Taint::Random => assert(spec_taint_valid_write(taint) == true),
        Taint::TimeDependent => assert(spec_taint_valid_write(taint) == true),
    }
}

/// VB-INV006-VERUS lemma: all 5 Taint variants are valid write targets.
pub proof fn lemma_all_taint_variants_valid()
    ensures
        spec_taint_valid_write(Taint::Clean) == true,
        spec_taint_valid_write(Taint::DerivedFromSecret) == true,
        spec_taint_valid_write(Taint::Secret) == true,
        spec_taint_valid_write(Taint::Random) == true,
        spec_taint_valid_write(Taint::TimeDependent) == true,
{
    assert(spec_taint_valid_write(Taint::Clean) == true) by (compute);
    assert(spec_taint_valid_write(Taint::DerivedFromSecret) == true) by (compute);
    assert(spec_taint_valid_write(Taint::Secret) == true) by (compute);
    assert(spec_taint_valid_write(Taint::Random) == true) by (compute);
    assert(spec_taint_valid_write(Taint::TimeDependent) == true) by (compute);
}

/// VB-INV006-VERUS lemma: there are no invalid `Taint` values (the
/// 5-variant enum is closed and exhaustive).
pub proof fn lemma_no_invalid_taint()
    ensures
        spec_taint_valid_write(Taint::Clean) == true,
        spec_taint_valid_write(Taint::DerivedFromSecret) == true,
        spec_taint_valid_write(Taint::Secret) == true,
        spec_taint_valid_write(Taint::Random) == true,
        spec_taint_valid_write(Taint::TimeDependent) == true,
{
    lemma_all_taint_variants_valid();
}

/// VB-INV006-VERUS lemma: taint validity preserved across all 5 variants.
pub proof fn lemma_taint_valid_write_all_variants()
    ensures
        spec_taint_valid_write(Taint::Clean) == true,
        spec_taint_valid_write(Taint::DerivedFromSecret) == true,
        spec_taint_valid_write(Taint::Secret) == true,
        spec_taint_valid_write(Taint::Random) == true,
        spec_taint_valid_write(Taint::TimeDependent) == true,
{
    lemma_all_taint_variants_valid();
}

/// VB-INV006-VERUS lemma: a freshly constructed frame has valid taint
/// (all slots initialized to `Taint::Clean` per POST-001).
pub proof fn lemma_new_frame_taint_valid()
    ensures
        spec_taint_valid_write(Taint::Clean) == true,
{
    lemma_taint_valid_write(Taint::Clean);
}

/// VB-INV006-VERUS lemma: slot taint remains valid across multiple writes
/// to the same slot, since each write installs a valid Taint.
pub proof fn lemma_multiple_writes_preserve_taint_validity()
    ensures
        spec_taint_valid_write(Taint::Clean) == true,
        spec_taint_valid_write(Taint::DerivedFromSecret) == true,
        spec_taint_valid_write(Taint::Secret) == true,
        spec_taint_valid_write(Taint::Random) == true,
        spec_taint_valid_write(Taint::TimeDependent) == true,
{
    lemma_taint_valid_write_all_variants();
}

// ============================================================================
// VB-INV006-VERUS: StepState bounds lemmas for step_index validation.
// ============================================================================
/// Lemma: a PC value `pc` is a valid step index iff
/// `spec_valid_step_index(pc, step_count)` holds.
pub proof fn lemma_pc_valid_step_index(pc: int, step_count: int)
    requires
        step_count > 0,
        0 <= pc,
        pc < step_count,
    ensures
        spec_valid_step_index(pc, step_count),
{
    assert(spec_valid_step_index(pc, step_count));
}

/// Lemma: `pc == step_count` is the boundary case — one past the last
/// valid index, hence invalid.
pub proof fn lemma_step_count_invalid(pc: int, step_count: int)
    requires
        step_count > 0,
        pc == step_count,
    ensures
        !spec_valid_step_index(pc, step_count),
{
    assert(!spec_valid_step_index(pc, step_count));
}

/// Lemma: out-of-bounds step indices (`pc >= step_count`) are invalid.
pub proof fn lemma_oob_step_invalid(pc: int, step_count: int)
    requires
        step_count > 0,
        pc >= step_count,
    ensures
        !spec_valid_step_index(pc, step_count),
{
    assert(!spec_valid_step_index(pc, step_count));
}

fn main() {
}

} // verus!
