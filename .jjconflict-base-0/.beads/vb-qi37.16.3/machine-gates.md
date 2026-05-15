bead_id: vb-qi37.16.3
phase: state-8
status: PASS_WITH_DEFERRED_GLOBAL

# Machine Gate / Regression Classification

Bead-local gates passed:

```bash
rtk cargo test -p vb_runtime --test durable_retry_red_phase  # PASS: 9 passed
rtk cargo test -p vb_runtime --lib                          # PASS: 1337 passed
rtk cargo test -p vb_runtime --test '*'                      # PASS: 18 passed
rtk cargo fmt --check -p vb_runtime                          # PASS
rtk cargo clippy -p vb_runtime --lib -- ...                  # PASS: 0 errors, 1 warning
```

Canonical gate:

```bash
moon ci
```

Classification: DEFERRED_GLOBAL.

Evidence: `moon ci` reached repo-wide format/lint tasks outside bead-local runtime scope and timed out. Reported diffs/errors were in unrelated paths such as `crates/vb_expr`, `crates/vb_proof_kernels`, `vb_storage` miri/kani helpers, `xtask`, and fuzz targets. State 1 baseline already proved canonical `moon ci` was not clean before bead-local edits due the isolated workspace `main` revision problem.

Conclusion: no bead-local regression detected; global gate debt deferred.
