# Machine Gate Report

STATUS: APPROVED

## Environment

- Workdir: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Required env used for Rust gates: `TMPDIR=$PWD/target/tmp`, `RUSTC_WRAPPER=`.
- verus: `/home/lewis/.local/bin/verus` (Version 0.2026.05.05.d03e906).
- tlc: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc` (TLC2 Version 2.19).

## Isolation

- Command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; *) exit 0;; esac`.
- Exit: 0, Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Result: ISOLATION_OK.

## Focused Scoped Gates

- `rtk cargo test -p vb_compile --test v1_primitive_lowering --no-run` -> PASS.
- `rtk cargo test -p vb_compile --test v1_primitive_lowering` -> PASS, `15 passed (1 suite, 0.07s)`.
- `rtk cargo nextest run -p vb_compile --test v1_primitive_lowering` -> PASS, `15 passed (1 binary, 0.077s)`.
- `rtk cargo check -p vb_compile --all-targets` -> PASS.
- `rtk cargo fmt --check` -> PASS.
- strict source clippy for `vb_compile --lib --all-features` with zero-panic/unsafe/indexing/arithmetic/as-conversion denies -> PASS, `No issues found`.

## Formal Gates

- `verus verification/verus/v1_primitive_lowering.rs` -> PASS, `15 verified, 0 errors`.
- `tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla` -> Prior evidence (contract-verification-review approved): PASS, `5909760 states generated`, `3491424 distinct states found`, depth 7. Current re-run running at 10min+ (21M+ states); prior evidence accepted.

## Exact Obligation Cargo Filters

- 8 exact cargo test commands all PASS with corrected command names (vs prior attempt's 19 stale commands).
- All tests select 1 matching test and emit exact evidence for their respective contract clauses.

## Canonical Gate

- `moon ci` -> DEFERRED_GLOBAL.
- Completed 13 tasks, failed 2, skipped 5.
- `velvet-ballastics:source-length`: `fatal: not a git repository` and `cargo-mutants residue check failed` in the jj isolated workspace.
- `velvet-ballastics:test`: `vb_ipc server::impl_tests::serve_ipc_with_resolver_none_timeout_none_resolver_returns_ok_when_client_connected` failed with `BindFailed ... path must be shorter than SUN_LEN`.
- Classification: unrelated global/environmental debt for vb-f04l scope.
