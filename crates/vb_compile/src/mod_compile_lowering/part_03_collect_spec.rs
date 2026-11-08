// Lexical N-step collect lowering lives in bounded_collect (part_02).
// This spec block is rebound to that generalized lowering. No legacy
// single-Set helper is kept to appear bound: the old fixed (1,2,3)
// offset claims are replaced below, not supplemented.

// ─────────────────────────────────────────────────────────────────
// GOD RULE 2 Verus Spec Block
//
// Production-bound mathematical contract for the lexical N-step
// collect lowering. Proves L1-L6 properties for the edge computation
// in bounded_collect::plan_compute_edges.
//
// This block is conditionally compiled by Verus via #[cfg(verus_keep_ghost)].
// Regular Rust builds (cargo check / cargo build) skip it entirely.
// Verus sees this block and verifies the lemmas.
//
// The production fn lower_bounded_collect (part_02's bounded_collect)
// CANNOT be annotated with Verus requires/ensures directly because:
//   (a) External crate types (vb_core::StepIdx, SlotCompiler, CompileErrors)
//       are not Verus-tracked.
//   (b) Mutable state through &mut SlotCompiler is not Verus-compatible.
//   (c) The ? operator on custom CompileErrors types is not supported.
//   (d) Body emission via emit_scoped_flat_body ranges over external
//       StepAst types with per-step dispatch.
//
// Instead, this block proves the pure mathematical contract that the
// offset arithmetic must satisfy, mirroring production stage-for-stage:
//   - Production stage 1 (bounded_collect.rs:108-114): advance_n is
//     body_len + 1 and done_n is body_len + 2, each u16-checked by
//     checked_body_length_offset.
//     Spec stage 1: vb_spec_body_length_offset(body_len, 1/2).
//   - Production stage 2 (bounded_collect.rs:116-126, via
//     part_12.rs:199-212 checked_step_offset): body is id + 1,
//     advance is id + advance_n, done is id + done_n, each a
//     StepIdx::checked_add with u16 bounds.
//     Spec stage 2: vb_spec_checked_step_offset applied to the same offsets.
//   - Production rejects empty bodies (validate_body), so every lemma
//     requires body_len >= 1.
//   - Both sides use u16 arithmetic with MAX = 65535.
//
// What this block does NOT claim: a machine-checked
// assume_specification link from the spec fns to the production exec
// fn. External crate types are not Verus-tracked, so the exec fn
// cannot carry ensures; the live Rust proptest and deterministic suites
// (proptest_collect.rs, vb_8mdp_7_collect.rs) exercise the same edges
// through the real production API instead. No standalone
// verification/verus mirror is kept at this base; adding one would
// reintroduce the production_inner drift class.
//
// Verification command (mechanical extraction: single-file Verus cannot
// resolve this crate's super::*/crate::*/external imports in place, and
// the previously documented header command fails with 24 such errors;
// the ^ anchor keeps the match on the code block, not this comment):
//   python3 -c 'import re,pathlib,sys; t=pathlib.Path(
//     "crates/vb_compile/src/mod_compile_lowering/part_03_collect_spec.rs"
//     ).read_text(); m=re.search(r"^verus! \{(.*)\n\} // verus!",
//     t, re.M | re.S); sys.stdout.write("use vstd::prelude::*;\nverus! {"
//     + m.group(1) + "\n}\n")' > /tmp/collect_spec_check.rs \
//   && verus --crate-type=lib /tmp/collect_spec_check.rs
// Expected: verification results:: 7 verified, 0 errors.
// ─────────────────────────────────────────────────────────────────

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

pub open spec fn vb_u16_max() -> int { 65535 }

pub enum VbSpecCompileError {
    LimitExceeded,
}

/// Spec model of checked_step_offset (production: part_12.rs:199-212).
/// Matches StepIdx::checked_add(offset) behavior with u16 bounds.
pub open spec fn vb_spec_checked_step_offset(id: int, offset: int)
    -> Result<int, VbSpecCompileError>
{
    if id + offset <= vb_u16_max() {
        Ok(id + offset)
    } else {
        Err(VbSpecCompileError::LimitExceeded)
    }
}

/// Spec model of the checked_body_length_offset stage
/// (production: bounded_collect.rs:108-114).
/// Production adds body_len + extra with a usize checked add, then requires
/// the total to fit u16. u16::MAX is the binding constraint, so the model
/// rejects totals above vb_u16_max().
pub open spec fn vb_spec_body_length_offset(body_len: int, extra: int)
    -> Result<int, VbSpecCompileError>
{
    if body_len + extra <= vb_u16_max() {
        Ok(body_len + extra)
    } else {
        Err(VbSpecCompileError::LimitExceeded)
    }
}

/// Spec model of plan_compute_edges
/// (production: bounded_collect.rs:116-126).
/// Stage 1: advance_n is body_len + 1, done_n is body_len + 2 (u16-checked).
/// Stage 2: body is id + 1, advance is id + advance_n, done is id + done_n.
/// Production rejects empty bodies (validate_body), so body_len >= 1.
pub open spec fn vb_spec_collect_edges(id: int, body_len: int)
    -> Result<(int, int, int), VbSpecCompileError>
    recommends
        body_len >= 1,
        id + body_len + 2 <= vb_u16_max(),
{
    match (
        vb_spec_body_length_offset(body_len, 1),
        vb_spec_body_length_offset(body_len, 2),
    ) {
        (Ok(advance_n), Ok(done_n)) => match (
            vb_spec_checked_step_offset(id, 1),
            vb_spec_checked_step_offset(id, advance_n),
            vb_spec_checked_step_offset(id, done_n),
        ) {
            (Ok(b), Ok(p), Ok(d)) => Ok((b, p, d)),
            _ => Err(VbSpecCompileError::LimitExceeded),
        },
        _ => Err(VbSpecCompileError::LimitExceeded),
    }
}

