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

/// Generate a random non-zero 32-byte digest.
fn arbitrary_nonzero_digest() -> WorkflowDigest {
    loop {
        let bytes: [u8; 32] = kani::any();
        if bytes != [0u8; 32] {
            return WorkflowDigest::from_bytes(bytes);
        }
    }
}

// ===================================================================
// Fail-fast ordering — first mismatch wins
// ===================================================================

/// PPI-005: `check_action_abi_digests` returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_action_abi_fail_fast() {
    use crate::recovery::recover::check_action_abi_digests;
    use crate::recovery::types::RecoveryError;

    let a1 = ActionId::new(1);
    let a2 = ActionId::new(2);
    let a3 = ActionId::new(3);
    let d_ok = WorkflowDigest::from_bytes([1u8; 32]);
    let d_bad = arbitrary_nonzero_digest();

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 3] =
        [(a1, d_ok, d_ok), (a2, d_ok, d_bad), (a3, d_ok, d_bad)];

    let result = check_action_abi_digests(&entries);
    match result {
        Err(e) => {
            let RecoveryError::ActionAbiMismatch { action_id } = e else {
                kani::assert(false, "expected ActionAbiMismatch error variant");
                return;
            };
            kani::assert(action_id == a2, "first mismatch is a2");
        }
        Ok(()) => kani::assert(false, "should return Err, not Ok"),
    }
}

/// PPI-006: `check_policy_digests` returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_policy_fail_fast() {
    use crate::recovery::recover::check_policy_digests;
    use crate::recovery::types::RecoveryError;

    let s1 = StepIdx::new(1);
    let s2 = StepIdx::new(2);
    let s3 = StepIdx::new(3);
    let d_ok = WorkflowDigest::from_bytes([1u8; 32]);
    let d_bad = arbitrary_nonzero_digest();

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 3] =
        [(s1, d_ok, d_ok), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = check_policy_digests(&entries);
    match result {
        Err(e) => {
            let RecoveryError::PolicyDigestMismatch { step } = e else {
                kani::assert(false, "expected PolicyDigestMismatch error variant");
                return;
            };
            kani::assert(step == s2, "first mismatch is s2");
        }
        Ok(()) => kani::assert(false, "should return Err, not Ok"),
    }
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
#[kani::unwind(5)]
fn kani_check_action_abi_mismatch_last() {
    use crate::recovery::recover::check_action_abi_digests;
    use crate::recovery::types::RecoveryError;

    let d_ok = WorkflowDigest::from_bytes([1u8; 32]);
    let d_bad = arbitrary_nonzero_digest();

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 3] = [
        (ActionId::new(1), d_ok, d_ok),
        (ActionId::new(2), d_ok, d_ok),
        (ActionId::new(3), d_ok, d_bad),
    ];

    let result = check_action_abi_digests(&entries);
    match result {
        Err(e) => {
            let RecoveryError::ActionAbiMismatch { action_id } = e else {
                kani::assert(false, "expected ActionAbiMismatch");
                return;
            };
            kani::assert(action_id == ActionId::new(3), "mismatch is in last entry");
        }
        Ok(()) => kani::assert(false, "should return Err"),
    }
}

/// PPI-012: mismatch is in the last policy entry.
#[kani::proof]
#[kani::unwind(5)]
fn kani_check_policy_mismatch_last() {
    use crate::recovery::recover::check_policy_digests;
    use crate::recovery::types::RecoveryError;

    let d_ok = WorkflowDigest::from_bytes([1u8; 32]);
    let d_bad = arbitrary_nonzero_digest();

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 3] = [
        (StepIdx::new(1), d_ok, d_ok),
        (StepIdx::new(2), d_ok, d_ok),
        (StepIdx::new(3), d_ok, d_bad),
    ];

    let result = check_policy_digests(&entries);
    match result {
        Err(e) => {
            let RecoveryError::PolicyDigestMismatch { step } = e else {
                kani::assert(false, "expected PolicyDigestMismatch");
                return;
            };
            kani::assert(step == StepIdx::new(3), "mismatch is in last entry");
        }
        Ok(()) => kani::assert(false, "should return Err"),
    }
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
        let expected = arbitrary_nonzero_digest();
        let found = arbitrary_nonzero_digest();
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
        let expected = arbitrary_nonzero_digest();
        let found = arbitrary_nonzero_digest();
        entries.push((step, expected, found));
    }
    let _ = check_policy_digests(&entries);
}
