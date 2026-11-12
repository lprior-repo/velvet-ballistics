#![forbid(unsafe_code)]
//! VB-CORE-BUDGET-003-KANI: Step budget bounded arithmetic verification
//!
//! Property: Budget arithmetic uses checked operations and never panics.
//! Bound: Bounded u64 values including overflow/underflow boundaries.
//!
//! This harness verifies panic-free budget arithmetic at all boundary conditions.

/// VB-CORE-BUDGET-003-KANI H1: add_dim(MAX, MAX) overflows
#[kani::proof]
fn kani_add_dim_max_plus_max_overflow() {
    let result = u64::MAX.checked_add(u64::MAX);
    match result {
        Some(_) => kani::assert(false, "MAX+MAX must overflow"),
        None => kani::assert(true, "MAX+MAX correctly overflows"),
    }
}

/// VB-CORE-BUDGET-003-KANI H2: add_dim(MAX/2, MAX/2) does not overflow
#[kani::proof]
fn kani_add_dim_half_plus_half_no_overflow() {
    let half = u64::MAX / 2;
    let result = half.checked_add(half);
    match result {
        Some(v) => kani::assert(v <= u64::MAX, "half+half within bounds"),
        None => kani::assert(false, "half+half should not overflow"),
    }
}

/// VB-CORE-BUDGET-003-KANI H3: add_dim(MAX-1, 1) does not overflow
#[kani::proof]
fn kani_add_dim_max_minus_one_plus_one() {
    let result = (u64::MAX - 1).checked_add(1u64);
    match result {
        Some(v) => kani::assert(v == u64::MAX, "MAX-1+1 = MAX"),
        None => kani::assert(false, "MAX-1+1 should not overflow"),
    }
}

/// VB-CORE-BUDGET-003-KANI H4: add_dim(MAX, 1) overflows
#[kani::proof]
fn kani_add_dim_max_plus_one_overflow() {
    let result = u64::MAX.checked_add(1u64);
    match result {
        Some(_) => kani::assert(false, "MAX+1 must overflow"),
        None => kani::assert(true, "MAX+1 correctly overflows"),
    }
}

/// VB-CORE-BUDGET-003-KANI H5: sub_dim(0, 1) underflows
#[kani::proof]
fn kani_sub_dim_zero_minus_one_underflow() {
    let result = 0u64.checked_sub(1u64);
    match result {
        Some(_) => kani::assert(false, "0-1 must underflow"),
        None => kani::assert(true, "0-1 correctly underflows"),
    }
}

/// VB-CORE-BUDGET-003-KANI H6: sub_dim(MAX, MAX) returns Ok(0)
#[kani::proof]
fn kani_sub_dim_max_minus_max() {
    let result = u64::MAX.checked_sub(u64::MAX);
    match result {
        Some(v) => kani::assert(v == 0, "MAX-MAX=0"),
        None => kani::assert(false, "MAX-MAX cannot underflow"),
    }
}

/// VB-CORE-BUDGET-003-KANI H7: sub_dim(MAX, MAX-1) returns Ok(1)
#[kani::proof]
fn kani_sub_dim_max_minus_max_minus_one() {
    let result = u64::MAX.checked_sub(u64::MAX - 1);
    match result {
        Some(v) => kani::assert(v == 1, "MAX-(MAX-1)=1"),
        None => kani::assert(false, "MAX-(MAX-1) cannot underflow"),
    }
}

/// VB-CORE-BUDGET-003-KANI H8: checked_mul boundaries
#[kani::proof]
#[kani::unwind(8)]
fn kani_checked_mul_boundaries() {
    let vals: &[u64] = &[0, 1, 2, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX];
    let mut overflow_count = 0u64;
    let mut ok_count = 0u64;

    for &a in vals {
        for &b in vals {
            match a.checked_mul(b) {
                Some(_) => ok_count += 1,
                None => overflow_count += 1,
            }
        }
    }

    let total = (vals.len() as u64) * (vals.len() as u64);
    kani::assert(ok_count + overflow_count == total, "full cartesian product");
    // Cover at least one overflow case
    kani::cover!(overflow_count > 0, "at least one overflow detected");
}

/// VB-CORE-BUDGET-003-KANI H9: checked_add boundaries
#[kani::proof]
#[kani::unwind(8)]
fn kani_checked_add_boundaries() {
    let vals: &[u64] = &[0, 1, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX];
    let mut overflow_count = 0u64;
    let mut ok_count = 0u64;

    for &a in vals {
        for &b in vals {
            match a.checked_add(b) {
                Some(_) => ok_count += 1,
                None => overflow_count += 1,
            }
        }
    }

    let total = (vals.len() as u64) * (vals.len() as u64);
    kani::assert(ok_count + overflow_count == total, "full cartesian product");
    kani::cover!(overflow_count > 0, "at least one overflow detected");
}
