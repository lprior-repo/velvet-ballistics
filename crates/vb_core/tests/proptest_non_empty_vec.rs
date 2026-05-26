// Proptest: NonEmptyVec round-trip and invariant properties
// PO-P02: NonEmptyVec properties (C3.3)
//
// Properties:
//  1. Round-trip: from_vec(v) → into_vec() preserves all elements and order
//  2. For-element iteration: into_iter() yields head first, then tail in order
//  3. with_tail(head, tail).into_vec() == [head] + tail
//  4. first() returns &head, last() returns last element
// Strategy: Vec<i32> of size 0..100, arbitrary elements

use proptest::prelude::*;
use vb_core::non_empty_vec::NonEmptyVec;

/// Strategy generating a NonEmptyVec of i32 with 1..100 elements.
fn non_empty_i32_vec() -> impl Strategy<Value = NonEmptyVec<i32>> {
    prop::collection::vec(any::<i32>(), 1..100).prop_map(|v| match NonEmptyVec::from_vec(v) {
        Some(nev) => nev,
        None => NonEmptyVec::new(0), // fallback, should never happen
    })
}

/// Strategy generating a Vec<i32> of 0..100 elements (may be empty).
fn optional_i32_vec() -> impl Strategy<Value = Vec<i32>> {
    prop::collection::vec(any::<i32>(), 0..100)
}

proptest! {
    #[test]
    fn round_trip_from_vec_into_vec_preserves_elements(
        elements in optional_i32_vec(),
    ) {
        let original = elements.clone();
        let maybe_nev = NonEmptyVec::from_vec(elements);

        match maybe_nev {
            None => {
                prop_assert!(original.is_empty());
            }
            Some(nev) => {
                prop_assert!(!original.is_empty());
                let recovered: Vec<i32> = nev.into_vec();
                prop_assert_eq!(recovered, original);
            }
        }
    }

    #[test]
    fn nev_len_always_ge_one(
        nev in non_empty_i32_vec(),
    ) {
        prop_assert!(nev.len() >= 1);
        prop_assert!(!nev.is_empty());
    }

    #[test]
    fn nev_first_returns_head(
        nev in non_empty_i32_vec(),
    ) {
        let first_val: i32 = *nev.first();
        let vec: Vec<i32> = nev.into_vec();
        let first_in_vec = vec.first().copied();
        prop_assert_eq!(first_val, first_in_vec.expect("vec must be non-empty"));
    }

    #[test]
    fn nev_last_returns_tail_end(
        nev in non_empty_i32_vec(),
    ) {
        // TF-VB-001 REPAIRED: Extract last() from nev BEFORE into_vec()
        // consumes it, then assert exact value against vec reference.
        let last_from_nev: i32 = *nev.last();
        let vec: Vec<i32> = nev.into_vec();
        let expected_last = vec.last().copied().expect("vec must be non-empty");
        prop_assert_eq!(last_from_nev, expected_last);
    }

    #[test]
    fn new_single_element_works(
        head in any::<i32>(),
    ) {
        let nev = NonEmptyVec::new(head);
        prop_assert_eq!(nev.len(), 1);
        prop_assert_eq!(*nev.first(), head);
        prop_assert!(!nev.is_empty());
    }

    #[test]
    fn with_tail_preserves_head_and_tail(
        head in any::<i32>(),
        tail in prop::collection::vec(any::<i32>(), 0..50),
    ) {
        let expected_len = 1 + tail.len();
        let nev = NonEmptyVec::with_tail(head, tail.clone());

        prop_assert_eq!(nev.len(), expected_len);
        prop_assert_eq!(*nev.first(), head);

        let recovered: Vec<i32> = nev.into_vec();
        if let Some(first_elem) = recovered.first() {
            prop_assert_eq!(*first_elem, head);
        }
        for (i, expected) in tail.iter().enumerate() {
            let idx = i.wrapping_add(1);
            if let Some(elem) = recovered.get(idx) {
                prop_assert_eq!(*elem, *expected);
            }
        }
    }

    #[test]
    fn is_empty_always_false(
        nev in non_empty_i32_vec(),
    ) {
        prop_assert!(!nev.is_empty());
    }

    // TF-VB-003: Mutating/iteration behavior tests
    // ---------------------------------------------------------

    #[test]
    fn push_increases_len_and_value_becomes_last(
        (mut nev, extra) in (non_empty_i32_vec(), any::<i32>()),
    ) {
        let old_len = nev.len();
        nev.push(extra);
        prop_assert_eq!(nev.len(), old_len + 1);
        prop_assert_eq!(*nev.last(), extra);
    }

    #[test]
    fn extend_appends_all_elements_preserving_order(
        (mut nev, tail) in (non_empty_i32_vec(),
            prop::collection::vec(any::<i32>(), 1..50)),
    ) {
        let old_len = nev.len();
        let tail_len = tail.len();
        let head_ref = *nev.first();
        nev.extend(tail.clone());

        prop_assert_eq!(nev.len(), old_len + tail_len);
        prop_assert_eq!(*nev.first(), head_ref);
        // Recover all elements and verify tail portion
        let recovered: Vec<i32> = nev.into_vec();
        prop_assert_eq!(&recovered[..1], &[head_ref]);
        prop_assert_eq!(&recovered[recovered.len() - tail_len..], &tail);
    }

    #[test]
    fn into_iter_yields_head_first_then_tail_in_order(
        nev in non_empty_i32_vec(),
    ) {
        let expected: Vec<i32> = nev.clone().into_vec();
        let collected: Vec<i32> = nev.into_iter().collect();
        prop_assert_eq!(collected, expected);
    }

    #[test]
    fn display_renders_comma_separated_elements(
        nev in non_empty_i32_vec(),
    ) {
        let rendered = format!("{nev}");
        // Rendered output must not be empty and must contain head element
        prop_assert!(!rendered.is_empty());
        let head_str = format!("{}", nev.first());
        prop_assert!(rendered.starts_with(&head_str));
        // Multi-element vecs must contain commas
        if nev.len() > 1 {
            prop_assert!(rendered.contains(','));
        }
    }
}

/// Zero-parameter tests moved outside proptest! block for compatibility
/// with proptest 1.11.0.

#[test]
fn from_vec_empty_returns_none() {
    let empty: Vec<i32> = Vec::new();
    assert!(NonEmptyVec::from_vec(empty).is_none());
}