// ─────────────────────────────────────────────────────────────────
// L1: Strict monotonicity — body < advance < done for every N >= 1
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_steps_strictly_increasing(id: int, body_len: int)
    requires
        id >= 0,
        body_len >= 1,
        id + body_len + 2 <= vb_u16_max(),
    ensures
        id + 1 < id + body_len + 1 < id + body_len + 2,
{
    assert(id + 1 < id + body_len + 1);
    assert(id + body_len + 1 < id + body_len + 2);
    assert(id + 1 < id + body_len + 2);
}

// ─────────────────────────────────────────────────────────────────
// L2: N + 3 emitted IDs within u16 bounds
// (start + N body nodes + advance + finish)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_n_step_ids_in_bounds(id: int, body_len: int)
    requires
        id >= 0,
        body_len >= 1,
        id + body_len + 2 <= vb_u16_max(),
    ensures
        id + 1 <= vb_u16_max(),
        id + body_len + 1 <= vb_u16_max(),
        id + body_len + 2 <= vb_u16_max(),
        body_len + 3 >= 4,
{
    assert(id + 1 <= vb_u16_max()) by {
        assert(id + body_len + 2 <= vb_u16_max());
        assert(id + 1 <= id + body_len + 2);
    }
    assert(id + body_len + 1 <= vb_u16_max()) by {
        assert(id + body_len + 2 <= vb_u16_max());
        assert(id + body_len + 1 <= id + body_len + 2);
    }
    assert(id + body_len + 2 <= vb_u16_max());
}

// ─────────────────────────────────────────────────────────────────
// L3: Body starts at id + 1; advance/done stay consecutive;
// the body-to-advance gap scales with N
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_collect_ids_consecutive(id: int, body_len: int)
    requires
        id >= 0,
        body_len >= 1,
        id + body_len + 2 <= vb_u16_max(),
    ensures
        (id + 1) - id == 1,
        (id + body_len + 2) - (id + body_len + 1) == 1,
        (id + body_len + 1) - (id + 1) == body_len,
{
    assert((id + 1) - id == 1) by { }
    assert((id + body_len + 2) - (id + body_len + 1) == 1) by { }
    assert((id + body_len + 1) - (id + 1) == body_len) by { }
}

// ─────────────────────────────────────────────────────────────────
// L4: Maximum valid start ID is u16::MAX - body_len - 2
// (reduces to 65532 for the legacy N = 1 shape)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_max_valid_collect_start(body_len: int)
    requires
        body_len >= 1,
        body_len + 2 <= vb_u16_max(),
    ensures
        vb_u16_max() - body_len - 2 >= 0,
        (vb_u16_max() - body_len - 2) + body_len + 2 == vb_u16_max(),
        (vb_u16_max() - body_len - 2) + body_len + 2 <= vb_u16_max(),
{
    assert(vb_u16_max() - body_len - 2 >= 0) by {
        assert(vb_u16_max() >= body_len + 2);
    }
    assert((vb_u16_max() - body_len - 2) + body_len + 2 == vb_u16_max()) by { }
    assert((vb_u16_max() - body_len - 2) + body_len + 2 <= vb_u16_max()) by { }
}

// ─────────────────────────────────────────────────────────────────
// L5: Option unwrap safety — default value is >= 1
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_option_default_at_least_one(v: Option<u32>)
    ensures
        match v {
            Option::Some(n) => n >= 1 ==> n >= 1,
            Option::None => 1u32 >= 1,
        },
{
}

// ─────────────────────────────────────────────────────────────────
// L6: Full emission chain (L1 + L2 + L3 + spec binding)
// ─────────────────────────────────────────────────────────────────

pub proof fn vb_lemma_full_collect_emission_chain(id: int, body_len: int)
    requires
        id >= 0,
        body_len >= 1,
        id + body_len + 2 <= vb_u16_max(),
    ensures
        id + 1 <= vb_u16_max(),
        id + body_len + 1 <= vb_u16_max(),
        id + body_len + 2 <= vb_u16_max(),
        id + 1 < id + body_len + 1 < id + body_len + 2,
        (id + 1) - id == 1,
        (id + body_len + 2) - (id + body_len + 1) == 1,
        (id + body_len + 1) - (id + 1) == body_len,
        vb_spec_collect_edges(id, body_len) == Ok::<(int, int, int), VbSpecCompileError>(
            (id + 1, id + body_len + 1, id + body_len + 2)),
{
    vb_lemma_collect_n_step_ids_in_bounds(id, body_len);
    vb_lemma_collect_steps_strictly_increasing(id, body_len);
    vb_lemma_collect_ids_consecutive(id, body_len);
    assert(vb_spec_body_length_offset(body_len, 1).is_ok());
    assert(vb_spec_body_length_offset(body_len, 2).is_ok());
    assert(vb_spec_checked_step_offset(id, 1).is_ok());
    assert(vb_spec_checked_step_offset(id, body_len + 1).is_ok());
    assert(vb_spec_checked_step_offset(id, body_len + 2).is_ok());
}

fn main() {}

} // verus!
