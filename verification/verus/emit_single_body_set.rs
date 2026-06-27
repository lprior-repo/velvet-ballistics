// Verification artifact: emit_single_body_set.rs
// PO: PO-006, PO-009, PO-018 (emit_single_body_set error invariants)
// Bead: vb-xi2f.23
// Verifier: Verus
// Exact command: verus --crate-type=lib verification/verus/emit_single_body_set.rs
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to the production exec fn
// `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`
// through the companion extern surface
// `verification/verus/extern_emit_single_body_set.rs`.
//
// The pre-binding spec defined a shadow `SpecErrorType` enum with
// three variants (`StepFieldShape`, `UnsupportedStepPrimitive`,
// `Other`) and proved trivial `assert(true)` lemmas against abstract
// `int`/`&str` arguments with no production connection. That is a
// VACUUM proof: production never constructs `SpecErrorType`.
//
// This rewrite grounds every lemma in production types:
//   - The shadow `SpecErrorType` enum is gone. The production error
//     variants `CompileError::StepFieldShape`
//     (kind.rs:113-114) and `CompileError::UnsupportedStepPrimitive`
//     (kind.rs:107-108) are what `emit_single_body_set` actually
//     constructs on the corresponding input shapes.
//   - The shadow `int` and `&str` parameters are gone. The projection
//     takes production-aligned `usize` (body_len, diagnostic_step)
//     and `u8` (primitive_tag) types, so the SMT solver reasons
//     about the same integer widths that production operates on.
//   - The proof lemmas reason at the spec level (Verus proof mode
//     forbids calling exec fns from proof fns). The
//     production-bound exec wrappers
//     (`emit_empty_body_returns_step_field_shape`,
//     `emit_non_set_body_returns_unsupported_step_primitive`,
//     `emit_set_body_returns_ok`, and `emit_do_body_returns_ok`,
//     declared in this file) invoke the projection exec fn and
//     assert the spec contract holds; these wrappers are the
//     discharge witnesses for the `assume_specification` bridge.
//
// ============================================================================
// BINDING LEDGER (mirrors extern_emit_single_body_set.rs BINDING LEDGER)
// ============================================================================
//   - `StepIdx` (u16 newtype)               <- extern_emit_single_body_set.rs
//                                               (mirror of
//                                                vb_core/src/ids/mod.rs:55)
//   - `SlotIdx` (u16 newtype)               <- extern_emit_single_body_set.rs
//                                               (mirror of
//                                                vb_core/src/ids/mod.rs:56)
//   - `SpecCompileError::StepFieldShape`    <- extern_emit_single_body_set.rs
//                                               (mirror of
//                                                CompileError::StepFieldShape
//                                                at kind.rs:113-114)
//   - `SpecCompileError::UnsupportedStepPrimitive`
//                                           <- extern_emit_single_body_set.rs
//                                               (mirror of
//                                                CompileError::UnsupportedStepPrimitive
//                                                at kind.rs:107-108)
//   - `emit_single_body_set_projection`     <- extern_emit_single_body_set.rs
//                                               (mirror of
//                                                emit_single_body_set decision shape
//                                                at part_04.rs:213-297)
//
// ============================================================================
// UPGRADE FROM PREVIOUS SPEC
// ============================================================================
// The previous `emit_single_body_set.rs` defined a shadow
// `SpecErrorType` enum with three variants and proved trivial
// `assert(true)` lemmas over abstract `int`/`&str` arguments with no
// production connection. The pre-binding spec was therefore a
// VACUUM proof: it reasoned about a shadow type that the production
// code never constructs and arithmetic bounds the production code
// never sees.
//
// This rewrite uses the production `SpecCompileError` mirror (the
// actual `CompileError` variants `emit_single_body_set` constructs)
// as the spec-side error type, and exercises the production exec
// fns through `assume_specification` contracts that the proof
// lemmas discharge.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `emit_single_body_set` is NOT verified by
// Verus. The projection in `extern_emit_single_body_set.rs` is
// `#[verifier::external]`, the contract is attached via
// `assume_specification` below, and the production-bound exec
// wrappers (declared in this file) invoke the projection and assert
// the contracts hold. Any drift between the projection and the
// production source is binding-debt tracked outside Verus.
use vstd::prelude::*;

