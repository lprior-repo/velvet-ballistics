//! Kani harnesses for KeyRange primitive operations.
//!
//! **Obligations:**
//! - PO-015: `KeyRange::contains(key)` returns true iff start <= key && key <= end
//! - PO-016: `KeyRange::intersection(a, b)` returns mathematical intersection
//! - PO-017: `KeyRange::is_disjoint(a, b)` is consistent with `intersection`
//!
//! **GOD RULE #1:** All harnesses use `kani::Arbitrary`.

#![forbid(unsafe_code)]

use super::KeyRange;

// ============================================================================
// PO-015: contains correctness
// ============================================================================

#[kani::proof]
fn key_range_contains_correct() {
    let range: KeyRange = kani::any();
    let key: u64 = kani::any();

    let result = range.contains(key);
    let expected = range.start() <= key && key <= range.end();

    //! Kani harnesses for KeyRange primitive operations.
//!
//! **Obligations:**
//! - PO-015: `KeyRange::contains(key)` returns true iff start <= key && key <= end
//! - PO-016: `KeyRange::intersection(a, b)` returns mathematical intersection
//! - PO-017: `KeyRange::is_disjoint(a, b)` is consistent with `intersection`
//!
//! **GOD RULE #1:** All harnesses use `kani::Arbitrary`.

#![forbid(unsafe_code)]

use super::KeyRange;

// ============================================================================
// PO-015: contains correctness
// ============================================================================

#[kani::proof]
fn key_range_contains_correct() {
    let range: KeyRange = kani::any();
    let key: u64 = kani::any();

    let result = range.contains(key);
    let expected = range.start() <= key && key <= range.end();

    kani::assert(
        result == expected,
        "contains must equal (start <= key && key <= end)",
    );

    kani::cover!(key == range.start(), "key at range start");
    kani::cover!(key == range.end(), "key at range end");
}

// ============================================================================
// PO-016: intersection correctness
// ============================================================================

#[kani::proof]
fn key_range_intersection_correct() {
    let a: KeyRange = kani::any();
    let b: KeyRange = kani::any();

    let result = a.intersection(b);

    let expected_start = if a.start() > b.start() {
        a.start()
    } else {
        b.start()
    };
    let expected_end = if a.end() < b.end() { a.end() } else { b.end() };

    if expected_start <= expected_end {
        match result {
            Some(r) => {
                kani::assert(r.start() == expected_start,
                    "intersection start must be max of inputs",
                );
                kani::assert(r.end() == expected_end,
                    "intersection end must be min of inputs",
                );
            }
            None => {
                 == expected_end,
                    "intersection end must be min of inputs",
                );
            }
            None => {
                kani::assert(
                    false,
                    "intersection must exist when expected_start <= expected_end",
                );
            }
        }
    } else {
        kani::assert(
            result.is_none(),
            "intersection must be None when ranges are disjoint",
        );
    }

    kani::cover!(
        a.start() == b.start() && a.end() == b.end(),
        "identical ranges"
    );
    kani::cover!(a.end() < b.start(), "a entirely before b");
}

// ============================================================================
// PO-017: is_disjoint consistency with intersection
// ============================================================================

#[kani::proof]
fn key_range_disjoint_consistent() {
    let a: KeyRange = kani::any();
    let b: KeyRange = kani::any();

    let disjoint = a.is_disjoint(b);
    let intersection_none = a.intersection(b).is_none();

    ,
            "intersection must be None when ranges are disjoint",
        );
    }

    kani::cover!(
        a.start() == b.start() && a.end() == b.end(),
        "identical ranges"
    );
    kani::cover!(a.end() < b.start(), "a entirely before b");
}

// ============================================================================
// PO-017: is_disjoint consistency with intersection
// ============================================================================

#[kani::proof]
fn key_range_disjoint_consistent() {
    let a: KeyRange = kani::any();
    let b: KeyRange = kani::any();

    let disjoint = a.is_disjoint(b);
    let intersection_none = a.intersection(b).is_none();

    kani::assert(
        disjoint == intersection_none,
        "is_disjoint must equal intersection().is_none()",
    );

    // Stronger: overlapping ranges must NOT be disjoint
    let overlap = a.start() <= b.end() && b.start() <= a.end();
    if overlap {
        .is_none()",
    );

    // Stronger: overlapping ranges must NOT be disjoint
    let overlap = a.start() <= b.end() && b.start() <= a.end();
    if overlap {
        kani::assert(!disjoint, "overlapping ranges must not be disjoint");
    }

    kani::cover!(a.end() < b.start(), "disjoint: a before b");
    kani::cover!(overlap, "overlapping ranges covered");
}

// ============================================================================
// Supplemental: adjacency correctness (Kani exhaustive)
// ============================================================================

#[kani::proof]
fn key_range_adjacent_correctness() {
    let a: KeyRange = kani::any();
    let b: KeyRange = kani::any();

    let result = a.is_adjacent_to(b);

    let expected =
        a.end().checked_add(1) == Some(b.start()) || b.end().checked_add(1) == Some(a.start());

     < b.start(), "disjoint: a before b");
    kani::cover!(overlap, "overlapping ranges covered");
}

// ============================================================================
// Supplemental: adjacency correctness (Kani exhaustive)
// ============================================================================

#[kani::proof]
fn key_range_adjacent_correctness() {
    let a: KeyRange = kani::any();
    let b: KeyRange = kani::any();

    let result = a.is_adjacent_to(b);

    let expected =
        a.end().checked_add(1) == Some(b.start()) || b.end().checked_add(1) == Some(a.start());

    kani::assert(
        result == expected,
        "is_adjacent_to must match mathematical definition",
    );

    // Symmetry
    kani::assert(
        a.is_adjacent_to(b) == b.is_adjacent_to(a),
        "is_adjacent_to must be symmetric",
    );

    // Overlapping ranges are never adjacent
    if a.intersection(b).is_some() {
         == b.is_adjacent_to(a),
        "is_adjacent_to must be symmetric",
    );

    // Overlapping ranges are never adjacent
    if a.intersection(b).is_some() {
        kani::assert(!result, "overlapping ranges must not be adjacent");
    }

    kani::cover!(
        a.end().checked_add(1) == Some(b.start()),
        "a adjacent before b"
    );
    kani::cover!(
        b.end().checked_add(1) == Some(a.start()),
        "b adjacent before a"
    );
}
