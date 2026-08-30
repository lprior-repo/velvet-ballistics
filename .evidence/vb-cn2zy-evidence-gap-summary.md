# VB-CN2ZY Evidence Gap Summary

Generated: 2026-08-30
Parent: vb-06f0 Moon CI Timeout Waiver

## Gap 1: Flux Evidence Missing

`scripts/flux-check-package.sh` exists but no execution logs found in `.evidence/`.

**Remediation:** Run `bash scripts/flux-check-package.sh <package>` for each package and
attach raw stdout/stderr to `.evidence/flux/<package>-smoke.log`.

## Gap 2: Kani Verification Logs Missing

`.evidence/kani-list/*.json` contains harness inventory (279 harnesses across 6 crates)
but no raw Kani verification logs (stdout/stderr from actual `cargo kani` runs).

**Inventory:**
- vb_core: 206 harnesses
- vb_runtime: 35 harnesses
- vb_validate: 31 harnesses
- vb_storage: 3 harnesses
- vb_verification: 3 harnesses
- vb_yaml: 1 harness

**Remediation:** Attach raw Kani verification logs to `.evidence/kani/`.

## Gap 3: Fuzz Run Evidence Missing

`fuzz/fuzz_targets/` contains 71 fuzz targets (prior claim was 58).
No raw fuzz run evidence in `.evidence/`.

**Remediation:** Run fuzz targets and attach results to `.evidence/fuzz/`.

## Gap 4: Loom Evidence Incomplete

11 `loom::model()` invocations across 6 files in
`crates/vb_runtime/src/models/loom/`.
Only 2 loom-related evidence files found (in vb-ko29.7).

**Remediation:** Run loom tests and attach results to `.evidence/loom/`.

## Gap 5: Moon CI Test Logs Missing

Waiver claims "12,693 / 12,696 tests (99.98%)" but no moon CI test summary
artifacts found in `.evidence/`. The test count is self-referenced.

**Remediation:** Re-run moon CI and attach test count artifacts.
