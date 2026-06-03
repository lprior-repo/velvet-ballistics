#![forbid(unsafe_code)]
//! Kani proof harnesses for digest-check functions exposed by vb-2bzz.
//!
//! Targets: `check_action_abi_digests` and `check_policy_digests` from
//! `vb_storage::recovery::recover`.  Every harness uses `kani::any()` to
//! generate digest inputs — no hardcoded shapes — so proofs cover the full
//! digest space.
//!
//! Obligations: PPI-005 … PPI-014

use vb_core::{ActionId, StepIdx, WorkflowDigest};

/// Generate an arbitrary digest.
fn arbitrary_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes(kani::any())
}

/// Generate an arbitrary digest that is guaranteed to differ from `excluded`.
fn arbitrary_digest_except(excluded: WorkflowDigest) -> WorkflowDigest {
    let bytes: [u8; 32] = kani::any();
    kani::assume(bytes != excluded.as_bytes());
    WorkflowDigest::from_bytes(bytes)
}

// ===================================================================
// Fail-fast ordering — first mismatch wins
// ===================================================================

/// PPI-005: `check_action_abi_digests` returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_fail_fast() {
    use crate::recovery::recover::check_action_abi_digests;
    use crate::recovery::types::RecoveryError;

    let a1 = ActionId::new(1);
    let a2 = ActionId::new(2);
    let a3 = ActionId::new(3);
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 3] =
        [(a1, d_ok, d_ok), (a2, d_ok, d_bad), (a3, d_ok, d_bad)];

    let result = check_action_abi_digests(&entries);
    match &result {
        Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        }) => {
            kani::assert(*action_id == a2, "first mismatch is a2");
            kani::assert(
                *expected == d_ok,
                "expected digest is first mismatching expected",
            );
            kani::assert(*found == d_bad, "found digest is first mismatching found");
            kani::cover!(true, "action_abi_fail_fast_mismatch_branch_reached");
        }
        Err(_) => kani::assert(false, "expected ActionAbiMismatch error variant"),
        Ok(()) => kani::assert(false, "should return Err, not Ok"),
    }
    std::mem::forget(result);
}

/// PPI-006: `check_policy_digests` returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_fail_fast() {
    use crate::recovery::recover::check_policy_digests;
    use crate::recovery::types::RecoveryError;

    let s1 = StepIdx::new(1);
    let s2 = StepIdx::new(2);
    let s3 = StepIdx::new(3);
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 3] =
        [(s1, d_ok, d_ok), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = check_policy_digests(&entries);
    match &result {
        Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        }) => {
            kani::assert(*step == s2, "first mismatch is s2");
            kani::assert(
                *expected == d_ok,
                "expected digest is first mismatching expected",
            );
            kani::assert(*found == d_bad, "found digest is first mismatching found");
            kani::cover!(true, "policy_fail_fast_mismatch_branch_reached");
        }
        Err(_) => kani::assert(false, "expected PolicyDigestMismatch error variant"),
        Ok(()) => kani::assert(false, "should return Err, not Ok"),
    }
    std::mem::forget(result);
}

// ===================================================================
// Empty-input edge: no panic, returns Ok
// ===================================================================

/// PPI-007: empty ABI entry list returns Ok (no panic).
#[kani::proof]
#[kani::unwind(3)]
fn kani_check_action_abi_empty() {
    use crate::recovery::recover::check_action_abi_digests;

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 0] = [];
    let result = check_action_abi_digests(&entries);
    kani::assert(result.is_ok(), "empty ABI list should return Ok");
}

/// PPI-008: empty policy entry list returns Ok (no panic).
#[kani::proof]
#[kani::unwind(3)]
fn kani_check_policy_empty() {
    use crate::recovery::recover::check_policy_digests;

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 0] = [];
    let result = check_policy_digests(&entries);
    kani::assert(result.is_ok(), "empty policy list should return Ok");
}

// ===================================================================
// All-match: no panic, returns Ok
// ===================================================================

