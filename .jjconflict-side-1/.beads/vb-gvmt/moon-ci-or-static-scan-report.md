# CI / Static Evidence: vb-gvmt

## Canonical Gate

```bash
moon ci
```

## Result

- Status: PASS
- Observed evidence: `Tasks: 19 completed (1 cached)`
- Runtime: `Time: 1m 37s 538ms`
- Test evidence inside gate: nextest `8276 tests run: 8276 passed, 0 skipped`

## Additional Direct Gates

- `rtk cargo fmt --all --check`: failed before formatting, then `rtk cargo fmt --all` was run; `moon ci` subsequently passed its `fmt` task.
- `rtk cargo test -p vb_expr edge_f64_rejected_by_integer_addition -- --nocapture && rtk cargo test -p vb_expr edge_f64_rejected_by_comparison -- --nocapture`: PASS, each targeted test passed.
- `rtk cargo test -p vb_expr --lib`: PASS, `cargo test: 304 passed (1 suite, 0.02s)`.
- `rtk cargo clippy -p vb_codegen --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock`: PASS, `cargo clippy: No issues found`.
