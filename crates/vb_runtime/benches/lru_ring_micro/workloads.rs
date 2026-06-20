use super::backends::{LruRingBTree, LruRingIndex};
use super::types::{CAPACITY, RunId, SWEEP_NOW, TTL_TICKS, TimerTick};

#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(1_184_299_674_549_157_465);
    z = (z ^ (z >> 27)).wrapping_mul(4_297_025_584_627_948_071);
    z ^ (z >> 31)
}

/// Deterministic Fisher-Yates permutation of `0..n`.
pub(super) fn permutation(n: usize) -> Vec<RunId> {
    let mut v: Vec<RunId> = Vec::with_capacity(n);
    let mut k: u64 = 0;
    while (k as usize) < n {
        v.push(RunId::new(k));
        k = k.saturating_add(1);
    }
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let mut i = n;
    while i > 1 {
        i -= 1;
        let j = (splitmix64(&mut state) >> 33) as usize % (i + 1);
        v.swap(i, j);
    }
    v
}

fn fill_index(ring: &mut LruRingIndex, keys: &[RunId], now: TimerTick) {
    for &k in keys {
        match ring.insert(k, now) {
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

fn fill_btree(ring: &mut LruRingBTree, keys: &[RunId], now: TimerTick) {
    for &k in keys {
        match ring.insert(k, now) {
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

pub(super) fn full_index(now: TimerTick) -> LruRingIndex {
    let mut ring = LruRingIndex::new(CAPACITY, TTL_TICKS);
    let keys = permutation(CAPACITY);
    fill_index(&mut ring, &keys, now);
    ring
}

pub(super) fn full_btree(now: TimerTick) -> LruRingBTree {
    let mut ring = LruRingBTree::new(CAPACITY, TTL_TICKS);
    let keys = permutation(CAPACITY);
    fill_btree(&mut ring, &keys, now);
    ring
}

/// Builds a ring whose first half expires at `SWEEP_NOW` and second half lives.
pub(super) fn half_expired_index() -> LruRingIndex {
    let mut ring = LruRingIndex::new(CAPACITY, TTL_TICKS);
    let half = CAPACITY / 2;
    let keys = permutation(CAPACITY);
    let mut i = 0;
    while i < half {
        let k = keys[i];
        ring.push_existing(k, TimerTick::new(0));
        i += 1;
    }
    while i < CAPACITY {
        let k = keys[i];
        ring.push_existing(k, TimerTick::new(SWEEP_NOW));
        i += 1;
    }
    ring
}

/// Builds a `BTreeSet` ring with the same half-expired shape.
pub(super) fn half_expired_btree() -> LruRingBTree {
    let mut ring = LruRingBTree::new(CAPACITY, TTL_TICKS);
    let half = CAPACITY / 2;
    let keys = permutation(CAPACITY);
    let mut i = 0;
    while i < half {
        let k = keys[i];
        ring.push_existing(k, TimerTick::new(0));
        i += 1;
    }
    while i < CAPACITY {
        let k = keys[i];
        ring.push_existing(k, TimerTick::new(SWEEP_NOW));
        i += 1;
    }
    ring
}