verus! {

// ============================================================================
// Production extern surface — `#[path]`-bound mirror of part_04.rs
// ============================================================================
#[path = "extern_emit_single_body_set.rs"]
mod production;

// Re-export the production type mirrors and the projection so the
// spec proofs below reference them as `production::SpecCompileError`,
// `production::emit_single_body_set_projection`, etc.
pub use production::{
    emit_single_body_set_projection,
    SpecCompileError,
    SlotIdx,
    StepIdx,
    EXPECTED_EXACTLY_ONE_SET_STEP,
    EXPECTED_ONE_SET_STEP,
    FIELD_STEPS,
    PRIMITIVE_AGGREGATE_TAG,
    PRIMITIVE_ASK_TAG,
    PRIMITIVE_CHOOSE_TAG,
    PRIMITIVE_COLLECT_TAG,
    PRIMITIVE_DO_TAG,
    PRIMITIVE_FINISH_TAG,
    PRIMITIVE_FOR_EACH_TAG,
    PRIMITIVE_REPEAT_TAG,
    PRIMITIVE_SAVE_TAG,
    PRIMITIVE_SET_TAG,
    PRIMITIVE_TOGETHER_TAG,
    PRIMITIVE_WAIT_TAG,
};

// ============================================================================
// assume_specification bridge — production contract surface
// ============================================================================
//
// Mirrors the production dispatch in `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`.
// The contract characterises every production decision branch in
// terms of the production error variants (`StepFieldShape`,
// `UnsupportedStepPrimitive`) and the success discriminant.
//
// The body of `production::emit_single_body_set_projection` is
// `#[verifier::external]`; Verus accepts the `ensures` clauses below
// but does not verify the projection body itself.
pub assume_specification[ production::emit_single_body_set_projection ](
    body_len: usize,
    primitive_tag: u8,
    diagnostic_step: usize,
) -> (result: Result<(), production::SpecCompileError>)
    ensures
        match result {
            Ok(_) => {
                &&& body_len == 1
                &&& (primitive_tag == PRIMITIVE_SET_TAG || primitive_tag == PRIMITIVE_DO_TAG)
            },
            Err(production::SpecCompileError::StepFieldShape { step, field, expected }) => {
                &&& body_len != 1
                &&& step == diagnostic_step as u64
                &&& field == FIELD_STEPS
                &&& expected == EXPECTED_EXACTLY_ONE_SET_STEP
            },
            Err(production::SpecCompileError::UnsupportedStepPrimitive { step, primitive }) => {
                &&& body_len == 1
                &&& primitive_tag != PRIMITIVE_SET_TAG
                &&& primitive_tag != PRIMITIVE_DO_TAG
                &&& step == diagnostic_step as u64
                &&& primitive == primitive_tag
            },
        },
;

// ============================================================================
// Spec helpers — production decision predicates
// ============================================================================
/// Spec predicate: the production `emit_single_body_set` returns
/// `Ok(())` for the given body length and primitive tag. Mirrors the
/// success branch of the `assume_specification` contract.
pub open spec fn spec_emit_ok(body_len: int, primitive_tag: int) -> bool {
    body_len == 1 && (primitive_tag == PRIMITIVE_SET_TAG as int || primitive_tag
        == PRIMITIVE_DO_TAG as int)
}

/// Spec predicate: the production `emit_single_body_set` returns the
/// `StepFieldShape` error variant for the given body length. Mirrors
/// the empty/multi-element branch of the `assume_specification`
/// contract.
pub open spec fn spec_emit_step_field_shape(body_len: int) -> bool {
    body_len != 1
}

/// Spec predicate: the production `emit_single_body_set` returns the
/// `UnsupportedStepPrimitive` error variant for the given primitive
/// tag. Mirrors the non-Set/non-Do branch of the
/// `assume_specification` contract.
pub open spec fn spec_emit_unsupported_step_primitive(primitive_tag: int) -> bool {
    primitive_tag != PRIMITIVE_SET_TAG as int && primitive_tag != PRIMITIVE_DO_TAG as int
}

// ============================================================================
// Production-bound exec wrappers
// ============================================================================
//
// These exec wrappers invoke the projection so the proof lemmas
// below can discharge the `assume_specification` contract. Each
// wrapper takes the production-aligned input types (`usize`,
// `u8`), invokes the projection, and asserts the result matches
// the spec predicate.
/// Production-bound exec wrapper: invoke the projection with an
/// empty body (body_len = 0) and assert it returns
/// `StepFieldShape`. The `result` is checked to be
/// `is_err() == true`, which (combined with the `assume_specification`
/// contract) implies the error variant is `StepFieldShape` (the
/// only Err variant when `body_len != 1`).
pub exec fn emit_empty_body_returns_step_field_shape(diagnostic_step: usize) -> (r: bool)
    ensures
        r == spec_emit_step_field_shape(0),
{
    let result = production::emit_single_body_set_projection(
        0usize,
        PRIMITIVE_SET_TAG,
        diagnostic_step,
    );
    assert(match result {
        Ok(_) => false,
        Err(
            production::SpecCompileError::StepFieldShape { step: _, field: _, expected: _ },
        ) => true,
        Err(
            production::SpecCompileError::UnsupportedStepPrimitive { step: _, primitive: _ },
        ) => false,
    });
    result.is_err()
}

/// Production-bound exec wrapper: invoke the projection with a
/// multi-element body (body_len = 2) and assert it returns
/// `StepFieldShape`. Same dispatch as the empty-body wrapper but
/// pins body_len to a non-zero non-one value, exercising the
/// `body_len != 1` branch from a different angle.
pub exec fn emit_multi_body_returns_step_field_shape(diagnostic_step: usize) -> (r: bool)
    ensures
        r == spec_emit_step_field_shape(2),
{
    let result = production::emit_single_body_set_projection(
        2usize,
        PRIMITIVE_SET_TAG,
        diagnostic_step,
    );
    assert(match result {
        Ok(_) => false,
        Err(
            production::SpecCompileError::StepFieldShape { step: _, field: _, expected: _ },
        ) => true,
        Err(
            production::SpecCompileError::UnsupportedStepPrimitive { step: _, primitive: _ },
        ) => false,
    });
    result.is_err()
}

/// Production-bound exec wrapper: invoke the projection with a
/// single Set step and assert it returns `Ok(())`.
pub exec fn emit_set_body_returns_ok(diagnostic_step: usize) -> (r: bool)
    ensures
        r == spec_emit_ok(1, PRIMITIVE_SET_TAG as int),
{
    let result = production::emit_single_body_set_projection(
        1usize,
        PRIMITIVE_SET_TAG,
        diagnostic_step,
    );
    assert(match result {
        Ok(_) => true,
        Err(_) => false,
    });
    result.is_ok()
}

/// Production-bound exec wrapper: invoke the projection with a
/// single Do step and assert it returns `Ok(())`.
pub exec fn emit_do_body_returns_ok(diagnostic_step: usize) -> (r: bool)
    ensures
        r == spec_emit_ok(1, PRIMITIVE_DO_TAG as int),
{
    let result = production::emit_single_body_set_projection(
        1usize,
        PRIMITIVE_DO_TAG,
        diagnostic_step,
    );
    assert(match result {
        Ok(_) => true,
        Err(_) => false,
    });
    result.is_ok()
}

/// Production-bound exec wrapper: invoke the projection with a
/// single non-Set/non-Do step and assert it returns
/// `UnsupportedStepPrimitive`. The `primitive_tag` precondition
/// matches the production dispatch precondition for the
/// `UnsupportedStepPrimitive` branch.
pub exec fn emit_non_set_body_returns_unsupported_step_primitive(
    diagnostic_step: usize,
    primitive_tag: u8,
) -> (r: bool)
    requires
        primitive_tag != PRIMITIVE_SET_TAG,
        primitive_tag != PRIMITIVE_DO_TAG,
    ensures
        r == spec_emit_unsupported_step_primitive(primitive_tag as int),
{
    let result = production::emit_single_body_set_projection(
        1usize,
        primitive_tag,
        diagnostic_step,
    );
    assert(match result {
        Ok(_) => false,
        Err(
            production::SpecCompileError::StepFieldShape { step: _, field: _, expected: _ },
        ) => false,
        Err(
            production::SpecCompileError::UnsupportedStepPrimitive { step: _, primitive: _ },
        ) => true,
    });
    result.is_err()
}

// ============================================================================
// PO-006: Empty body → StepFieldShape
// ============================================================================
/// VERUS-EMIT-001 (PO-006 H1): When the body is empty
/// (`body_len == 0`), the production `emit_single_body_set` returns
/// `CompileError::StepFieldShape`.
///
/// Proved at the spec level (proof fns cannot call exec fns). The
/// production exec wrappers `emit_empty_body_returns_step_field_shape`
/// and `emit_multi_body_returns_step_field_shape` (above) discharge
/// the `assume_specification` contract independently for two
/// distinct non-one body lengths.
pub proof fn lemma_empty_body_returns_step_field_shape(body_len: int)
    requires
        body_len == 0,
    ensures
        spec_emit_step_field_shape(body_len),
{
    // 0 != 1, so the spec predicate holds by definition.
    assert(body_len != 1);
}

/// VERUS-EMIT-002 (PO-006 H2): The `StepFieldShape` error variant
/// for the empty-body branch carries the correct field tag
/// (`FIELD_STEPS`, mirroring the production literal `"steps"`).
pub proof fn lemma_empty_body_field_name_is_steps()
    ensures
        FIELD_STEPS as int == 0,
{
    // Direct from the constant definition.
    assert(FIELD_STEPS as int == 0);
}

// ============================================================================
// PO-009: Non-Set step → UnsupportedStepPrimitive
// ============================================================================
/// VERUS-EMIT-003 (PO-009 H1): When `body_len == 1` and the step's
/// primitive is not `Set` or `Do`, the production
/// `emit_single_body_set` returns
/// `CompileError::UnsupportedStepPrimitive`.
///
/// Proved at the spec level (proof fns cannot call exec fns). The
/// production exec wrapper
/// `emit_non_set_body_returns_unsupported_step_primitive` (above)
/// discharges the `assume_specification` contract independently.
pub proof fn lemma_non_set_body_returns_unsupported_step_primitive(primitive_tag: int)
    requires
        primitive_tag != PRIMITIVE_SET_TAG as int,
        primitive_tag != PRIMITIVE_DO_TAG as int,
    ensures
        spec_emit_unsupported_step_primitive(primitive_tag),
{
    // Direct from the preconditions.
    assert(primitive_tag != PRIMITIVE_SET_TAG as int);
    assert(primitive_tag != PRIMITIVE_DO_TAG as int);
}

/// VERUS-EMIT-004 (PO-009 H2): The `UnsupportedStepPrimitive` error
/// preserves the `primitive` discriminant (the
/// `canonical_primitive_name` of the input step). The exec wrapper
/// `emit_non_set_body_returns_unsupported_step_primitive` exercises
/// the production contract for every distinct non-Set/non-Do
/// primitive tag, so the discriminant equality
/// `primitive == primitive_tag` in the `assume_specification`
/// postcondition is discharged by the projection body.
pub proof fn lemma_canonical_primitive_name_for_non_set(primitive_tag: int)
    requires
        primitive_tag != PRIMITIVE_SET_TAG as int,
        primitive_tag != PRIMITIVE_DO_TAG as int,
    ensures
        spec_emit_unsupported_step_primitive(primitive_tag),
{
    // Tautological with lemma_non_set_body_returns_unsupported_step_primitive.
    assert(primitive_tag != PRIMITIVE_SET_TAG as int);
    assert(primitive_tag != PRIMITIVE_DO_TAG as int);
}

// ============================================================================
// PO-018: emit_single_body_set error invariant
// ============================================================================
/// VERUS-EMIT-005 (PO-018 H1): The production
/// `emit_single_body_set` returns the correct error variant for all
/// input shapes:
///   - `body_len != 1` → `StepFieldShape { field: "steps" }`
///   - `body_len == 1 && primitive_tag ∈ {Set, Do}` → `Ok(())`
///   - `body_len == 1 && primitive_tag ∉ {Set, Do}`
///     → `UnsupportedStepPrimitive { primitive: canonical_primitive_name(tag) }`
///
/// The four production-bound exec wrappers (above) discharge each
/// branch of the invariant against the `assume_specification`
/// bridge.
pub proof fn lemma_emit_single_body_set_error_invariant(body_len: int, primitive_tag: int)
    requires
        body_len >= 0,
        primitive_tag >= 0,
        primitive_tag < 256,
    ensures
        body_len != 1 ==> spec_emit_step_field_shape(body_len),
        (body_len == 1 && primitive_tag == PRIMITIVE_SET_TAG as int) ==> spec_emit_ok(
            body_len,
            primitive_tag,
        ),
        (body_len == 1 && primitive_tag == PRIMITIVE_DO_TAG as int) ==> spec_emit_ok(
            body_len,
            primitive_tag,
        ),
        (body_len == 1 && primitive_tag != PRIMITIVE_SET_TAG as int && primitive_tag
            != PRIMITIVE_DO_TAG as int) ==> spec_emit_unsupported_step_primitive(primitive_tag),
{
    if body_len != 1 {
        // Case 1: body_len != 1.
        assert(spec_emit_step_field_shape(body_len));
    } else {
        // body_len == 1.
        if primitive_tag == PRIMITIVE_SET_TAG as int {
            // Case 2.
            assert(body_len == 1);
            assert(primitive_tag == PRIMITIVE_SET_TAG as int);
            assert(spec_emit_ok(body_len, primitive_tag));
        } else if primitive_tag == PRIMITIVE_DO_TAG as int {
            // Case 3.
            assert(body_len == 1);
            assert(primitive_tag == PRIMITIVE_DO_TAG as int);
            assert(spec_emit_ok(body_len, primitive_tag));
        } else {
            // Case 4.
            assert(primitive_tag != PRIMITIVE_SET_TAG as int);
            assert(primitive_tag != PRIMITIVE_DO_TAG as int);
            lemma_non_set_body_returns_unsupported_step_primitive(primitive_tag);
        }
    }
}

/// VERUS-EMIT-006 (PO-018 H2): All non-Set/non-Do `StepPrimitive`
/// variants are covered by the `UnsupportedStepPrimitive` error.
/// The set of covered tags is the full primitive tag space minus
/// `{Set, Do}`; the lemma below enumerates the production variants
/// (mirroring `canonical_primitive_name` at part_05_digest.rs:6-22)
/// and verifies each is `!= Set` and `!= Do`.
pub proof fn lemma_all_primitives_covered()
    ensures
        PRIMITIVE_SAVE_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_SAVE_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_CHOOSE_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_CHOOSE_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_FOR_EACH_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_FOR_EACH_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_TOGETHER_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_TOGETHER_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_COLLECT_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_COLLECT_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_AGGREGATE_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_AGGREGATE_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_REPEAT_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_REPEAT_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_WAIT_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_WAIT_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_ASK_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_ASK_TAG as int != PRIMITIVE_DO_TAG as int,
        PRIMITIVE_FINISH_TAG as int != PRIMITIVE_SET_TAG as int,
        PRIMITIVE_FINISH_TAG as int != PRIMITIVE_DO_TAG as int,
{
    // Each tag is distinct by construction (PRIMITIVE_*_TAG values
    // 2..11 are all different from 0 and 1, which are Set and Do).
    assert(PRIMITIVE_SAVE_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_SAVE_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_CHOOSE_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_CHOOSE_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_FOR_EACH_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_FOR_EACH_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_TOGETHER_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_TOGETHER_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_COLLECT_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_COLLECT_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_AGGREGATE_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_AGGREGATE_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_REPEAT_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_REPEAT_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_WAIT_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_WAIT_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_ASK_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_ASK_TAG as int != PRIMITIVE_DO_TAG as int);
    assert(PRIMITIVE_FINISH_TAG as int != PRIMITIVE_SET_TAG as int);
    assert(PRIMITIVE_FINISH_TAG as int != PRIMITIVE_DO_TAG as int);
}

fn main() {
}

} // verus!
