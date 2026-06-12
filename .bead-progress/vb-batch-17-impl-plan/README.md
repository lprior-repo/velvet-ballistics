# vb-batch-17-impl-plan

**Batch:** 17 Ready Beads Implementation Plan  
**Generated:** 2026-06-11  
**Status:** ANALYSIS COMPLETE — Ready for Implementation  

---

## Documents in This Directory

| File | Description |
|------|-------------|
| `README.md` | This file — master index |
| `IMPLEMENTATION.md` | Full implementation plan with phase ordering, risk register |
| `QUICK-REF.md` | Quick reference card — all beads, hours, files |
| `PER-BEAD-DETAILS.md` | Detailed per-bead implementation guide |

---

## Bead Roster

| Bead | Priority | Title | Phase | Est. Hours |
|------|----------|-------|-------|------------|
| vb-a408a | P1 | Reconcile stale ArrayQueue prose with Gate 8 scope | 1 | 1h + decision |
| vb-6xh6c | P1 | Repair remaining Gate 8 Kani 120s timeouts | 6 | 20h |
| vb-81urc | P1 | Wire test-determinism into CI | 2 | 63h |
| vb-9ckqp | P2 | Red Queen BLOCKER: vb_validate pre-existing-build | 1 | 5min |
| vb-f2xk3 | P2 | Red Queen BLOCKER: vb_core pre-existing-build | 1 | 5min |
| vb-benchbn01 | P2 | Add 2 missing benchmark groups | 6 | 4h |
| vb-wstlsl01 | P2 | Delete self-laundering Section 17 tests | 6 | 2h |
| vb-strecov01 | P2 | Add error_recovery test for fuzz-malformed journal | 5 | 8h |
| vb-stortst01 | P2 | Split tests.rs (8,091 LoC) under 300-line cap | 5 | 40h |
| vb-ybzsz | P2 | Wire flux-check-package.sh and loom task | 2 | 4h |
| vb-dxi1k | P2 | Add verify-kani-vb-validate to pipeline | 2 | 5min |
| vb-27jox | P2 | Split output.rs under 300-line cap | 3 | 2h |
| vb-jy6re | P2 | Split cli_postcard/types.rs under 300-line cap | 3 | 4h |
| vb-e6xr7 | P2 | Split errors.rs under 300-line cap | 3 | 8h |
| vb-9zy8r | P2 | Split frame.rs under 300-line cap | 4 | 12h |
| vb-p9owu | P2 | Split diagnostic.rs under 300-line cap | 4 | 16h |
| vb-32gwc | P2 | Split budget.rs under 300-line cap | 4 | 20h |

**Total Estimated Hours:** ~204h + decision

---

## Phase Execution Order

### Phase 1: Quick Fixes + Decision
1. **vb-9ckqp** (5min) — Change `--expect_exit 0` → `--expect_exit 1` in Red Queen smoke test
2. **vb-f2xk3** (5min) — Same fix for vb_core
3. **vb-a408a** (1h + decision) — Decision bead; no code until scope resolved

### Phase 2: CI Wiring + Probes
4. **vb-dxi1k** (5min) — One-line edit: wire verify-kani-vb-validate into pipeline
5. **vb-ybzsz** (4h) — Create `.moon/tasks/flux.yml` + `.moon/tasks/loom.yml`
6. **vb-81urc** (63h) — T0 archive baseline + T2-T3 script + T4-T5 flip runInCI

### Phase 3: File Splits (Medium)
7. **vb-27jox** (2h) — output.rs split (already 280 lines, just needs directory)
8. **vb-jy6re** (4h) — types.rs → types/ directory (27 re-exports preserved)
9. **vb-e6xr7** (8h) — errors.rs → errors/ + stale ledger fix

### Phase 4: File Splits (Large)
10. **vb-9zy8r** (12h) — frame.rs → frame/ + kani-frame feature isolation
11. **vb-p9owu** (16h) — diagnostic.rs → diagnostic/ (CODE_REGISTRY constraint)
12. **vb-32gwc** (20h) — budget.rs → budget/ (orphaned dir exists)

### Phase 5: Storage Tests
13. **vb-strecov01** (8h) — Create 10 error_recovery tests
14. **vb-stortst01** (40h) — Split 8,215-line tests.rs into ~30 files

### Phase 6: Kani + Benchmarks + Tests
15. **vb-6xh6c** (20h) — Archive logs + bounded kani::any() + feature isolation
16. **vb-benchbn01** (4h) — Create warm_throughput + digest_computation benchmarks
17. **vb-wstlsl01** (2h) — Delete self-laundering tests (will break until codes implemented)

---

## Key Constraints

### Source-Length Exceptions Ledger

| Bead | Action Required |
|------|-----------------|
| vb-e6xr7 | Delete stale row at `.config/source-length-exceptions.txt:85` |

### Feature Flags Required

| Bead | Feature | Location |
|------|---------|----------|
| vb-9zy8r | `kani-frame = []` | vb_core/Cargo.toml |

### Critical Re-Export Blocks

| Bead | File | Items |
|------|------|-------|
| vb-jy6re | cli_postcard/mod.rs | 27 re-exports |
| vb-e6xr7 | errors/mod.rs | 10+ items |
| vb-9zy8r | frame/mod.rs | StepState, RunFrame |
| vb-p9owu | diagnostic/mod.rs | Full public API |
| vb-32gwc | budget/mod.rs | 10 public items |

### Single-Const Constraints

| Bead | Const | Location | Reason |
|------|-------|----------|--------|
| vb-p9owu | `CODE_REGISTRY` | diagnostic/codes.rs | Single source of truth for symbolic code identity |

### Dependency Chain

```
vb-a408a (decision) ─┐
                      ├─ vb-dxi1k (depends on vb-e5lfm unwinds)
vb-6xh6c ─────────────┤
                      └─ vb-ybzsz.2 (depends on vb-lbg3h)
```

---

## Verification Commands

```bash
# Build gate
cargo build --workspace

# Test gate
cargo test --workspace

# Lint gate
moon :lint-src

# Source length gate
moon :source-length

# Full CI
moon ci

# Specific Kani validate
moon run :verify-kani-vb-validate

# Bench compile check
cargo bench --no-run
```

---

## File Modification Summary

| Category | Count | Examples |
|----------|-------|----------|
| New benchmark files | 2 | warm_throughput.rs, digest_computation.rs |
| New moon task files | 2 | flux.yml, loom.yml |
| New test files | 1+ | recovery/tests/error_recovery_tests.rs |
| Split targets | 6 | errors.rs, frame.rs, diagnostic.rs, budget.rs, tests.rs, types.rs |
| Module directories | 3 | output/, tests/, budget/ |
| Config edits | 3 | .moon.yml, Cargo.toml, source-length-exceptions.txt |
| Script modifications | 1 | check-test-determinism.py |
