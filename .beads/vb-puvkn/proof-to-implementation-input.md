# Proof → Implementation Bridge Input — vb-puvkn

## Rust Source Anchors

| Claim | Source File | Lines | Function |
|---|---|---|---|
| exec_shard_index_runtime | crates/vb_runtime/src/verification/verus/runtime_facade_api.rs | 126-142 | exec_shard_index_runtime |
| lemma_exec_shard_index_matches_spec | crates/vb_runtime/src/verification/verus/runtime_facade_api.rs | 145-159 | lemma_exec_shard_index_matches_spec |
| lemma_production_runtime_shard_index_eq_spec (NEW) | crates/vb_runtime/src/verification/verus/runtime_facade_api.rs | TBD | lemma_production_runtime_shard_index_eq_spec |
| Production Runtime::shard_index | crates/vb_runtime/src/runtime/mod.rs | (target line TBD) | Runtime::shard_index |
| Production RunId::shard_index | crates/vb_core/src/ids/mod.rs | 350 | RunId::shard_index |

## Independent Behavior Tests

| Test | File | Type | Cases |
|---|---|---|---|
| proptest_shard_index_zero_count_returns_zero  | crates/vb_runtime/tests/proptest_runtime_shard_index.rs | proptest | 4096 |
| proptest_shard_index_nonzero_eq_spec          | crates/vb_runtime/tests/proptest_runtime_shard_index.rs | proptest | 16384 |
| proptest_shard_index_bounded_by_count         | crates/vb_runtime/tests/proptest_runtime_shard_index.rs | proptest | 16384 |
| proptest_shard_index_idempotent_same_input    | crates/vb_runtime/tests/proptest_runtime_shard_index.rs | proptest | 4096 |
| proptest_shard_index_distribution_uniform     | crates/vb_runtime/tests/proptest_runtime_shard_index.rs | proptest | 32768 |

## Kani Harness References

| Harness | File | Spec Function |
|---|---|---|
| kani_production_shard_index_zero_shard_count       | crates/vb_runtime/src/verification/kani/runtime_facade_shard_index.rs | spec_shard_index |
| kani_production_shard_index_nonzero_matches_spec   | crates/vb_runtime/src/verification/kani/runtime_facade_shard_index.rs | spec_shard_index |
| kani_production_shard_index_idempotent_same_input  | crates/vb_runtime/src/verification/kani/runtime_facade_shard_index.rs | spec_shard_index |
| kani_production_shard_index_bounded_by_count       | crates/vb_runtime/src/verification/kani/runtime_facade_shard_index.rs | spec_shard_index |

## Required Evidence Commands

```
bash scripts/verify-verus.sh
bash scripts/kani-list.sh vb_runtime
cargo kani --harness kani_production_shard_index_zero_shard_count -p vb_runtime --features kani-runtime-facade
cargo kani --harness kani_production_shard_index_nonzero_matches_spec -p vb_runtime --features kani-runtime-facade
cargo kani --harness kani_production_shard_index_idempotent_same_input -p vb_runtime --features kani-runtime-facade
cargo kani --harness kani_production_shard_index_bounded_by_count -p vb_runtime --features kani-runtime-facade
cargo nextest run -p vb_runtime shard_index
```

## Implementation Rule

The implementation engineer MUST annotate the production `Runtime::shard_index`
method (via `extern_spec` in `runtime_facade_api.rs`) so the bridge is
provably bound to production. If the lemma cannot be proven with the
extern_spec pattern, fall back to extracting a shared helper, NOT to
adding `#[verifier::external_body]` to the bridge itself.