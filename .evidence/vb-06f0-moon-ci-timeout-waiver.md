# vb-06f0 Moon CI Timeout Waiver

## Tooling Operational Status: EVIDENCE GAP ANALYSIS

This waiver replaces previously unsupported "fully verified" claims with
evidence-grounded status. Raw command logs and evidence directories are
incomplete for several tooling lanes.

### Evidence Audit

| Tool | Prior Claim | Actual Evidence Found | Status |
|------|-------------|----------------------|--------|
| Kani | 215 harnesses across 6 crates | 279 harnesses in `.evidence/kani-list/*.json` (6 files: vb_core=206, vb_runtime=35, vb_validate=31, vb_storage=3, vb_verification=3, vb_yaml=1) | PARTIAL — harness inventory exists; no raw verification log artifacts attached |
| Flux | Smoke passes all packages | No `.evidence/**/*flux*` files found | UNSUPPORTED — `scripts/flux-check-package.sh` exists but no execution logs in evidence store |
| Verus | Verification complete | 30 PASS files in `.evidence/verus/` (Verus 0.2026.05.05, 35 targets compiled, 30 verified) | PARTIAL — verification outputs exist; raw `verify-verus.sh` command logs not attached |
| Proptest | Integrated | Standard cargo integration (no separate evidence required) | ACCEPTED — cargo-native, no standalone evidence expected |
| Cargo-fuzz | 58 targets | 71 files in `fuzz/fuzz_targets/` | PARTIAL — target count under-reported (58→71); no raw fuzz run evidence in `.evidence/` |
| Loom | 13 concurrency tests | 11 `loom::model()` invocations across 6 source files in `crates/vb_runtime/src/models/loom/`; 2 loom evidence files (vb-ko29.7) | PARTIAL — invocation count under-reported (13→11); minimal loom evidence store |

### Test Results Summary

- **Prior claim:** 12,693 / 12,696 tests (99.98%) — self-referenced in this waiver only
- **Independent evidence:** None found in `.evidence/` (no moon CI logs, no test summary artifacts)
- **Status:** UNVERIFIED — the test count and pass/fail breakdown cannot be independently confirmed from evidence files
- **SIGTERM-cancelled:** 3 tests (`journal_side_index_contracts` suite) — claimed but no raw test log evidence
- **Individual pass:** All 3 cancelled tests pass in isolation — claimed but not independently verified

## Moon CI Timeout Analysis

**Root Cause (prior claim):** CI scheduling pressure from 12,696-test suite hitting 10-minute wall clock limit.

**Revised assessment:** The timeout root cause is plausible but cannot be independently verified
from evidence files. The following tooling lanes have at least partial evidence:

- Kani: Harness inventory confirmed (279 harnesses, 6 crates). Verification logs pending.
- Verus: 30 verification files PASS. Raw command logs pending.
- Fuzz: 71 targets exist in source. Fuzz run evidence pending.
- Loom: 11 concurrency test invocations exist. Loom run evidence partial.
- Flux: No evidence found in `.evidence/`. Needs re-execution and log capture.

## Unsupported Claims — Replaced with Accurate Status

1. ~~"Flux: ✓ Smoke passes all packages"~~ → **UNSUPPORTED**: No flux execution logs in `.evidence/`.
2. ~~"Kani: ✓ 215 harnesses"~~ → **CORRECTED**: 279 harnesses in kani-list inventory; no raw verification logs.
3. ~~"Fuzz: ✓ 58 targets"~~ → **CORRECTED**: 71 targets found; no raw fuzz evidence.
4. ~~"Loom: ✓ 13 concurrency tests"~~ → **CORRECTED**: 11 `loom::model()` invocations across 6 files; minimal evidence.
5. ~~"12,693 / 12,696 tests (99.98%)"~~ → **UNVERIFIED**: No independent moon CI test logs found.
6. ~~"All 3 cancelled tests pass when run in isolation"~~ → **CLAIMED**: No raw individual test logs attached.

## Waiver Justification (Revised)

1. **Partial verification:** Kani, Verus, and Fuzz have partial evidence (inventory/outputs exist without raw logs).
2. **Unverified claims:** Flux evidence is missing; test counts lack independent artifacts.
3. **No behavioral regression claimed but not proven:** Individual test pass evidence is self-referenced.
4. **CI scheduling hypothesis:** Plausible but unsubstantiated by evidence files.

## Required Follow-up (VB-CN2ZY)

- [ ] Execute `scripts/flux-check-package.sh` for each package and attach raw logs to `.evidence/flux/`
- [ ] Attach raw Kani verification logs (stdout/stderr) for each harness to `.evidence/kani/`
- [ ] Attach raw Verus `verify-verus.sh` execution log to `.evidence/verus/run.log`
- [ ] Capture moon CI test run with raw test count artifacts
- [ ] Run `journal_side_index_contracts*` tests individually and attach pass logs

## Sign-off (Revised)

This waiver **partially approves** landing vb-06f0 despite Moon CI timeout.
Tooling is **partially operational** — inventory and output artifacts exist for
Kani, Verus, and Fuzz; Flux evidence is missing; test count and individual test
evidence are unverified. The "fully verified" designation has been replaced with
accurate evidence-grounded status per VB-CN2ZY.
