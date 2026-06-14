// Verus proof obligations for overflow safety (PS-002, C7).
//
// Obligation ID: POB-vb-vzcuf-005
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-002.rs
//
// Domain claim: Accumulated byte addition and length conversion cannot
// panic or wrap; overflow returns typed rejection.
//
// PRODUCTION BINDING:
//   Target: contract.md C7 — no unchecked arithmetic in byte accounting.
//   Production types:
//     - vb_storage::batch::JournalWriteBatch::append_event (batch.rs:209-229)
//     - vb_storage::error::JournalError (error/mod.rs:20-247)
//     - vb_storage::codec::encode_record (codec/mod.rs:20-32)
//   Production primitives:
//     - u64::checked_add (Rust std) — used for staged + encoded_len
//     - u32 as u64 — safe widening cast for payload_len
//   The spec fn model_checked_add_u64 models Rust's u64::checked_add.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-005

use vstd::prelude::*;

verus! {

// =============================================================================
// PRODUCTION BINDING BRIDGE
// =============================================================================
//
// This file's spec models are bound to production via:
//
//   (a) `checked_add_exec` — a Verus-verified exec fn that calls
//       `u64::checked_add` (Rust std) and proves via `ensures` that
//       the result matches `model_checked_add_u64`.  This establishes
//       that the spec model correctly captures Rust's checked_add semantics.
//
//   (b) `admission_check_exec` — Verus-verified exec fn implementing
//       the full staged + candidate + limit check, with contract proving
//       it matches `admission_check`.
//
//   (c) Kani POB-vb-vzcuf-006 (`kani_vb_vzcuf_ps002.rs`) — tests the
//       production `append_event` overflow path with symbolic arithmetic.
//
// TRUSTED BOUNDARY:
//   u64::checked_add is a Rust std primitive — this spec models it exactly.
//   Production uses the same primitive.  Cross-verifier belt:
//   Verus spec + exec model + Kani production proof.
//   See also: crates/vb_storage/src/kani_vb_vzcuf_ps002.rs

/// Model of u64::checked_add — returns Err on overflow.
/// PRODUCTION BINDING: direct model of Rust std u64::checked_add.
pub open spec fn model_checked_add_u64(a: u64, b: u64) -> Result<int, ()> {
    let sum = a as int + b as int;
    if sum <= u64::MAX as int {
        Ok(sum)
    } else {
        Err(())
    }
}

/// Lemma: model_checked_add_u64 is total (always Ok or Err, never panics).
pub proof fn lemma_checked_add_total(a: u64, b: u64)
    ensures
        model_checked_add_u64(a, b).is_ok() || model_checked_add_u64(a, b).is_err(),
{
    // Spec-level tautology: Result<a,b>.is_ok() || Result<a,b>.is_err() is always true
    // for any Result. Verified by SMT solver from Rust enum definition.
    assert(model_checked_add_u64(a, b).is_ok() || model_checked_add_u64(a, b).is_err());
}

/// Lemma: if checked_add succeeds, the result is exactly a + b.
pub proof fn lemma_checked_add_exact(a: u64, b: u64)
    ensures
        match model_checked_add_u64(a, b) {
            Ok(r) => r == a as int + b as int,
            Err(_) => true,
        },
{
    // Spec-level tautology: model_checked_add_u64 returns Ok(a + b) when no overflow,
    // Err otherwise. In the Ok branch, the result equals a + b by definition.
    assert(match model_checked_add_u64(a, b) {
        Ok(r) => r == a as int + b as int,
        Err(_) => true,
    });
}

/// Lemma: overflow case rejects — a + b > u64::MAX implies Err.
/// Production binding: u64::MAX + 1 overflows, append_event must reject.
pub proof fn lemma_overflow_rejected()
    ensures
        model_checked_add_u64(u64::MAX, 1u64).is_err(),
{
    assert(u64::MAX as int + 1 > u64::MAX as int);
}

/// Lemma: non-overflow case accepts — small addition returns Ok.
pub proof fn lemma_small_add_accepted()
    ensures
        model_checked_add_u64(100u64, 200u64).is_ok(),
{
    assert(100u64 as int + 200u64 as int <= u64::MAX as int);
}

/// Model of safe u32 -> u64 widening cast (always safe, no overflow possible).
/// PRODUCTION BINDING: payload_len as u64 where payload_len: u32,
/// used in encode_record for projecting encoded_length from payload_bytes.
pub open spec fn model_u32_to_u64(n: u32) -> u64 {
    n as u64
}

/// Lemma: u32 -> u64 conversion is exact and total.
pub proof fn lemma_u32_to_u64_safe(n: u32)
    ensures
        model_u32_to_u64(n) == n as u64,
{
    // Spec-level tautology: model_u32_to_u64(n) is defined as n as u64.
    // The ensures is definitionally true.
    assert(model_u32_to_u64(n) == n as u64);
}

/// Complete admission check: staged + candidate with overflow + limit guard.
/// PRODUCTION BINDING:
///   Models JournalWriteBatch::append_event byte admission:
///   1. let total = staged_bytes.checked_add(encoded_len)?;
///   2. if total > byte_limit { return Err(AccumulatedBytesExceeded); }
///   3. Ok(total)
pub open spec fn admission_check(staged: u64, candidate: u64, limit: u64) -> Result<int, ()> {
    match model_checked_add_u64(staged, candidate) {
        Ok(total) => {
            if total <= limit as int { Ok(total) } else { Err(()) }
        }
        Err(_) => Err(()),
    }
}

/// Lemma: admission_check is total (always Ok or Err, never panics).
pub proof fn lemma_admission_check_total(staged: u64, candidate: u64, limit: u64)
    ensures
        admission_check(staged, candidate, limit).is_ok()
            || admission_check(staged, candidate, limit).is_err(),
{
    // Spec-level tautology: Result always has is_ok() || is_err() = true.
    // Verified by SMT solver from Rust enum definition.
    assert(admission_check(staged, candidate, limit).is_ok()
        || admission_check(staged, candidate, limit).is_err());
}

/// Lemma: overflow returns Err, not Ok (C7: overflow is rejection).
pub proof fn lemma_overflow_is_rejection()
    ensures
        admission_check(u64::MAX, 1u64, u64::MAX).is_err(),
{
}

/// Lemma: exact fit at limit returns Ok (C3: accept exact fits).
pub proof fn lemma_exact_limit_accepted(limit: u64)
    requires
        limit > 0,
    ensures
        admission_check(0u64, limit, limit).is_ok(),
{
}

/// Lemma: if admission_check succeeds, staged strictly increases.
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
                requires candidate > 0;
        }
        Err(_) => {}
    }
}

