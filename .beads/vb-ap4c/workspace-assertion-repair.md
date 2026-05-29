# vb-ap4c Workspace Assertion Fixture Repair

## Scope

Repaired the synthetic valid-workspace fixture used by
`crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs` so it matches the
current root workspace contract.

## Change

- Added `crates/vb_test_util` to the generated valid fixture workspace members.
- Added a generated `crates/vb_test_util/Cargo.toml` fixture manifest.
- Added `kani-diagnostic-codes = []` to the generated `vb_core` fixture feature
  set.

## Evidence

Commands run from `/home/lewis/src/velvet-ballistics`:

```text
bash scripts/check-workspace-assertions.sh
PASS

rtk cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions valid_workspace_passes_sharpened_assertions -- --nocapture
PASS: cargo test: 1 passed, 6 filtered out (1 suite, 0.10s)

rtk cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions -- --nocapture
PASS: cargo test: 7 passed (1 suite, 0.12s)

rtk cargo nextest run -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions
PASS: cargo nextest: 7 passed (1 binary, 0.119s)
```

## Residual Risk

Full `moon ci` was not rerun after this focused repair because the previous full
run exposed unrelated global blockers (`verify-tlc` missing `tla2tools.jar`,
source-length violations, and long-running `journal_side_index_contracts` tests).
This repair addresses the local workspace assertion failure only.
