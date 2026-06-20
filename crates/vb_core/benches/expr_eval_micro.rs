//! Microbench: `vb_core` expression-evaluator list/object kernels.
//!
//! Purpose: produce EVIDENCE for whether the current O(n^2) / O(L*R)
//! algorithms in the per-step HOT path (`crates/vb_core/src/engine/expr_eval/`)
//! are a measurable latency hazard at production list/object sizes, and how
//! much the candidate O(n) fixes would save.
//!
//! Mirrors the self-contained pattern of `crates/vb_runtime/benches/lru_ring_micro.rs`:
//! BOTH the current and candidate algorithms are defined LOCALLY in this file,
//! byte-for-byte faithful to production logic. No production code is imported,
//! no production code is modified. This bench exists to justify (or reject) a
//! rule exception BEFORE any production change.
//!
//! # What is being compared
//!
//! - `eval_unique` (production: `ops_text_list.rs:203`):
//!     * CURRENT  -> `Vec<SlotValue>` + `seen.contains(&item)` per item  = O(n^2)
//!     * CANDIDATE -> `IndexSet<SlotValue>` collect (order-preserving)    = O(n)
//! - `eval_merge` (production: `ops.rs:182` `eval_merge_combine_fields`):
//!     * CURRENT  -> clone left + `merged.iter().position()` per right   = O(L + R*L)
//!     * CANDIDATE -> `HashMap<SymbolId, usize>` position index           = O(L + R)
//! - `eval_append` (production: `ops_text_list.rs:166`):
//!     * CURRENT  -> N appends, each cloning the whole list + re-insert   = O(n^2) cumulative
//!       (no candidate here: the fix is a design decision — persistent vector
//!        or batched builder — deferred until the pain is quantified below.)
//!
//! # Workload characterization
//!
//! - Workload: per-step IR expression-evaluator list/object operations.
//! - Hot path: `eval_unique` / `eval_merge` / `eval_append` inside the
//!   deterministic `drive_deterministic` transition loop. Classified HOT by
//!   `scripts/hotpath-scan.sh` (path `crates/vb_core/src/engine/expr_eval/`).
//! - Sizes N: {64, 1024, 8192, 65536}. 65536 == `MAX_LIST_ITEMS_PER_VALUE`
//!   and `MAX_OBJECT_FIELDS_PER_VALUE` (`crates/vb_core/src/limits.rs`).
//! - Distribution: ~30% duplicate rate for unique (realistic dedup load);
//!   ~50% key overlap between merge sides; mixed I64/Bool/Symbol values.
//! - Target HW: see report; bench pins no CPU flags.
//!
//! # Why a local SlotValue mirror
//!
//! Production `SlotValue` does not yet impl `Hash` (the `FiniteF64` variant
//! has no Hash). The CANDIDATE `eval_unique` needs `Hash`. Rather than modify
//! the production type before evidence exists, this bench defines a local
//! `SV` enum that mirrors `SlotValue` and includes the 5-line CORRECT manual
//! `Hash` (with `-0.0` normalization so `hash(-0.0)==hash(+0.0)`, consistent
//! with `PartialEq`). This is exactly the impl proposed for production, so the
//! candidate measurement is faithful.

// Bench targets are excluded from the strict source lint gate (see existing
// benches). We keep the bench unsafe-free per task constraints.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::iter_over_hash_type,
    clippy::let_underscore_must_use,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::cast_lossless,
    clippy::module_inception,
    clippy::collapsible_else_if,
    clippy::manual_div_ceil
)]
#![allow(dead_code)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use indexmap::IndexSet;
use std::hint::black_box;

// ============================================================================
// Local type mirrors (faithful to production).
// ============================================================================

/// Local mirror of `vb_core::value::FiniteF64` (`#[repr(transparent)]` f64 that
/// rejects NaN/Inf at construction). Includes the CORRECT manual Hash proposed
/// for production: `-0.0` normalizes to `+0.0` so hashing is consistent with
/// `PartialEq` (which says `-0.0 == +0.0`).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
struct FF(f64);
impl Eq for FF {}
impl Hash for FF {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // PartialEq: -0.0 == +0.0, therefore their hashes MUST be equal.
        // A naive f64::to_bits() hash would violate the Hash/Eq contract here.
        let normalized = if self.0 == 0.0 { 0.0_f64 } else { self.0 };
        normalized.to_bits().hash(state);
    }
}

