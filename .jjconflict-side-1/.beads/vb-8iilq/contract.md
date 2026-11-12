# VB-8IILQ: Remove Test-Only Shard Submission Fabrication

## Contract

ONE production implementation of `handle_submit` and `handle_submit_with_inputs` in all builds, no silently substituted action contracts/caps/I64(0) input. Legitimate action tests must explicitly supply typed contracts/grants/inputs at test fixture calls; real production missing/invalid inputs/contracts/caps remain rejected. No all-in-one test wrapper secretly overrides restricted grants; retain negative admission and actual API semantics.

## Scope

- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` (production handlers)
- Test fixtures/call-sites in `crates/vb_runtime/src/shard/tests/` and `crates/vb_runtime/src/shard/lifecycle_tests/`
- `crates/vb_runtime/src/runtime_tests.rs` (runtime-level submit helpers)

## Acceptance Criteria

1. No `#[cfg(test)]` alternative implementations of `handle_submit` or `handle_submit_with_inputs`
2. No orphan `#[cfg(test)]` fabrication helpers (`test_first_do_action`, `test_contract_required_capability`, `test_contract_grants`, `test_action_contract`, `test_contracts_through`) in production lifecycle file
3. All shard tests compile and pass with explicit fixture calls
4. All runtime tests compile and pass with explicit fixture calls
5. No new `expect`/`unwrap`/`panic`/index/cast/ignored results in test code (except where explicitly required by existing test patterns)
6. Negative admission tests (missing/invalid contracts, denied capabilities) unchanged in expectation
