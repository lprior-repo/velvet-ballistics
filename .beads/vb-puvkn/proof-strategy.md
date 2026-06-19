# Proof Strategy — vb-puvkn

**Bead:** vb-puvkn — bind runtime_facade_api exec fns to Runtime production methods
**Master sections:** §40, §44
**Status:** PARTIAL → planning full closure

## 1. Problem Frame

`crates/vb_runtime/src/verification/verus/runtime_facade_api.rs` defines:

- `spec_shard_index(run_id: u64, shard_count: u64) -> u64` — closed spec fn
  capturing `run_id % shard_count` with zero fallback.
- `exec_shard_index_runtime(run_id, shard_count) -> u64` — exec fn whose
  body literally re-implements the spec using `checked_rem(0).unwrap_or(0)`.
- `lemma_exec_shard_index_matches_spec` — proof fn proving the exec fn
  equals the spec.

The bead note observes:
> exec_shard_index_runtime(run_id, shard_count) — a separate function that
> reproduces the spec formula. The actual Runtime::shard_index is in a
> different file and is NOT proven to equal the spec.

This is the core binding gap. The current artifact proves that the
**exec fn** equals the spec, but does NOT prove that the **production
method** equals the spec. GOD RULE 2 prefers direct binding, so either:

(a) Annotate the production `Runtime::shard_index` method with a Verus
    `ensures` clause that quotes the spec.
(b) Extract a shared helper that both runtime_facade_api and production
    call, then annotate the helper.

Option (a) is simpler and aligns with the master recommendation.

## 2. Anti-Laundering Mandate

The current `exec_shard_index_runtime` body does NOT use
`#[verifier::external_body]`, but it duplicates the spec formula. The
plan MUST:

- Add a new `proof fn lemma_production_runtime_shard_index_eq_spec` that
  asserts the production method equals the spec, **with** an `assert ...`
  body that invokes `checked_rem` and proves the case split (zero vs
  non-zero).
- Strengthen `lemma_exec_shard_index_matches_spec` to use
  `assert(... by(compute))` on the `checked_rem(Some(...)) == Some(run_id % c)`
  equivalence.

## 3. Lane Selections

| Lane | Required? | Rationale |
|---|---|---|
| Verus (L4) | YES | shard_index is a pure u64 function; L4 gives strongest evidence. |
| Kani (L3) | YES | Zero vs non-zero shard_count is the binding boundary; Kani enumerates. |
| proptest (L1) | YES | Random u64 inputs cover the modulo distribution. |
| Flux | NO | Pure modulo function; Flux refines would duplicate Verus. |
| Loom | NO | shard_index is synchronous; concurrency is a separate Runtime concern. |
| cargo-fuzz | NO | No parser/codec at this layer. |
| TLA+ | NO | No temporal behavior. |

## 4. Risk Tags (from seed)

- `production-binding`: spec must equal production Runtime::shard_index.
- `spec-leakage`: exec fn in verification/verus/ cannot be a parallel
  implementation; it must reference production.
- `runtime-facade`: this is the runtime facade API, so correctness of
  shard_index is foundational.

## 5. Execution Order

1. Annotate `Runtime::shard_index` in `crates/vb_runtime/src/runtime/mod.rs`
   with `#[verifier::external] extern_spec` or extract a shared helper.
2. Add `lemma_production_runtime_shard_index_eq_spec` to
   `runtime_facade_api.rs` that proves production ≡ spec.
3. Strengthen `lemma_exec_shard_index_matches_spec` body with
   `assert(... by(compute))`.
4. Write `crates/vb_runtime/src/verification/kani/runtime_facade_shard_index.rs`
   with 4 harnesses.
5. Write `crates/vb_runtime/tests/proptest_runtime_shard_index.rs` with
   5 properties.
6. Run all gates, update bead.

## 6. Out of Scope

- `Runtime::submit_direct` and `Runtime::inspect_run` Verus specs (the
  bead notes them as lacking Verus coverage but defers to v0.2.0).
- Concurrency / spawn discipline for the runtime as a whole (covered by
  separate async-runtime audit beads).