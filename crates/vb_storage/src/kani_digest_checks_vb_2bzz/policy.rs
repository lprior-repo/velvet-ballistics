#![forbid(unsafe_code)]

use super::support::{arbitrary_digest, arbitrary_digest_except};
use vb_core::{StepIdx, WorkflowDigest};

/// PPI-006: policy mismatch selection returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_fail_fast() {
    use crate::recovery::digest::first_policy_mismatch;

    let s1 = StepIdx::new(kani::any());
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(s1, d_ok, d_ok), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            #![forbid(unsafe_code)]

use super::support::{arbitrary_digest, arbitrary_digest_except};
use vb_core::{StepIdx, WorkflowDigest};

/// PPI-006: policy mismatch selection returns the *first* mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_fail_fast() {
    use crate::recovery::digest::first_policy_mismatch;

    let s1 = StepIdx::new(kani::any());
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(s1, d_ok, d_ok), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            kani::assert(step == s2, "first mismatch is s2");
            kani::assert(expected == d_ok, "expected digest is first mismatch");
            kani::assert(found == d_bad, "found digest is first mismatch");
        }
        None => kani::assert(false, "should return first policy mismatch"),
    }
}

/// PPI-006b: first-entry policy mismatch wins even when later entries also mismatch.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_first_entry_mismatch() {
    use crate::recovery::digest::first_policy_mismatch;

    let s1 = StepIdx::new(kani::any());
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(s1, d_ok, d_bad), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            );
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let entries = [(s1, d_ok, d_bad), (s2, d_ok, d_bad), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            kani::assert(step == s1, "first-entry mismatch step is returned");
            kani::assert(expected == d_ok, "first-entry expected digest is returned");
            kani::assert(found == d_bad, "first-entry found digest is returned");
        }
        None => kani::assert(false, "first-entry policy mismatch should be returned"),
    }
}

/// PPI-008: empty policy entry list has no mismatch.
#[kani::proof]
#[kani::unwind(8)]
fn kani_check_policy_empty() {
    use crate::recovery::digest::first_policy_mismatch;

    let entries: [(StepIdx, WorkflowDigest, WorkflowDigest); 0] = [];
    let result = first_policy_mismatch(&entries);
    kani::assert(
        result.is_none(),
        "empty policy list should have no mismatch",
    );
}

/// PPI-009b: single-entry policy mismatch returns that entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_single_entry_mismatch() {
    use crate::recovery::digest::first_policy_mismatch;

    let step = StepIdx::new(kani::any());
    let expected = arbitrary_digest();
    let found = arbitrary_digest_except(expected);
    let entries = [(step, expected, found)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((found_step, found_expected, found_digest)) => {
            ,
        "empty policy list should have no mismatch",
    );
}

/// PPI-009b: single-entry policy mismatch returns that entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_single_entry_mismatch() {
    use crate::recovery::digest::first_policy_mismatch;

    let step = StepIdx::new(kani::any());
    let expected = arbitrary_digest();
    let found = arbitrary_digest_except(expected);
    let entries = [(step, expected, found)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((found_step, found_expected, found_digest)) => {
            kani::assert(found_step == step, "single mismatch step is returned");
            kani::assert(
                found_expected == expected,
                "single mismatch expected digest",
            );
            kani::assert(found_digest == found, "single mismatch found digest");
        }
        None => kani::assert(false, "single policy mismatch should be returned"),
    }
}

/// PPI-010: all policy entries match, so no mismatch is selected.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_all_match() {
    use crate::recovery::digest::first_policy_mismatch;

    let step = StepIdx::new(kani::any());
    let d = arbitrary_digest();
    let entries = [(step, d, d)];
    let result = first_policy_mismatch(&entries);
    kani::assert(result.is_none(),
        "all-matching policy list should have no mismatch",
    );
}

/// PPI-012: mismatch is in the last policy entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_mismatch_last() {
    use crate::recovery::digest::first_policy_mismatch;

    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let s1 = StepIdx::new(kani::any());
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let entries = [(s1, d_ok, d_ok), (s2, d_ok, d_ok), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            ,
        "all-matching policy list should have no mismatch",
    );
}

/// PPI-012: mismatch is in the last policy entry.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_mismatch_last() {
    use crate::recovery::digest::first_policy_mismatch;

    let d_ok = arbitrary_digest();
    let d_bad = arbitrary_digest_except(d_ok);
    let s1 = StepIdx::new(kani::any());
    let s2 = StepIdx::new(kani::any());
    let s3 = StepIdx::new(kani::any());
    let entries = [(s1, d_ok, d_ok), (s2, d_ok, d_ok), (s3, d_ok, d_bad)];

    let result = first_policy_mismatch(&entries);
    match result {
        Some((step, expected, found)) => {
            kani::assert(step == s3, "mismatch step is from last entry");
            kani::assert(expected == d_ok, "expected digest is last mismatch");
            kani::assert(found == d_bad, "found digest is last mismatch");
        }
        None => kani::assert(false, "should return last policy mismatch"),
    }
}

/// PPI-014: policy mismatch selection never panics for bounded arbitrary inputs.
#[kani::proof]
#[kani::unwind(40)]
fn kani_check_policy_no_panic() {
    use crate::recovery::digest::first_policy_mismatch;

    let e1 = (
        StepIdx::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let e2 = (
        StepIdx::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let e3 = (
        StepIdx::new(kani::any()),
        arbitrary_digest(),
        arbitrary_digest(),
    );
    let empty: [(StepIdx, WorkflowDigest, WorkflowDigest); 0] = [];
    let one = [e1];
    let two = [e1, e2];
    let three = [e1, e2, e3];

    let _ = first_policy_mismatch(&empty);
    let _ = first_policy_mismatch(&one);
    let _ = first_policy_mismatch(&two);
    let _ = first_policy_mismatch(&three);
}
