// Kani proof: NonEmptyVec invariants
// PO-K02: NonEmptyVec bounded-state invariants (C3.1-C3.3)
//
// Proves against the real NonEmptyVec<T> type:
//  1. from_vec(empty) returns None
//  2. new(x).first() == x, len() == 1, is_empty() == false
//  3. with_tail(x, tail).len() == 1 + tail.len()
//  4. len() >= 1 always
//  5. is_empty() always false
//  6. first() never panics (for any NonEmptyVec)
//  7. into_vec() round-trip preserves all elements
// Bounds: tail vec bounded to 0..15 for Kani model checking

#![forbid(unsafe_code)]

use crate::non_empty_vec::NonEmptyVec;

/// Maximum tail size for Kani bounded model checking.
const MAX_TAIL_SIZE: usize = 15;

#[kani::proof]
#[kani::unwind(16)]
fn nev_len_ge_one() {
    // Test with new()
    let head: i32 = kani::any();
    let nev = NonEmptyVec::new(head);
    assert!(nev.len() >= 1);
    assert!(!nev.is_empty());

    // Test with with_tail()
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let tail: Vec<i32> = (0..tail_size).map(|_| kani::any()).collect();
    let nev2 = NonEmptyVec::with_tail(head, tail);
    assert!(nev2.len() >= 1);
    assert!(!nev2.is_empty());
}

#[kani::proof]
fn nev_from_vec_empty() {
    let empty: Vec<i32> = Vec::new();
    let result = NonEmptyVec::from_vec(empty);
    assert!(result.is_none());
}

#[kani::proof]
fn nev_from_vec_non_empty() {
    let head: i32 = kani::any();
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let mut v: Vec<i32> = vec![head];
    for _ in 0..tail_size {
        v.push(kani::any());
    }

    let result = NonEmptyVec::from_vec(v.clone());
    assert!(result.is_some());
    let nev = match result {
        Some(n) => n,
        None => {
            kani::assert(false, "NonEmptyVec from non-empty vec");
            return;
        }
    };
    assert_eq!(*nev.first(), head);
    assert_eq!(nev.len(), v.len());
    assert!(!nev.is_empty());
}

#[kani::proof]
fn nev_with_tail_count() {
    let head: i32 = kani::any();
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let tail: Vec<i32> = (0..tail_size).map(|_| kani::any()).collect();

    let nev = NonEmptyVec::with_tail(head, tail.clone());
    assert_eq!(nev.len(), 1 + tail.len());
    assert!(nev.len() >= 1);
}

#[kani::proof]
fn nev_is_empty_false() {
    // Construct a NonEmptyVec via new()
    let head: i32 = kani::any();
    let nev = NonEmptyVec::new(head);
    assert!(!nev.is_empty());
    assert!(nev.len() >= 1);

    // Construct via with_tail with variable size
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let tail: Vec<i32> = (0..tail_size).map(|_| kani::any()).collect();
    let nev2 = NonEmptyVec::with_tail(head, tail);
    assert!(!nev2.is_empty());
    assert!(nev2.len() >= 1);
}

#[kani::proof]
fn nev_first_never_panics() {
    let head: i32 = kani::any();
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let tail: Vec<i32> = (0..tail_size).map(|_| kani::any()).collect();

    let nev = NonEmptyVec::with_tail(head, tail);

    // first() should NOT panic and should return &head
    let first: &i32 = nev.first();
    assert_eq!(*first, head);

    // last() should NOT panic
    let _last: &i32 = nev.last();
}

#[kani::proof]
fn nev_into_vec_round_trip() {
    let head: i32 = kani::any();
    let tail_size: usize = kani::any();
    kani::assume(tail_size <= MAX_TAIL_SIZE);
    let tail: Vec<i32> = (0..tail_size).map(|_| kani::any()).collect();

    let nev = NonEmptyVec::with_tail(head, tail.clone());
    let vec: Vec<i32> = nev.into_vec();

    // Round-trip: head + tail == vec
    assert_eq!(vec.len(), 1 + tail.len());
    // Use safe get() for index access
    if let Some(first) = vec.first() {
        assert_eq!(*first, head);
    } else {
        kani::assert(false, "empty vec after NonEmptyVec.into_vec()");
        return;
    }
    for (i, expected) in tail.iter().enumerate() {
        let idx = i.wrapping_add(1);
        if let Some(val) = vec.get(idx) {
            assert_eq!(*val, *expected);
        } else {
            kani::assert(false, "index out of bounds in into_vec round-trip");
            return;
        }
    }
}
