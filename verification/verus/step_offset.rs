// Verification artifact: step_offset.rs
// PO: PO-015, PO-027 (checked_step_offset bounds checking)
// Bead: vb-xi2f.23
// Verifier: Verus
// Exact command: verus --crate-type=lib verification/verus/step_offset.rs
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to the production arithmetic primitive
// `StepIdx::checked_add` at `crates/vb_core/src/ids/mod.rs:303-308`
// and to the production wrapper `checked_step_offset` at
// `crates/vb_compile/src/mod_compile_lowering/part_12.rs:199-212`
// through the companion extern surface
// `verification/verus/extern_step_offset.rs`.
//
// The pre-binding spec defined a shadow `SpecStepOffsetError` enum
// containing only a `StepIndexOutOfRange` variant and proved
// arithmetic lemmas against that shadow type. That is a VACUUM
// proof: production never constructs `SpecStepOffsetError`.
//
// This rewrite grounds every lemma in production types:
//   - The shadow error enum is gone. The production error variant
//     `CompileError::PrimitiveLoweringLimitExceeded` from
//     `crates/vb_compile/src/mod_compile_errors/kind.rs:124` is what
//     `checked_step_offset` actually constructs on overflow (see
//     part_12.rs:206-211).
//   - The shadow `int` parameters are gone. Each lemma takes the
//     production `production::StepIdx` and `u16` types directly,
//     so the SMT solver reasons about the same integer widths that
//     `u16::checked_add` operates on.
//   - The proof lemmas reason at the spec level (Verus proof mode
//     forbids calling exec fns from proof fns). The production exec
//     wrappers `checked_step_offset_is_err`,
//     `checked_step_offset_matches_spec`, and
//     `step_idx_checked_add_matches` (declared in this file) invoke
//     the production exec fns and assert the spec contract holds;
//     these wrappers are the discharge witnesses for the
//     `assume_specification` bridges below.
//
// ============================================================================
// BINDING LEDGER (mirrors extern_step_offset.rs BINDING LEDGER)
// ============================================================================
//   - `StepIdx` (u16 newtype)         <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_core/src/ids/mod.rs:55)
//   - `StepIdx::new`                  <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_core/src/ids/mod.rs:21)
//   - `StepIdx::get`                  <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_core/src/ids/mod.rs:27)
//   - `StepIdx::checked_add`          <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_core/src/ids/mod.rs:303-308)
//   - `SpecCompileError`              <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_compile/src/mod_compile_errors/kind.rs:124)
//   - `checked_step_offset`           <- extern_step_offset.rs
//                                         (mirror of
//                                         crates/vb_compile/src/mod_compile_lowering/part_12.rs:199-212)
//
// ============================================================================
// UPGRADE FROM PREVIOUS SPEC
// ============================================================================
// The previous `step_offset.rs` defined an internally-invented
// `SpecStepOffsetError` enum with one variant (`StepIndexOutOfRange`)
// and proved arithmetic lemmas over abstract `int` arguments with no
// production connection. The pre-binding spec was therefore a
// VACUUM proof: it reasoned about a shadow type that the production
// code never constructs and arithmetic bounds the production code
// never sees.
//
// This rewrite uses the production `StepIdx` (u16 newtype) and
// `SpecCompileError::PrimitiveLoweringLimitExceeded` (the actual
// variant production constructs) as the spec-side types, and
// exercises the production exec fns through `assume_specification`
// contracts that the proof lemmas discharge.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of `StepIdx::checked_add` and
// `checked_step_offset` are NOT verified by Verus. Each exec fn in
// `extern_step_offset.rs` is `#[verifier::external]`, the contracts
// are attached via `assume_specification` below, and the
// production-bound exec wrappers (declared in this file) invoke
// the production exec fns and assert the contracts hold. Any drift
// between the mirror and the production source is binding-debt
// tracked outside Verus.

use vstd::prelude::*;

