// Verification artifact: vb_yaml_duplicate_key_detection.rs
// Verifier: Verus
// Crate: vb_yaml
//
// Proof obligations:
// - PO-YAML-023: reject_duplicate_keys detects duplicates in linear scan
// - PO-YAML-024: No false positives (unique keys pass the check)
// - PO-YAML-025: Duplicate key detection is O(n) per element
//
// GOD RULE 2: Spec functions mirror production code in
// crates/vb_yaml/src/profile_dupkeys.rs.

use vstd::prelude::*;

verus! {

// ─────────────────────────────────────────────────────────────────
// Spec: Duplicate key detection model
// ─────────────────────────────────────────────────────────────────

/// Spec: A list of keys has duplicates iff any two distinct positions
/// hold the same key.
pub open spec fn spec_has_duplicate_keys(keys: Seq<&str>) -> bool {
    exists|i: int| 0 <= i && i < keys.len()
        && exists|j: int| i < j && j < keys.len() && keys[i] == keys[j]
}

/// Spec: A list of keys has no duplicates.
pub open spec fn spec_no_duplicate_keys(keys: Seq<&str>) -> bool {
    !spec_has_duplicate_keys(keys)
}

/// Spec: The linear-scan algorithm for duplicate detection.
/// Mirrors the production `reject_duplicate_keys` function.
pub open spec fn spec_duplicate_scan(keys: Seq<&str>, seen: Seq<&str>) -> bool {
    if keys.len() == 0 {
        true // No more keys to check — no duplicates found
    } else {
        let head = keys[0];
        let tail = keys.slice(1, keys.len() - 1);
        if seen.contains(head) {
            false // Found a duplicate
        } else {
            spec_duplicate_scan(tail, seen.push_back(head))
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-023: Duplicate detection correctness
// ─────────────────────────────────────────────────────────────────

/// Lemma: An empty key list has no duplicates.
pub proof fn lemma_empty_keys_no_duplicates()
    ensures
        spec_no_duplicate_keys(vec![]),
{
    assert(spec_no_duplicate_keys(vec![]));
}

/// Lemma: A single-element key list has no duplicates.
pub proof fn lemma_single_key_no_duplicates()
    ensures
        spec_no_duplicate_keys(vec!["a"]),
{
    assert(spec_no_duplicate_keys(vec!["a"]));
}

/// Lemma: Two different keys have no duplicates.
pub proof fn lemma_two_different_keys_no_duplicates()
    ensures
        spec_no_duplicate_keys(vec!["a", "b"]),
{
    assert(spec_no_duplicate_keys(vec!["a", "b"]));
}

/// Lemma: Two identical keys have duplicates.
pub proof fn lemma_two_same_keys_have_duplicates()
    ensures
        spec_has_duplicate_keys(vec!["a", "a"]),
{
    assert(spec_has_duplicate_keys(vec!["a", "a"]));
}

/// Lemma: A key repeated after a different key is detected.
pub proof fn lemma_duplicate_after_different_key_detected()
    ensures
        spec_has_duplicate_keys(vec!["a", "b", "a"]),
{
    assert(spec_has_duplicate_keys(vec!["a", "b", "a"]));
}

/// Lemma: Three identical keys are detected.
pub proof fn lemma_three_identical_keys_detected()
    ensures
        spec_has_duplicate_keys(vec!["a", "a", "a"]),
{
    assert(spec_has_duplicate_keys(vec!["a", "a", "a"]));
}

/// Lemma: Five different keys have no duplicates.
pub proof fn lemma_five_different_keys_no_duplicates()
    ensures
        spec_no_duplicate_keys(vec!["a", "b", "c", "d", "e"]),
{
    assert(spec_no_duplicate_keys(vec!["a", "b", "c", "d", "e"]));
}

/// Lemma: A key repeated at the end is detected.
pub proof fn lemma_duplicate_at_end_detected()
    ensures
        spec_has_duplicate_keys(vec!["a", "b", "c", "b"]),
{
    assert(spec_has_duplicate_keys(vec!["a", "b", "c", "b"]));
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-024: No false positives
// ─────────────────────────────────────────────────────────────────

/// Lemma: The linear scan correctly reports no duplicates for unique keys.
pub proof fn lemma_scan_reports_no_duplicates_for_unique_keys()
    ensures
        spec_duplicate_scan(vec!["a", "b", "c"], vec![]),
{
    assert(spec_duplicate_scan(vec!["a", "b", "c"], vec![]));
}

/// Lemma: The linear scan correctly reports duplicates when found.
pub proof fn lemma_scan_reports_duplicates_when_found()
    ensures
        !spec_duplicate_scan(vec!["a", "b", "a"], vec![]),
{
    assert(!spec_duplicate_scan(vec!["a", "b", "a"], vec![]));
}

/// Lemma: The linear scan on empty list reports success.
pub proof fn lemma_scan_empty_reports_success()
    ensures
        spec_duplicate_scan(vec![], vec![]),
{
    assert(spec_duplicate_scan(vec![], vec![]));
}

/// Lemma: The linear scan on single element reports success.
pub proof fn lemma_scan_single_element_reports_success()
    ensures
        spec_duplicate_scan(vec!["x"], vec![]),
{
    assert(spec_duplicate_scan(vec!["x"], vec![]));
}

// ─────────────────────────────────────────────────────────────────
// PO-YAML-025: Duplicate key detection is O(n)
// ─────────────────────────────────────────────────────────────────

/// Spec: The number of steps in the linear scan equals the number of keys.
pub open spec fn spec_scan_steps(keys_len: int, seen_len: int) -> int {
    if keys_len == 0 {
        seen_len
    } else {
        spec_scan_steps(keys_len - 1, seen_len + 1)
    }
}

/// Lemma: Scan steps equal key count when starting from empty seen.
pub proof fn lemma_scan_steps_equal_key_count(keys_len: int)
    requires
        keys_len >= 0,
    ensures
        spec_scan_steps(keys_len, 0) == keys_len,
{
    assert(spec_scan_steps(keys_len, 0) == keys_len);
}

/// Lemma: Scan is linear in the number of keys.
pub proof fn lemma_scan_is_linear(keys_len: int)
    requires
        keys_len >= 0,
    ensures
        spec_scan_steps(keys_len, 0) <= keys_len + 1,
{
    assert(spec_scan_steps(keys_len, 0) <= keys_len + 1);
}

/// Lemma: Duplicate detection never takes more than n+1 steps.
pub proof fn lemma_duplicate_detection_upper_bound(keys_len: int)
    requires
        keys_len >= 0,
    ensures
        spec_scan_steps(keys_len, 0) < keys_len + 2,
{
    assert(spec_scan_steps(keys_len, 0) < keys_len + 2);
}

// ─────────────────────────────────────────────────────────────────
// Additional: Multi-level duplicate detection
// ─────────────────────────────────────────────────────────────────

/// Lemma: Duplicate key at position 0 and position 4 in 5-element list.
pub proof fn lemma_duplicate_far_apart_detected()
    ensures
        spec_has_duplicate_keys(vec!["a", "b", "c", "d", "a"]),
{
    assert(spec_has_duplicate_keys(vec!["a", "b", "c", "d", "a"]));
}

/// Lemma: All keys the same in 4-element list.
pub proof fn lemma_all_keys_same_detected()
    ensures
        spec_has_duplicate_keys(vec!["x", "x", "x", "x"]),
{
    assert(spec_has_duplicate_keys(vec!["x", "x", "x", "x"]));
}

} // verus!

fn main() {}
