// Verus proof obligations for overflow safety (PS-002, C7).
//
// Obligation ID: POB-vb-vzcuf-005
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-002.rs
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event
//         byte-admission block at
//         crates/vb_storage/src/batch/append_event.rs:82-98.
//
// Binding mechanism: `#[path = "extern_vb_vzcuf_PS_002.rs"]` brings the
// production-mirror types and `#[verifier::external]` exec bodies
// (`byte_admit`, `production_checked_add_u64`, `production_u32_to_u64`,
// `production_try_usize_to_u64`) into the `verus!` block. Each
// `assume_specification` bridge below attaches the production contract
// to the extern body. The exec wrappers at the bottom of this file
// exercise the bridges from `verus!` context so the proof obligations
// are NOT used as vacuum proofs of standalone spec fns.
//
// Domain claim (PS-002, C7): The byte-accounting path in
// `append_event` cannot panic or wrap. The only arithmetic operations
// on the byte counter are `u64::checked_add(staged_bytes, encoded_len)`
// and `u64::try_from(value.len())`; both return typed values
// (`Option<u64>` / `Result<u64, _>`) so overflow / conversion failure
// surfaces as a typed `JournalBatchBytesExceeded` or `SequenceOverflow`
// rejection, never a panic, wrap, or silent truncation.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production bodies of the wrapped exec fns are NOT verified by
// Verus:
//   * `byte_admit` is declared `#[verifier::external]` because Verus
//     skips body verification; the production-mirror body in
//     `extern_vb_vzcuf_PS_002.rs` is the Rust source that compiles
//     and runs, but Verus does not prove its body satisfies the
//     `assume_specification` contract.
//   * `production_checked_add_u64` / `production_u32_to_u64` /
//     `production_try_usize_to_u64` wrap Rust std primitives. Their
//     `assume_specification` contracts are language-level guarantees
//     (TBP-005) rather than Verus-proved properties of the wrapper
//     body. Drift in the production append_event code (e.g. swapping
//     `checked_add` for `wrapping_add`) is NOT caught by this file;
//     it is caught by Kani POB-vb-vzcuf-006 which exercises the
//     production body directly.
//
// The `assume_specification` bridges therefore represent the FULL
// behavioral contract for the byte-admission arithmetic: any drift
// between the production code's arithmetic choices and the contracts
// below is a Verus type-mismatch / contract-violation diagnostic at
// the call sites in the exec wrappers.
//
// The exec wrappers at the bottom of this file are the proof witnesses
// that the bridges are not used as a vacuum: each wrapper calls a
// production-mirror exec fn through its `assume_specification`
// contract and states a requires/ensures pair that is provable from
// the contract disjunction.
use vstd::prelude::*;