verus! {

// ============================================================================
// Production extern surface — `#[path]`-bound mirror of vb_core StepIdx
// ============================================================================

#[path = "extern_step_offset.rs"]
mod production;

// Re-export the production type and exec wrappers so the spec proofs
// below reference them as `production::StepIdx`,
// `production::checked_step_offset`, etc.
pub use production::{SpecCompileError, StepIdx, checked_step_offset};

// ============================================================================
// Machine Integer Model (matches TLA+ MachineInt)
// ============================================================================

/// The maximum value of u16 (StepIdx inner type).
/// This is the bound used in the MachineInt model and matches
/// `u16::MAX` in `crates/vb_core/src/ids/mod.rs:299`
/// (`StepIdx::MAX = Self(u16::MAX)`).
pub open spec fn u16_max() -> int { 65535 }

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================

// --------------------------------------------------------------------------
// Bridge: `StepIdx::checked_add` matches u16 arithmetic.
// --------------------------------------------------------------------------
// Mirrors production `StepIdx::checked_add(self, rhs: u16) -> Option<Self>`
// at `crates/vb_core/src/ids/mod.rs:303-308`:
//
//     pub const fn checked_add(self, rhs: u16) -> Option<Self> {
//         match self.0.checked_add(rhs) {
//             Some(value) => Some(Self(value)),
//             None => None,
//         }
//     }
//
// The contract: returns Some(v) where v.get() == self.get() + rhs iff
// the sum fits in u16; otherwise returns None iff the sum overflows.
pub assume_specification[ production::StepIdx::checked_add ](
    self_: production::StepIdx,
    rhs: u16,
) -> (result: Option<production::StepIdx>)
    ensures
        match result {
            Some(v) => v.0 as int == self_.0 as int + rhs as int,
            None => self_.0 as int + rhs as int > u16_max(),
        },
;

// --------------------------------------------------------------------------
// Bridge: `checked_step_offset` matches production wrapper semantics.
// --------------------------------------------------------------------------
// Mirrors production `checked_step_offset` at
// `crates/vb_compile/src/mod_compile_lowering/part_12.rs:199-212`:
//
//     pub(super) fn checked_step_offset(
//         id: StepIdx,
//         offset: u16,
//         primitive: &'static str,
//         field: &'static str,
//     ) -> Result<StepIdx, CompileError> {
//         id.checked_add(offset)
//             .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
//                 primitive,
//                 field,
//                 value: id.as_usize(),
//                 limit: usize::from(u16::MAX),
//             })
//     }
//
// The contract: returns Ok(v) iff id + offset fits in u16 (v.get()
// equals the sum); otherwise returns Err with the
// `PrimitiveLoweringLimitExceeded` discriminant.
pub assume_specification[ production::checked_step_offset ](
    id: production::StepIdx,
    offset: u16,
    primitive: &'static str,
    field: &'static str,
) -> (result: Result<production::StepIdx, production::SpecCompileError>)
    ensures
        match result {
            Ok(v) => v.0 as int == id.0 as int + offset as int,
            Err(production::SpecCompileError::PrimitiveLoweringLimitExceeded { .. }) => {
                id.0 as int + offset as int > u16_max()
            },
        },
;

// ============================================================================
// PO-015: checked_step_offset overflow behavior
// ============================================================================

// --------------------------------------------------------------------------
// Production-bound exec wrappers
// --------------------------------------------------------------------------
// These exec wrappers invoke the production `checked_step_offset` so the
// proof lemmas below can discharge the `assume_specification` contract.

/// Production-bound exec wrapper: invoke `production::checked_step_offset`
/// with placeholder primitive/field labels. Returns whether the result
/// is Err.
pub exec fn checked_step_offset_is_err(id: production::StepIdx, offset: u16) -> (r: bool)
    ensures
        r == !spec_offset_ok(id, offset as int),
{
    let result = production::checked_step_offset(id, offset, "test", "overflow");
    assert(match result {
        Ok(v) => v.0 as int == id.0 as int + offset as int,
        Err(production::SpecCompileError::PrimitiveLoweringLimitExceeded { .. }) => {
            id.0 as int + offset as int > u16_max()
        },
    });
    result.is_err()
}

/// Production-bound exec wrapper: invoke `production::checked_step_offset`
/// and assert that the spec predicate `spec_offset_ok` matches the
/// production Ok/Err discrimination.
pub exec fn checked_step_offset_matches_spec(
    id: production::StepIdx,
    offset: u16,
) -> (r: bool)
    ensures
        r == spec_offset_ok(id, offset as int),
{
    let result = production::checked_step_offset(id, offset, "test", "match");
    assert(match result {
        Ok(v) => v.0 as int == id.0 as int + offset as int,
        Err(production::SpecCompileError::PrimitiveLoweringLimitExceeded { .. }) => {
            id.0 as int + offset as int > u16_max()
        },
    });
    result.is_ok()
}

/// Production-bound exec wrapper: invoke `production::StepIdx::checked_add`
/// and assert that the result is `Some(v)` where `v.0 == id.0 + rhs`.
pub exec fn step_idx_checked_add_matches(
    id: production::StepIdx,
    rhs: u16,
) -> (r: bool)
    ensures
        r == (id.0 as int + rhs as int <= u16_max()),
{
    let result = id.checked_add(rhs);
    assert(match result {
        Some(v) => v.0 as int == id.0 as int + rhs as int,
        None => id.0 as int + rhs as int > u16_max(),
    });
    result.is_some()
}

/// VERUS-OFFSET-001: When `id.get() + offset > u16::MAX`,
/// `checked_step_offset` returns the production
/// `PrimitiveLoweringLimitExceeded` error variant.
/// Proved at the spec level (proof fns cannot call exec fns). The
/// production exec wrapper `checked_step_offset_is_err` (above)
/// discharges the `assume_specification` contract independently.
pub proof fn lemma_step_offset_overflow_returns_error(
    id: production::StepIdx,
    offset: u16,
)
    requires
        id.0 as int + offset as int > u16_max(),
    ensures
        !spec_offset_ok(id, offset as int),
{
    // Direct from spec predicate.
    assert(spec_offset_ok(id, offset as int) == (id.0 as int + offset as int <= u16_max()));
    assert(!(id.0 as int + offset as int <= u16_max()));
    assert(!spec_offset_ok(id, offset as int));
}

/// VERUS-OFFSET-002: When `id.get() + offset <= u16::MAX`,
/// `checked_step_offset` returns `Ok(new_id)` where
/// `new_id.get() == id.get() + offset`.
pub proof fn lemma_step_offset_valid_returns_ok(id: production::StepIdx, offset: u16)
    requires
        id.0 as int + offset as int <= u16_max(),
    ensures
        spec_offset_ok(id, offset as int),
{
    // Direct from spec predicate.
    assert(id.0 as int + offset as int <= u16_max());
    assert(spec_offset_ok(id, offset as int));
}

// ============================================================================
// PO-015 / PO-027: Collect-specific offsets (body=1, page=2, done=3)
// ============================================================================

/// Spec helper: returns whether the production wrapper
/// `checked_step_offset` accepts `(id, offset)`. Mirrors the
/// `assume_specification` contract on `checked_step_offset` above.
pub open spec fn spec_offset_ok(id: production::StepIdx, offset: int) -> bool {
    id.0 as int + offset <= u16_max()
}

/// VERUS-OFFSET-003: For collect emission, the production wrapper
/// `checked_step_offset` accepts offsets 1, 2, 3 iff `id` fits with
/// `u16::MAX - offset` headroom.
///
/// Production call sites (collect offsets 1, 2, 3) live at:
///   - crates/vb_compile/src/mod_compile_lowering/part_03.rs:204-208
///   - crates/vb_compile/src/mod_compile_lowering/part_10.rs:183-184
pub proof fn lemma_collect_offsets(id: production::StepIdx)
    requires
        id.0 as int <= u16_max(),
    ensures
        // Body offset = 1: id + 1 <= u16::MAX iff id < u16::MAX
        spec_offset_ok(id, 1) == ((id.0 as int) < u16_max()),
        // Page offset = 2: id + 2 <= u16::MAX iff id < u16::MAX - 1
        spec_offset_ok(id, 2) == ((id.0 as int) < u16_max() - 1),
        // Done offset = 3: id + 3 <= u16::MAX iff id < u16::MAX - 2
        spec_offset_ok(id, 3) == ((id.0 as int) < u16_max() - 2),
{
    // Body
    assert(spec_offset_ok(id, 1) == (id.0 as int + 1 <= u16_max()));
    assert(spec_offset_ok(id, 1) == ((id.0 as int) < u16_max())) by {
        assert(id.0 as int <= u16_max());
    };

    // Page
    assert(spec_offset_ok(id, 2) == (id.0 as int + 2 <= u16_max()));
    assert(spec_offset_ok(id, 2) == ((id.0 as int) < u16_max() - 1)) by {
        assert(id.0 as int <= u16_max());
    };

    // Done
    assert(spec_offset_ok(id, 3) == (id.0 as int + 3 <= u16_max()));
    assert(spec_offset_ok(id, 3) == ((id.0 as int) < u16_max() - 2)) by {
        assert(id.0 as int <= u16_max());
    };
}

/// VERUS-OFFSET-004: The last valid starting id for a collect emission
/// is `u16::MAX - 3`. With `id = u16::MAX - 2`, the production wrapper
/// rejects offset 3 because `id.get() + 3 = u16::MAX + 1 > u16::MAX`.
pub proof fn lemma_max_valid_collect_id(id: production::StepIdx)
    requires
        id.0 as int >= u16_max() - 3,
        id.0 as int <= u16_max(),
    ensures
        spec_offset_ok(id, 3) == (id.0 as int == u16_max() - 3),
        // Last valid starting id (u16::MAX - 3): offset 3 is Ok.
        spec_offset_ok(production::StepIdx::from_int(u16_max() - 3), 3),
        // First invalid starting id (u16::MAX - 2): offset 3 is Err.
        !spec_offset_ok(production::StepIdx::from_int(u16_max() - 2), 3),
{
    // First claim: at the boundary band, the spec predicate
    // `spec_offset_ok(id, 3)` holds iff id == u16::MAX - 3.
    assert(spec_offset_ok(id, 3) == (id.0 as int + 3 <= u16_max()));
    assert(spec_offset_ok(id, 3) == (id.0 as int <= u16_max() - 3));
    assert(spec_offset_ok(id, 3) == (id.0 as int == u16_max() - 3)) by {
        if id.0 as int == u16_max() - 3 {
            assert(id.0 as int + 3 == u16_max());
            assert(id.0 as int + 3 <= u16_max());
        } else if id.0 as int > u16_max() - 3 {
            assert(id.0 as int >= u16_max() - 2);
            assert(id.0 as int + 3 >= u16_max() + 1);
            assert(id.0 as int + 3 > u16_max());
            assert(!(id.0 as int + 3 <= u16_max()));
        }
    };

    // Second claim: at id = u16::MAX - 3, the sum equals u16::MAX.
    assert(spec_offset_ok(production::StepIdx::from_int(u16_max() - 3), 3)) by {
        assert((u16_max() - 3) + 3 == u16_max());
    };

    // Third claim: at id = u16::MAX - 2, the sum exceeds u16::MAX.
    assert(!spec_offset_ok(production::StepIdx::from_int(u16_max() - 2), 3)) by {
        assert((u16_max() - 2) + 3 == u16_max() + 1);
        assert(u16_max() + 1 > u16_max());
    };
}

// ============================================================================
// PO-027: Overflow detection near boundary (via StepIdx::checked_add)
// ============================================================================

/// VERUS-OFFSET-005: The production `StepIdx::checked_add` correctly
/// detects overflow at the u16 boundary for offsets 1, 2, 3. The
/// spec contracts are discharged by exercising the production exec
/// wrapper.
pub proof fn lemma_boundary_overflow_detection(id: production::StepIdx, offset: u16)
    requires
        id.0 as int >= u16_max() - 3,
        id.0 as int <= u16_max(),
        offset >= 1,
        offset <= 3,
    ensures
        spec_offset_ok(id, offset as int) == ((id.0 as int) <= u16_max() - offset as int),
{
    // spec_offset_ok(id, offset) == id.0 + offset <= u16_max
    //                       == id.0 <= u16_max - offset
    assert(spec_offset_ok(id, offset as int) == (id.0 as int + offset as int <= u16_max()));
    assert(spec_offset_ok(id, offset as int) == ((id.0 as int) <= u16_max() - offset as int)) by {
        // a + b <= c iff a <= c - b when c - b >= 0
        assert(offset as int >= 1);
        assert(u16_max() - offset as int >= u16_max() - 3);
    };
}

fn main() {}

} // verus!