/// PPI-009: all ABI entries match → Ok.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_action_abi_all_match() {
    use crate::recovery::recover::check_action_abi_digests;

    let d = WorkflowDigest::from_bytes([0xAB; 32]);
    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 4] = [
        (ActionId::new(1), d, d),
        (ActionId::new(2), d, d),
        (ActionId::new(3), d, d),
        (ActionId::new(4), d, d),
    ];

    let result = check_action_abi_digests(&entries);
    kani::assert(result.is_ok(), "all-matching ABI list should return Ok");
}

/// PPI-010: all policy entries match → Ok.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_policy_all_match() {
    use crate::recovery::recover::check_policy_digests;

    let d = WorkflowDigest::from_bytes([0xCD; 32]);
    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 4] = [
        (StepIdx::new(1), d, d),
        (StepIdx::new(2), d, d),
        (StepIdx::new(3), d, d),
        (StepIdx::new(4), d, d),
    ];

    let result = check_policy_digests(&entries);
    kani::assert(result.is_ok(), "all-matching policy list should return Ok");
}

// ===================================================================
// Mismatch position coverage — last entry is the one that fails
// ===================================================================

/// PPI-011: mismatch is in the last entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_mismatch_last() {
    use crate::recovery::recover::check_action_abi_digests;
    use crate::recovery::types::RecoveryError;

    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 3] = [
        (ActionId::new(1), d_ok, d_ok),
        (ActionId::new(2), d_ok, d_ok),
        (ActionId::new(3), d_ok, d_bad),
    ];

    let result = check_action_abi_digests(&entries);
    match &result {
        Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        }) => {
            kani::assert(*action_id == ActionId::new(3), "mismatch is in last entry");
            kani::assert(
                *expected == d_ok,
                "expected digest is last mismatching expected",
            );
            kani::assert(*found == d_bad, "found digest is last mismatching found");
            kani::cover!(true, "action_abi_last_mismatch_branch_reached");
        }
        Err(_) => kani::assert(false, "expected ActionAbiMismatch"),
        Ok(()) => kani::assert(false, "should return Err"),
    }
    std::mem::forget(result);
}

/// PPI-012: mismatch is in the last policy entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_mismatch_last() {
    use crate::recovery::recover::check_policy_digests;
    use crate::recovery::types::RecoveryError;

    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 3] = [
        (StepIdx::new(1), d_ok, d_ok),
        (StepIdx::new(2), d_ok, d_ok),
        (StepIdx::new(3), d_ok, d_bad),
    ];

    let result = check_policy_digests(&entries);
    match &result {
        Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        }) => {
            kani::assert(*step == StepIdx::new(3), "mismatch is in last entry");
            kani::assert(
                *expected == d_ok,
                "expected digest is last mismatching expected",
            );
            kani::assert(*found == d_bad, "found digest is last mismatching found");
            kani::cover!(true, "policy_last_mismatch_branch_reached");
        }
        Err(_) => kani::assert(false, "expected PolicyDigestMismatch"),
        Ok(()) => kani::assert(false, "should return Err"),
    }
    std::mem::forget(result);
}

// ===================================================================
// Panic-freedom: never unwrap / expect / panic on valid inputs
// ===================================================================

/// PPI-013: `check_action_abi_digests` never panics for arbitrary inputs.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_action_abi_no_panic() {
    use crate::recovery::recover::check_action_abi_digests;

    let count: usize = kani::any();
    let count = count % 20;
    let mut entries = Vec::new();
    for _ in 0..count {
        let action = ActionId::new(kani::any());
        let expected = arbitrary_digest();
        let found = arbitrary_digest();
        entries.push((action, expected, found));
    }
    let _ = check_action_abi_digests(&entries);
}

/// PPI-014: `check_policy_digests` never panics for arbitrary inputs.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_policy_no_panic() {
    use crate::recovery::recover::check_policy_digests;

    let count: usize = kani::any();
    let count = count % 20;
    let mut entries = Vec::new();
    for _ in 0..count {
        let step = StepIdx::new(kani::any());
        let expected = arbitrary_digest();
        let found = arbitrary_digest();
        entries.push((step, expected, found));
    }
    let _ = check_policy_digests(&entries);
}