/// Local mirror of `vb_core::value::SlotValue` (Copy + Eq handle enum). The
/// handle variants mirror `SymbolId`/`ListId`/`ObjectId`/`BlobId` u32/u64
/// newtypes, all of which derive Hash in production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SV {
    Null,
    Bool(bool),
    I64(i64),
    F64(FF),
    Sym(u32),
    List(u32),
    Obj(u32),
    Blob(u64),
}

/// Local mirror of `vb_core::value_store::ObjectField` (key + value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field {
    key: u32,
    value: SV,
}

// ============================================================================
// Deterministic PRNG (SplitMix64, identical to lru_ring_micro.rs).
// ============================================================================

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

/// Builds a list of `n` SlotValues with a target duplicate rate so `unique`
/// does real (but not trivial) work. Value mix mirrors realistic expression
/// results: mostly I64, some Bool, some Symbol. Includes both -0.0 and +0.0
/// F64 values so the candidate Hash's -0.0 normalization is exercised.
fn make_list(n: usize, dup_rate: u64, seed: u64) -> Vec<SV> {
    let mut out = Vec::with_capacity(n);
    let mut state = seed;
    let pool = n / 3usize.max(1); // pool of distinct values to draw duplicates from
    let pool = if pool == 0 { 1 } else { pool };
    let mut distinct: Vec<SV> = Vec::with_capacity(pool);
    for _ in 0..pool {
        let r = splitmix64(&mut state);
        let v = match r % 10 {
            0..=6 => SV::I64((r as i64).wrapping_shr(8)),
            7 => SV::Bool(r % 2 == 0),
            8 => SV::Sym((r >> 32) as u32),
            _ => SV::F64(FF(if r % 3 == 0 { -0.0 } else { (r as f64) * 1.5 })),
        };
        distinct.push(v);
    }
    for i in 0..n {
        // draw from pool with prob `dup_rate/100`, else fresh value
        let r = splitmix64(&mut state);
        let pick_existing = (r % 100) < dup_rate;
        if pick_existing && !distinct.is_empty() {
            let idx = (splitmix64(&mut state) as usize) % distinct.len();
            out.push(distinct[idx]);
        } else {
            out.push(SV::I64((i as i64).wrapping_mul(765_432_1)));
        }
    }
    out
}

/// Builds two field slices for `eval_merge` with ~`overlap_pct`% of right keys
/// already present in left.
fn make_merge_sides(
    left_n: usize,
    right_n: usize,
    overlap_pct: u64,
    seed: u64,
) -> (Vec<Field>, Vec<Field>) {
    let mut state = seed;
    let mut left = Vec::with_capacity(left_n);
    for k in 0..left_n {
        left.push(Field {
            key: k as u32,
            value: SV::I64(k as i64 * 3),
        });
    }
    let mut right = Vec::with_capacity(right_n);
    for j in 0..right_n {
        let r = splitmix64(&mut state);
        let pick_existing = (r % 100) < overlap_pct;
        let key = if pick_existing && !left.is_empty() {
            // overlap: reuse a left key (so merge overwrites)
            (splitmix64(&mut state) as usize) % left.len()
        } else {
            left_n + j // fresh key
        };
        right.push(Field {
            key: key as u32,
            value: SV::I64(-(j as i64)),
        });
    }
    (left, right)
}

// ============================================================================
// eval_unique — CURRENT (O(n^2), mirrors ops_text_list.rs:203 production).
// ============================================================================