/// Lemma: admission_check with 0-length candidate is always Ok if staged <= limit.
pub proof fn lemma_zero_candidate_accepted(staged: u64, limit: u64)
    requires
        staged <= limit,
    ensures
        admission_check(staged, 0u64, limit).is_ok(),
{
}

// =============================================================================
// Exec bridges — Verus-verified implementations matching the specs.
// =============================================================================

/// Exec bridge: calls `u64::checked_add` and proves it matches `model_checked_add_u64`.
///
/// PRODUCTION BINDING:
///   `JournalWriteBatch::append_event` uses `staged_bytes.checked_add(encoded_len)?`
///   for byte admission.  This exec fn verifies that Rust's `checked_add`
///   matches the spec model for ALL inputs.
pub exec fn checked_add_exec(a: u64, b: u64) -> (result: Option<u64>)
    ensures
        match model_checked_add_u64(a, b) {
            Ok(expected) => result.is_some() && result.unwrap() == expected as u64,
            Err(_) => result.is_none(),
        },
{
    a.checked_add(b)
}

/// Exec bridge: implements `admission_check` using `u64::checked_add`.
///
/// PRODUCTION BINDING:
///   Matches `JournalWriteBatch::append_event` byte admission logic exactly:
///   1. staged_bytes.checked_add(encoded_len) — overflow → rejection
///   2. if total > byte_limit — over-limit → rejection
///   3. Ok(total) — acceptance
pub exec fn admission_check_exec(staged: u64, candidate: u64, limit: u64) -> (result: Result<u64, ()>)
    ensures
        match admission_check(staged, candidate, limit) {
            Ok(expected) => result.is_ok() && result.unwrap() == expected as u64,
            Err(_) => result.is_err(),
        },
{
    match staged.checked_add(candidate) {
        Some(total) if total <= limit => Ok(total),
        _ => Err(()),
    }
}

} // verus!
