#![forbid(unsafe_code)]
//! PRNG reference tests for the seeded autonomous scheduler facade.
//!
//! splitmix64 is the canonical "good enough" deterministic PRNG for
//! replay-style exploration; the tests below pin its contract against
//! the published reference implementation (Steele/Lea/Flood, 2014).

use crate::scheduler::rng::RngState;

#[test]
fn splitmix_zero_seed_produces_known_value() {
    // splitmix64 with seed 0 -> first value is 0xE220A8397B1DCDAF.
    // Verified against the published reference implementation.
    let mut rng = RngState::new(0);
    let first = rng.next_u64();
    assert_eq!(first, 0xE220A8397B1DCDAF);
}

#[test]
fn splitmix_same_seed_same_stream() {
    let mut a = RngState::new(42);
    let mut b = RngState::new(42);
    for _ in 0..16 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn splitmix_different_seed_different_stream() {
    let mut a = RngState::new(1);
    let mut b = RngState::new(2);
    let mut diverged = false;
    for _ in 0..8 {
        if a.next_u64() != b.next_u64() {
            diverged = true;
            break;
        }
    }
    assert!(diverged, "different seeds must produce different streams");
}

#[test]
fn splitmix_bounded_never_exceeds_bound() {
    let mut rng = RngState::new(12345);
    for _ in 0..1024 {
        let v = rng.next_bounded(7);
        assert!(v < 7, "bounded value {v} exceeded bound 7");
    }
}

#[test]
fn splitmix_bounded_zero_returns_zero() {
    let mut rng = RngState::new(99);
    assert_eq!(rng.next_bounded(0), 0);
}

#[test]
fn splitmix_u32_is_upper_32_bits_of_u64() {
    // `next_u32` decomposes the upper 32 bits of `next_u64` via
    // big-endian byte slicing; assert the equivalence directly.
    let mut rng = RngState::new(7);
    let u32_value = rng.next_u32();
    let mut rng2 = RngState::new(7);
    let u64_value = rng2.next_u64();
    let expected = u32::try_from(u64_value >> 32).unwrap_or(0);
    assert_eq!(u32_value, expected);
}

#[test]
fn splitmix_raw_state_is_zero_after_construction_then_advances() {
    let rng = RngState::new(0);
    assert_eq!(rng.raw_state(), 0);
    let mut rng = rng;
    let _ = rng.next_u64();
    // After one draw the state must have advanced from 0.
    assert_ne!(rng.raw_state(), 0);
}
