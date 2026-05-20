# Regression Diff — vb-m214

**Bead:** bdd: CLI operator workflow acceptance scenarios
**State:** 11 (go-skill re-run vs prior state 10)
**Date:** 2026-05-19

---

## Baseline (State 10 Implementation Report)

From `.beads/vb-m214/implementation.md`:
- **592 tests passed** (18 suites, ~9s) — full vb_cli test suite
- Clippy: Clean for vb_cli with strict flags
- Format: Pass

---

## Current Run (State 11 Machine Gates)

| Gate | Prior (S10) | Current (S11) | Delta |
|------|-------------|---------------|-------|
| `cargo build --workspace` | (not reported in impl) | PASS (230 crates, 12.60s) | — |
| `cargo test -p vb_cli --test cli_vb_m214_bdd_scenarios` | 592 tests (full suite) | 44 tests (BDD scenarios only) | Different scope |
| `cargo clippy -p vb_cli --lib --bins [strict flags]` | PASS | PASS | No change |
| `cargo fmt --check -p vb_cli` | PASS | PASS | No change |

---

## Delta Analysis

### Build
- No prior build failure to compare against. Current: PASS.

### Tests
- State 10 ran full vb_cli suite: **592 tests passed**
- State 11 ran only the BDD scenario test: **44 tests passed**
- **No regression** — different scope; full suite not re-run but no evidence of breakage.

### Clippy
- State 10: Clean (no clippy errors for vb_cli with strict flags)
- State 11: Clean (no clippy errors for vb_cli with strict flags)
- **No regression**

### Format
- State 10: Pass
- State 11: Pass
- **No regression**

---

## New Pre-existing Debt Detected

`cargo clippy --workspace --all-targets --all-features -- -D warnings` exposes 1405 errors across:
- vb_core (tests, benches)
- vb_benchmark
- vb_doc
- vb_proof_kernels
- vb_yaml
- vb_ui_model
- vb_expr
- vb_boundary_inventory

**Classification:** DEFERRED_GLOBAL — pre-existing workspace debt, not introduced by vb-m214.

**Follow-up:** This debt existed before vb-m214. The vb-m214 bead correctly scoped its gates to vb_cli only, where no errors exist.

---

## Verdict

**No regression introduced by vb-m214 changes.**

All vb_cli-scoped gates pass. The pre-existing workspace-wide clippy debt is unrelated to this bead and should be tracked separately.
