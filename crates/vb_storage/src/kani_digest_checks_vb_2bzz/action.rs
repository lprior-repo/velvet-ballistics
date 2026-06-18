#![forbid(unsafe_code)]

use super::support::{arbitrary_digest, arbitrary_digest_except};
use vb_core::{ActionId, WorkflowDigest};

/// PPI-005: action ABI mismatch selection returns the first mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_fail_fast() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let a1 = ActionId::new(kani::any());
    let a2 = ActionId::new(kani::any());
    let a3 = ActionId::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(a1, d_ok, d_ok), (a2, d_ok, d_bad), (a3, d_ok, d_bad)];

    match first_action_abi_mismatch(&entries) {
        Some((action_id, expected, found)) => {
            kani::assert(action_id == a2, "first mismatch is a2");
            kani::assert(expected == d_ok, "expected digest is first mismatch");
            kani::assert(found == d_bad, "found digest is first mismatch");
        }
        None => kani::assert(false, "should return first action ABI mismatch"),
    }
}

/// PPI-006a: first-entry ABI mismatch wins even when later entries also mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_first_entry_mismatch() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let a1 = ActionId::new(kani::any());
    let a2 = ActionId::new(kani::any());
    let a3 = ActionId::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(a1, d_ok, d_bad), (a2, d_ok, d_bad), (a3, d_ok, d_bad)];

    match first_action_abi_mismatch(&entries) {
        Some((action_id, expected, found)) => {
            kani::assert(action_id == a1, "first-entry mismatch action is returned");
            kani::assert(expected == d_ok, "first-entry expected digest is returned");
            kani::assert(found == d_bad, "first-entry found digest is returned");
        }
        None => kani::assert(false, "first-entry action ABI mismatch should be returned"),
    }
}

/// PPI-007: empty ABI entry list has no mismatch.
#[kani::proof]
#[kani::unwind(8)]
fn kani_check_action_abi_empty() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let entries: [(ActionId, WorkflowDigest, WorkflowDigest); 0] = [];
    let result = first_action_abi_mismatch(&entries);
    kani::assert(result.is_none(), "empty ABI list should have no mismatch");
}

/// PPI-009a: single-entry ABI mismatch returns that entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_single_entry_mismatch() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let action = ActionId::new(kani::any());
    let expected = arbitrary_digest();
    let found = arbitrary_digest_except(expected);
    let entries = [(action, expected, found)];

    match first_action_abi_mismatch(&entries) {
        Some((found_action, found_expected, found_digest)) => {
            kani::assert(found_action == action, "single mismatch action is returned");
            kani::assert(found_expected == expected, "single mismatch expected digest");
            kani::assert(found_digest == found, "single mismatch found digest");
        }
        None => kani::assert(false, "single action ABI mismatch should be returned"),
    }
}

/// PPI-009: all ABI entries match, so no mismatch is selected.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_all_match() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let action = ActionId::new(kani::any());
    let digest = arbitrary_digest();
    let entries = [(action, digest, digest)];
    let result = first_action_abi_mismatch(&entries);
    kani::assert(result.is_none(), "all-matching ABI list should have no mismatch");
}

/// PPI-011: mismatch is in the last entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_mismatch_last() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let a1 = ActionId::new(kani::any());
    let a2 = ActionId::new(kani::any());
    let a3 = ActionId::new(kani::any());
    let entries = [(a1, d_ok, d_ok), (a2, d_ok, d_ok), (a3, d_ok, d_bad)];

    match first_action_abi_mismatch(&entries) {
        Some((action_id, expected, found)) => {
            kani::assert(action_id == a3, "mismatch action is from last entry");
            kani::assert(expected == d_ok, "expected digest is last mismatch");
            kani::assert(found == d_bad, "found digest is last mismatch");
        }
        None => kani::assert(false, "should return last action ABI mismatch"),
    }
}

/// PPI-013: action ABI mismatch selection never panics for bounded arbitrary inputs.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_action_abi_no_panic() {
    use crate::recovery::digest::first_action_abi_mismatch;

    let e1 = (
        ActionId::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let e2 = (
        ActionId::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let e3 = (
        ActionId::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let empty: [(ActionId, WorkflowDigest, WorkflowDigest); 0] = [];
    let one = [e1];
    let two = [e1, e2];
    let three = [e1, e2, e3];

    let _empty_result = first_action_abi_mismatch(&empty);
    let _one_result = first_action_abi_mismatch(&one);
    let _two_result = first_action_abi_mismatch(&two);
    let _three_result = first_action_abi_mismatch(&three);
}
