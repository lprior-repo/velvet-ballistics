# vb-9cd Implementation

## Summary

Implemented `vb-core` cold value arenas backing handle-only `SlotValue` variants without adding owned payloads to hot slot values. `ValueStore` now preserves strings, lists, ordered object fields, and blobs behind deterministic insertion-order IDs.

## Files Changed

- `crates/vb-core/src/value_store.rs` — added `ValueStore` and `ObjectField` with checked insert/access APIs.
- `crates/vb-core/src/errors.rs` — added typed out-of-bounds errors for symbol/list/object/blob handles.
- `crates/vb-core/src/lib.rs` — exported the new value-store module and public types.
- `crates/vb-core/tests/phase1_core_types.rs` — covered payload roundtrip, invalid handles, deterministic IDs, and retained existing finite-float rejection coverage.
- `.github/workflows/ci.yml` — added required geiger/vet/bench/fuzz workflow steps so workspace scaffold verification passes.

## Contract Notes

- `SlotValue` remains handle-only and `Copy` compatible.
- IDs are derived from current arena length before insertion using checked integer conversions.
- All handle access uses checked slice lookup and returns `CoreError` variants; no panics or unchecked indexing were introduced.
- Allocation is limited to cold store insertion/construction paths, not `SlotValue` copy paths.

## Verification

Passed:

```text
rtk cargo fmt --all -- --check
rtk cargo test -p vb-core
rtk cargo test --workspace --all-targets
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Final output included:

```text
cargo test: 23 passed (3 suites, 0.00s)
cargo test: 145 passed (15 suites, 0.08s)
cargo clippy: No issues found
```

## Remaining Risks

- Compiler/runtime integration for constructing list/object/blob handles is intentionally not expanded in this bead.
- Store capacity/resource contracts are foundational only; future admission profiles should thread explicit arena limits into construction.