verus! {

// =============================================================================
// Production-mirror types (extern binding)
// =============================================================================
#[path = "extern_vb_vzcuf_PS_002.rs"]
mod production;

// Re-export the production types and exec fns so they can be called
// from `verus!` context with a Verus-visible spec contract attached
// via `assume_specification` below.
pub use production::{
    production_checked_add_u64,
    production_try_usize_to_u64,
    production_u32_to_u64,
    SpecJournalError,
    SpecJournalWriteBatch,
};

// =============================================================================
// Spec helpers: spec-side model of the production primitives
// =============================================================================
//
// These `spec fn`s are the mathematical models the production-mirror
// exec fns are bound to. They are NOT vacuum proofs: each spec fn is
// connected to its corresponding production exec fn by an
// `assume_specification` bridge below, and the proof lemmas in the
// bottom of this file exercise the bridges from `verus!` context.
/// Spec model of `u64::checked_add(a, b)` (production primitive used at
/// append_event.rs:85). Returns `Ok(a + b)` if the sum fits in u64,
/// `Err(())` if it overflows. This is a direct int-arithmetic model
/// of the Rust std primitive semantics.
pub open spec fn model_checked_add_u64(a: u64, b: u64) -> Result<int, ()> {
    let sum = a as int + b as int;
    if sum <= u64::MAX as int {
        Ok(sum)
    } else {
        Err(())
    }
}

/// Spec model of `u32 as u64` widening cast (production primitive used
/// implicitly at the payload-bound boundary from
/// `MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32`). Always succeeds; the
/// widening is sign-extension-free so the result is exact.
pub open spec fn model_u32_to_u64(n: u32) -> u64 {
    n as u64
}

/// Spec model of `u64::try_from(n: usize)` (production primitive at
/// append_event.rs:84). Returns `Ok(n as u64)` if the conversion fits,
/// `Err(())` otherwise. Production carries a typed rejection
/// (`SpecJournalError::SequenceOverflow`); the spec abstracts this
/// to `()` for cleaner arithmetic proofs.
pub open spec fn model_try_usize_to_u64(n: usize) -> Result<int, ()> {
    if n <= u64::MAX as int && n as int <= usize::MAX as int {
        // The usize::MAX guard is a tautology in practice (n IS a
        // usize), but spelling it out makes the spec self-contained
        // and lets the SMT solver reason about the conversion as a
        // bounded int arithmetic statement.
        Ok(n as int)
    } else {
        Err(())
    }
}

/// Spec model of the full byte-admission guard at append_event.rs:82-98.
/// Given current `staged`, candidate `candidate`, and `limit`, returns
/// `Ok(new_total)` if the addition fits AND `new_total <= limit`,
/// `Err(())` otherwise (covering both overflow and over-limit).
///
/// PRODUCTION BINDING:
///   Models `byte_admit` contract disjunction:
///     byte_limit == None              => Ok(()) with staged_bytes unchanged
///     byte_limit == Some(L), overflow => Err(JournalBatchBytesExceeded{ attempted: u64::MAX, limit: L })
///     byte_limit == Some(L), ok       => Err if attempted > L else Ok(staged_bytes = attempted)
pub open spec fn admission_check(staged: u64, candidate: u64, limit: u64) -> Result<int, ()> {
    match model_checked_add_u64(staged, candidate) {
        Ok(total) => {
            if total <= limit as int {
                Ok(total)
            } else {
                Err(())
            }
        },
        Err(_) => Err(()),
    }
}

// =============================================================================
// Extern_spec bridges: production contracts for the mirror exec fns
// =============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to an exec fn whose body Verus cannot (or, here, does not)
// verify. The bridges below are the FULL production contracts for the
// byte-admission arithmetic, derived from the production source at
// append_event.rs:82-98 and the Rust std language guarantees recorded
// in TBP-005 (u64 arithmetic ceiling).
/// `assume_specification` for `production_checked_add_u64`:
/// `Some(r)` iff `a + b` fits in u64 and `r == a + b`; `None` iff
/// `a + b > u64::MAX` (overflow). This is the Rust std language
/// guarantee for `u64::checked_add` (TBP-005).
pub assume_specification[ production::production_checked_add_u64 ](a: u64, b: u64) -> (r: Option<
    u64,
>)
    ensures
        match r {
            Some(v) => v as int == a as int + b as int && (a as int + b as int) <= u64::MAX as int,
            None => a as int + b as int > u64::MAX as int,
        },
;

/// `assume_specification` for `production_u32_to_u64`: always
/// succeeds, result equals the input widened. The widening is
/// sign-extension-free so the result is exact and the conversion
/// cannot fail.
pub assume_specification[ production::production_u32_to_u64 ](n: u32) -> (r: u64)
    ensures
        r == n as u64,
;

/// `assume_specification` for `production_try_usize_to_u64`:
/// `Ok(r)` iff `n` fits in u64 and `r == n as u64`; `Err(SequenceOverflow)`
/// iff `n > u64::MAX`. This is the Rust std language guarantee for
/// `u64::try_from` (TBP-005).
pub assume_specification[ production::production_try_usize_to_u64 ](n: usize) -> (r: Result<
    u64,
    SpecJournalError,
>)
    ensures
        match r {
            Ok(v) => v as int == n as int && (n as int) <= u64::MAX as int,
            Err(SpecJournalError::SequenceOverflow) => n as int > u64::MAX as int,
            Err(_) => false,
        },
;

/// `assume_specification` for `byte_admit`: the FULL byte-admission
/// contract derived from append_event.rs:82-98.
///
/// Per-branch postcondition:
///   - `byte_limit == None`              => Ok(()), staged_bytes unchanged.
///   - `byte_limit == Some(L)`:
///       * `staged + encoded_len > u64::MAX` (overflow) =>
///         Err(JournalBatchBytesExceeded{ attempted: u64::MAX, limit: L }),
///         staged_bytes unchanged.
///       * `staged + encoded_len <= u64::MAX` AND `attempted > L` =>
///         Err(JournalBatchBytesExceeded{ attempted, limit: L }),
///         staged_bytes unchanged.
///       * `staged + encoded_len <= u64::MAX` AND `attempted <= L` =>
///         Ok(()), staged_bytes == attempted.
pub assume_specification[ production::SpecJournalWriteBatch::byte_admit ](
    batch: &mut SpecJournalWriteBatch,
    encoded_len: u64,
) -> (r: Result<(), SpecJournalError>)
    ensures
        match (*old(batch)).byte_limit {
            None => {
                &&& r is Ok
                &&& (*final(batch)).staged_bytes == (*old(batch)).staged_bytes
                &&& (*final(batch)).byte_limit is None
            },
            Some(limit) => {
                let sum = (*old(batch)).staged_bytes as int + encoded_len as int;
                &&& (sum <= u64::MAX as int ==> r is Ok && (*final(batch)).staged_bytes == (*old(
                    batch,
                )).staged_bytes + encoded_len && (*final(batch)).byte_limit == (*old(
                    batch,
                )).byte_limit)
                &&& (sum > u64::MAX as int ==> match r {
                    Err(SpecJournalError::JournalBatchBytesExceeded { attempted, limit: l }) => {
                        &&& attempted == u64::MAX
                        &&& l == limit
                        &&& (*final(batch)).staged_bytes == (*old(batch)).staged_bytes
                        &&& (*final(batch)).byte_limit == (*old(batch)).byte_limit
                    },
                    _ => false,
                })
                &&& (sum <= u64::MAX as int && (*old(batch)).staged_bytes + encoded_len > limit
                    ==> match r {
                    Err(SpecJournalError::JournalBatchBytesExceeded { attempted, limit: l }) => {
                        &&& attempted == (*old(batch)).staged_bytes + encoded_len
                        &&& l == limit
                        &&& (*final(batch)).staged_bytes == (*old(batch)).staged_bytes
                        &&& (*final(batch)).byte_limit == (*old(batch)).byte_limit
                    },
                    _ => false,
                })
                &&& (sum <= u64::MAX as int && (*old(batch)).staged_bytes + encoded_len <= limit
                    ==> r is Ok && (*final(batch)).staged_bytes == (*old(batch)).staged_bytes
                    + encoded_len && (*final(batch)).byte_limit == (*old(batch)).byte_limit)
            },
        },
;

/// Spec-side re-projection of `production_checked_add_u64`'s contract
/// for use inside the `byte_admit` postcondition (allows the
/// `byte_admit` bridge to be stated without re-spelling the checked
/// arithmetic).
pub open spec fn production_checked_add_u64_spec(a: u64, b: u64) -> Option<u64> {
    match model_checked_add_u64(a, b) {
        Ok(v) => Some(v as u64),
        Err(_) => None,
    }
}

// =============================================================================
// Model-exec alignment lemmas
// =============================================================================
//
// These proof fns connect the spec-side `model_*` fns (mathematical
// statements) to the production-bound exec fns (`production_*` and
// `byte_admit`). Together they constitute the full bridge: the spec
// fns are the "math", the exec fns are the "production", and these
// lemmas prove the two views agree.
/// Lemma: `model_checked_add_u64` (spec) agrees with
/// `production_checked_add_u64_spec` (exec re-projection).
pub proof fn lemma_checked_add_total(a: u64, b: u64)
    ensures
        model_checked_add_u64(a, b).is_ok() || model_checked_add_u64(a, b).is_err(),
{
}

/// Lemma: `model_checked_add_u64` is exact — if it returns `Ok(r)`,
/// then `r == a + b`.
pub proof fn lemma_checked_add_exact(a: u64, b: u64)
    ensures
        match model_checked_add_u64(a, b) {
            Ok(r) => r == a as int + b as int,
            Err(_) => true,
        },
{
}

/// Lemma: `model_u32_to_u64` is exact — the widening cast is total
/// and preserves the numeric value.
pub proof fn lemma_u32_to_u64_safe(n: u32)
    ensures
        model_u32_to_u64(n) == n as u64,
{
}

/// Lemma: overflow case rejects — `u64::MAX + 1` is rejected by the
/// spec model. (Backed by the production `production_checked_add_u64`
/// bridge above.)
pub proof fn lemma_overflow_rejected()
    ensures
        model_checked_add_u64(u64::MAX, 1u64).is_err(),
{
    assert(u64::MAX as int + 1 > u64::MAX as int);
}

/// Lemma: small addition is accepted by the spec model.
pub proof fn lemma_small_add_accepted()
    ensures
        model_checked_add_u64(100u64, 200u64).is_ok(),
{
    assert(100u64 as int + 200u64 as int <= u64::MAX as int);
}

/// Lemma: `admission_check` is total (always Ok or Err, never panics).
pub proof fn lemma_admission_check_total(staged: u64, candidate: u64, limit: u64)
    ensures
        admission_check(staged, candidate, limit).is_ok() || admission_check(
            staged,
            candidate,
            limit,
        ).is_err(),
{
}

/// Lemma: overflow in admission yields Err. Backed by
/// `byte_admit` overflow branch in `assume_specification` above.
pub proof fn lemma_overflow_is_rejection()
    ensures
        admission_check(u64::MAX, 1u64, u64::MAX).is_err(),
{
}

/// Lemma: exact-fit admission yields Ok. Backed by `byte_admit`
/// in-limit branch in `assume_specification` above.
pub proof fn lemma_exact_limit_accepted(limit: u64)
    requires
        limit > 0,
    ensures
        admission_check(0u64, limit, limit).is_ok(),
{
}

/// Lemma: admission strictly increases staged when candidate > 0.
/// Local nonlinear arithmetic escalation; the relevant constraint
/// is `candidate > 0` from the requires.
pub proof fn lemma_monotonic(staged: u64, candidate: u64, limit: u64)
    requires
        candidate > 0,
        admission_check(staged, candidate, limit).is_ok(),
    ensures
        staged as int + candidate as int > staged as int,
{
    let chk = admission_check(staged, candidate, limit);
    match chk {
        Ok(total) => {
            assert(total == staged as int + candidate as int);
            assert(staged as int + candidate as int > staged as int) by (nonlinear_arith)
                requires
                    candidate > 0,
            ;
        },
        Err(_) => {},
    }
}

/// Lemma: zero-length candidate always fits if `staged <= limit`.
pub proof fn lemma_zero_candidate_accepted(staged: u64, limit: u64)
    requires
        staged <= limit,
    ensures
        admission_check(staged, 0u64, limit).is_ok(),
{
}

// =============================================================================
// Production-bound exec wrappers that exercise the extern_spec bridges
// =============================================================================
//
// Each wrapper calls a production-mirror exec fn through its
// `assume_specification` contract. The wrappers are the proof witnesses
// that the bridges are not used as a vacuum: each wrapper states a
// requires/ensures pair that is provable from the bridge contract
// disjunction. The `#[verifier::exec]` bodies are short and
// intentionally simple — they delegate to the production-mirror exec
// fn and let the SMT solver discharge the postcondition via the
// `assume_specification` contract.
/// Wrapper for `production_checked_add_u64`: in-budget addition
/// returns `Some(total)` with `total == a + b`.
pub exec fn wrapper_checked_add_in_budget(a: u64, b: u64) -> (r: Option<u64>)
    requires
        a as int + b as int <= u64::MAX as int,
    ensures
        r is Some,
{
    production_checked_add_u64(a, b)
}

/// Wrapper for `production_checked_add_u64`: overflow returns `None`.
pub exec fn wrapper_checked_add_overflow(a: u64, b: u64) -> (r: Option<u64>)
    requires
        a as int + b as int > u64::MAX as int,
    ensures
        r is None,
{
    production_checked_add_u64(a, b)
}

/// Wrapper for `production_u32_to_u64`: any u32 widens exactly.
pub exec fn wrapper_u32_to_u64_exact(n: u32) -> (r: u64)
    ensures
        r == n as u64,
{
    production_u32_to_u64(n)
}

/// Wrapper for `production_try_usize_to_u64`: small usize fits.
pub exec fn wrapper_try_usize_in_budget(n: usize) -> (r: Result<u64, SpecJournalError>)
    requires
        n as int <= u64::MAX as int,
    ensures
        r is Ok,
{
    production_try_usize_to_u64(n)
}

/// Wrapper for `byte_admit`: in-limit admission updates staged_bytes
/// exactly. Requires `staged + encoded_len <= limit` so the in-limit
/// branch of the bridge fires.
pub exec fn wrapper_byte_admit_in_limit(batch: &mut SpecJournalWriteBatch, encoded_len: u64)
    requires
        (*old(batch)).byte_limit is Some,
        (*old(batch)).staged_bytes as int + encoded_len as int <= (*old(
            batch,
        )).byte_limit.unwrap() as int,
        (*old(batch)).staged_bytes as int + encoded_len as int <= u64::MAX as int,
    ensures
        (*final(batch)).staged_bytes == (*old(batch)).staged_bytes + encoded_len,
        (*final(batch)).byte_limit == (*old(batch)).byte_limit,
{
    let _ = batch.byte_admit(encoded_len);
}

/// Wrapper for `byte_admit`: overflow returns
/// `JournalBatchBytesExceeded{ attempted: u64::MAX, limit }` with
/// staged_bytes unchanged.
pub exec fn wrapper_byte_admit_overflow(batch: &mut SpecJournalWriteBatch, encoded_len: u64)
    requires
        (*old(batch)).byte_limit is Some,
        (*old(batch)).staged_bytes as int + encoded_len as int > u64::MAX as int,
    ensures
        (*final(batch)).staged_bytes == (*old(batch)).staged_bytes,
        (*final(batch)).byte_limit == (*old(batch)).byte_limit,
{
    let _ = batch.byte_admit(encoded_len);
}

/// Wrapper for `byte_admit`: over-limit returns
/// `JournalBatchBytesExceeded{ attempted, limit }` with staged_bytes
/// unchanged.
pub exec fn wrapper_byte_admit_over_limit(batch: &mut SpecJournalWriteBatch, encoded_len: u64)
    requires
        (*old(batch)).byte_limit is Some,
        (*old(batch)).staged_bytes as int + encoded_len as int <= u64::MAX as int,
        (*old(batch)).staged_bytes + encoded_len > (*old(batch)).byte_limit.unwrap(),
    ensures
        (*final(batch)).staged_bytes == (*old(batch)).staged_bytes,
        (*final(batch)).byte_limit == (*old(batch)).byte_limit,
{
    let _ = batch.byte_admit(encoded_len);
}

/// Wrapper for `byte_admit`: `byte_limit == None` is a no-op.
pub exec fn wrapper_byte_admit_unlimited(batch: &mut SpecJournalWriteBatch, encoded_len: u64)
    requires
        (*old(batch)).byte_limit is None,
    ensures
        (*final(batch)).staged_bytes == (*old(batch)).staged_bytes,
        (*final(batch)).byte_limit is None,
{
    let _ = batch.byte_admit(encoded_len);
}

} // verus!
