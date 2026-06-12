# 17-Ready Beads: Quick Reference

## Priority P1 (Fix First)

| Bead | Title | Action | Est. |
|------|-------|--------|------|
| vb-a408a | Reconcile stale ArrayQueue prose | **DECISION** — code blocked until scope resolved | 1h |
| vb-6xh6c | Repair Kani 120s timeouts | Archive logs + bounded kani::any() + feature isolation | 20h |
| vb-81urc | Wire test-determinism into CI | Archive baseline + modify script + flip runInCI | 63h |

## Priority P2: Quick Fix

| Bead | Action | Est. |
|------|--------|------|
| vb-9ckqp | Change `--expect_exit 0` → `--expect_exit 1` in Red Queen vb_validate check | 5min |
| vb-f2xk3 | Change `--expect_exit 0` → `--expect_exit 1` in Red Queen vb_core check | 5min |

## Priority P2: CI Wiring

| Bead | Action | Est. |
|------|--------|------|
| vb-dxi1k | One-line edit: add `verify-kani-vb-validate` to `.moon.yml` pipeline | 5min |
| vb-ybzsz | Create `.moon/tasks/flux.yml` + `.moon/tasks/loom.yml`, wire into pipeline | 4h |

## Priority P2: File Splits

| Bead | File | Lines | Split Target | Est. |
|------|------|-------|--------------|------|
| vb-27jox | output.rs | 280 | `output/{format,io,json,compat}.rs` | 2h |
| vb-jy6re | types.rs | 628 | `types/{header_payload,diagnostic,verify,explain,events,trace,replay_diff}.rs` | 4h |
| vb-e6xr7 | errors.rs | 738 | `errors/{core,collect,lifecycle,journal_replay}.rs` + stale ledger fix | 8h |
| vb-9zy8r | frame.rs | 1,254 | `frame/{state,transitions,frame_struct,kani_harnesses}.rs` + kani-frame feature | 12h |
| vb-p9owu | diagnostic.rs | 2,143 | `diagnostic/{codes,numeric,record}.rs` + CODE_REGISTRY constraint | 16h |
| vb-32gwc | budget.rs | 2,394 | `budget/{policy,compute,validation}.rs` + orphaned dir cleanup | 20h |

## Priority P2: Storage

| Bead | Action | Est. |
|------|--------|------|
| vb-strecov01 | Create 10 error_recovery tests for fuzz-malformed journal records | 8h |
| vb-stortst01 | Split 8,215-line tests.rs into ~30 files | 40h |

## Priority P2: Benchmarks + Tests

| Bead | Action | Est. |
|------|--------|------|
| vb-benchbn01 | Create warm_throughput.rs + digest_computation.rs benchmarks | 4h |
| vb-wstlsl01 | Delete self-laundering tests for 11 missing Section 17 codes | 2h |

---

## Total Estimated Hours

| Phase | Beads | Hours |
|-------|-------|-------|
| Phase 1 | vb-9ckqp, vb-f2xk3, vb-a408a | 1h + decision |
| Phase 2 | vb-81urc, vb-ybzsz, vb-dxi1k | 67h |
| Phase 3 | vb-27jox, vb-jy6re, vb-e6xr7 | 14h |
| Phase 4 | vb-9zy8r, vb-p9owu, vb-32gwc | 48h |
| Phase 5 | vb-strecov01, vb-stortst01 | 48h |
| Phase 6 | vb-6xh6c, vb-benchbn01, vb-wstlsl01 | 26h |
| **TOTAL** | | **~204h + decision** |

---

## Files to Modify Summary

| Category | Count | Files |
|----------|-------|-------|
| New benchmark files | 2 | `benches/warm_throughput.rs`, `benches/digest_computation.rs` |
| New moon task files | 2 | `.moon/tasks/flux.yml`, `.moon/tasks/loom.yml` |
| New test files | 1+ | `recovery/tests/error_recovery_tests.rs` + 10 tests |
| Split file targets | 6 | errors.rs, frame.rs, diagnostic.rs, budget.rs, tests.rs, types.rs |
| Module directory targets | 3 | output/, tests/, budget/ |
| Config edits | 3 | `.moon.yml` (pipeline), `Cargo.toml` (benches + deps), `source-length-exceptions.txt` |
| Script modifications | 2 | `check-test-determinism.py` |

---

## Acceptance Gates Summary

| Gate | Command |
|------|---------|
| Build | `cargo build --workspace` |
| Tests | `cargo test --workspace` |
| Lint | `moon :lint-src` |
| Source length | `moon :source-length` |
| Moon CI | `moon ci` |
| Kani validate | `moon run :verify-kani-vb-validate` |
| Bench compile | `cargo bench --no-run` |