fn unique_current(items: &[SV]) -> Vec<SV> {
    let mut seen: Vec<SV> = Vec::new();
    for &item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

// ============================================================================
// eval_unique — CANDIDATE (O(n), IndexSet order-preserving collect).
// ============================================================================

fn unique_candidate(items: &[SV]) -> Vec<SV> {
    let set: IndexSet<SV> = items.iter().copied().collect();
    set.into_iter().collect()
}

// ============================================================================
// eval_merge — CURRENT (O(L+R*L), mirrors ops.rs:182 production).
// ============================================================================

fn merge_current(left: &[Field], right: &[Field]) -> Vec<Field> {
    let mut merged: Vec<Field> = left.to_vec();
    for &field in right {
        if let Some(pos) = merged.iter().position(|f| f.key == field.key) {
            if let Some(entry) = merged.get_mut(pos) {
                *entry = field;
            }
        } else {
            merged.push(field);
        }
    }
    merged
}

// ============================================================================
// eval_merge — CANDIDATE (O(L+R), HashMap<SymbolId,usize> position index).
// ============================================================================

fn merge_candidate(left: &[Field], right: &[Field]) -> Vec<Field> {
    let mut merged: Vec<Field> = left.to_vec();
    // Position index: key -> index in `merged`. Seeded from left.
    let mut pos: HashMap<u32, usize> = HashMap::with_capacity(left.len() + right.len());
    for (i, f) in merged.iter().enumerate() {
        pos.entry(f.key).or_insert(i);
    }
    for &field in right {
        match pos.get(&field.key).copied() {
            Some(i) => {
                if let Some(entry) = merged.get_mut(i) {
                    *entry = field;
                }
            }
            None => {
                let i = merged.len();
                pos.insert(field.key, i);
                merged.push(field);
            }
        }
    }
    merged
}

// ============================================================================
// eval_append — CURRENT cumulative cost (O(n^2), mirrors ops_text_list.rs:166).
// Models an agent fan-out loop doing N single-item appends, each cloning the
// whole list (production semantics: lists are immutable arena values, so every
// append materializes a fresh Vec).
// ============================================================================

fn append_loop_current(n: usize, seed: u64) -> Vec<SV> {
    let items = make_list(n, 0, seed);
    let mut acc: Vec<SV> = Vec::new();
    for &item in &items {
        let mut grown = acc.clone(); // O(n) clone every iteration
        grown.push(item);
        acc = grown;
    }
    acc
}

/// Upper-bound savings reference: the SAME final list built by a single
/// `extend` (batch), showing the cost if appends could be coalesced. NOT a
/// drop-in replacement for `eval_append` (semantics differ) — it bounds the
/// achievable win and frames the design decision.
fn append_batch_reference(n: usize, seed: u64) -> Vec<SV> {
    let items = make_list(n, 0, seed);
    let mut acc: Vec<SV> = Vec::with_capacity(n);
    acc.extend(items);
    acc
}

// ============================================================================
// Benchmark groups.
// ============================================================================

const SIZES: &[usize] = &[64, 1024, 8192, 65536];

fn bench_eval_unique(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_unique");
    for &n in SIZES {
        let input = make_list(n, 30, 0xC0FFEE);
        group.throughput(Throughput::Elements(n as u64));
        // Smaller sample size for very large n to keep wall time bounded.
        if n >= 8192 {
            group.sample_size(20);
        }
        group.bench_with_input(BenchmarkId::new("current_O_n2", n), &input, |b, inp| {
            b.iter(|| {
                let out = unique_current(black_box(inp));
                black_box(out);
            });
        });
        group.bench_with_input(BenchmarkId::new("candidate_O_n", n), &input, |b, inp| {
            b.iter(|| {
                let out = unique_candidate(black_box(inp));
                black_box(out);
            });
        });
    }
    group.finish();
}

fn bench_eval_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_merge");
    for &n in SIZES {
        let (left, right) = make_merge_sides(n, n, 50, 0xBA5EBA11);
        let total = n as u64;
        group.throughput(Throughput::Elements(total));
        if n >= 8192 {
            group.sample_size(20);
        }
        group.bench_with_input(BenchmarkId::new("current_O_LxR", n), &left, |b, l| {
            b.iter(|| {
                let out = merge_current(black_box(l), black_box(&right));
                black_box(out);
            });
        });
        group.bench_with_input(BenchmarkId::new("candidate_O_LpR", n), &left, |b, l| {
            b.iter(|| {
                let out = merge_candidate(black_box(l), black_box(&right));
                black_box(out);
            });
        });
    }
    group.finish();
}

fn bench_eval_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_append_cumulative");
    for &n in SIZES {
        group.throughput(Throughput::Elements(n as u64));
        if n >= 1024 {
            group.sample_size(10);
        }
        if n >= 8192 {
            group.sample_size(10);
        }
        group.bench_with_input(BenchmarkId::new("current_O_n2", n), &n, |b, &n| {
            b.iter(|| {
                let out = append_loop_current(black_box(n), 0xDECAFBAD);
                black_box(out);
            });
        });
        group.bench_with_input(BenchmarkId::new("batch_reference", n), &n, |b, &n| {
            b.iter(|| {
                let out = append_batch_reference(black_box(n), 0xDECAFBAD);
                black_box(out);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_eval_unique,
    bench_eval_merge,
    bench_eval_append,
);
criterion_main!(benches);
