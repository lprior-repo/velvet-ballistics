# Regression Diff Report

Bead: `vb-qi37.26.1`  
Baseline: `.beads/vb-qi37.26.1/baseline-report.md`  
Current: Formal verification execution, 2026-05-19  

## Baseline Summary

The baseline was captured against the isolated workspace `/home/lewis/src/femdation-vb-qi37-26-1` with the following verified state:

- `cargo check --package velvet-ballistics-workspace-tests --tests` → PASS (exit 0, 0 errors)
- `cargo check --package vb_ipc` → PASS (exit 0, 0 errors)
- `crates/vb_ipc/src/server/handlers.rs` identical between source checkout and isolated workspace
- No regressions detected at baseline
- All workspace-tests compile prerequisites met

## Current Execution Results

| Obligation | Layer | Command | Result | Classification |
|---|---|---|---|---|
| COMP-001 | static-scan | `cargo check -p vb_ipc` | **PASS** | bead-local |
| COMP-002 | static-scan | `cargo check -p velvet-ballistics-workspace-tests --tests` | **PASS** | workspace |
| COMP-003 | static-scan | `cargo clippy -p vb_ipc -- -D warnings` | **PASS** | bead-local |
| SAFE-001 | static-scan | `grep` (diff-scoped against `0ebc5270`) | **PASS** | bead-local |
| SAFE-002 | static-scan | `grep` (diff-scoped against `0ebc5270`) | **PASS** | bead-local |
| ORPH-001 | static-scan | `ls` + `cargo check` | **PASS** | bead-local |
| TYPE-001 | static-scan | `cargo check` + `grep` | **PASS** | bead-local |

## Diff Classification

### No Regressions Detected

All 7 obligations that were **PASS** at baseline remain **PASS** post-fix:

- **COMP-001/002/003:** Compilation and clippy gates remain clean with zero errors/warnings.
- **SAFE-001/002:** No new unwrap/expect/panic/todo/unimplemented/unsafe introduced by the fix commit `0ebc5270`. Diff-scoped evidence confirms the changed regions are clean.
- **ORPH-001:** No `handlers/mod.rs` exists; build remains unbroken.
- **TYPE-001:** 227 typed enum variant usages confirmed (matching baseline count). No String literal regressions.

### New Issues
- **None.**

### Pre-existing Debt
- **None relevant to bead scope.** The 102 pre-existing `unwrap`/`expect` matches in `handlers.rs` are outside the changed regions and were not introduced by this fix. They are tracked as existing codebase debt, not bead-local regressions.

## Verdict

```
REGRESSION STATUS: CLEAN
BLOCKERS: 0
FAIL_LOCAL: 0
FAIL_REGRESSION: 0
WAIVED: 0
DEFERRED_GLOBAL: 0
```

The fix in commit `0ebc5270` (String→enum type mismatches resolved in `vb_ipc` handlers) introduces **no regressions** against the baseline. All compilation, safety, orphan-file, and type-consistency gates pass. The bead is approved for landing.
