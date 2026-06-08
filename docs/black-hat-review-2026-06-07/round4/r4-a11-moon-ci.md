# Round 4 Agent A11 — Moon CI Pipeline Integrity (CRITICAL)

**Reviewer:** black-hat-reviewer · **STATUS: REJECTED — SHIP-BLOCKER · 88/100**

The `moon ci` pipeline is structurally unsound as a verification gate. A "GREEN" result from moon ci conveys "fmt, lint, cargo check, and nextest pass on the current tree" — nothing more.

## Per-Task Inventory (21 pipeline tasks)

| # | Task | What it does | Genuine or Smoke? |
|---|------|--------------|---------------------|
| 1 | `fmt` | `cargo fmt --all --check` | ✅ Genuine |
| 2 | `lint-src` | clippy `-D warnings` | ✅ Genuine (strict) |
| 3 | `check` | `cargo check --workspace --all-targets --all-features` | ✅ Genuine |
| 4 | `sanitizer-address-check` | ASan cargo test | ✅ Genuine but DEPENDS on `check` |
| 5 | `verify-kani` | 4 Kani harnesses in `vb_core` | 🟡 Partial |
| 6 | `nightly-feature-gate` | scan against allowlist | ✅ Genuine |
| 7 | `nightly-feature-cargo-probe` | **SCRIPT BODY IS `true`** | ❌ **Phantom** |
| 8 | `source-length` | ≤300 line gate | ❌ **RED**: 7 over-limit files |
| 9 | `supply-chain` | audit/deny/vet/geiger/machete | ✅ Genuine |
| 10 | `feature-powerset` | cargo hack check | ✅ Genuine |
| 11 | `hardened-build` | cargo build --profile hardened | ✅ Genuine |
| 12 | `test` | cargo nextest | ✅ Genuine |
| 13 | `doc-test` | cargo test --doc | ✅ Genuine |
| 14 | `doc` | cargo doc | ✅ Genuine |
| 15 | `mutants-smoke` | 1 function, 1 mutation | ❌ **Theater** (0.026% coverage) |
| 16 | `fuzz-smoke` | 5 targets × 1s | ❌ **Theater** (5.4% / 5s total) |
| 17 | `miri` | 3 test filters | ❌ **Theater** (0.14% coverage) |
| 18 | `verify-verus` | registry-driven | ✅ Genuine (but limited set) |
| 19 | `verify-tlc` | 2 root specs | ✅ Genuine (fail-closed) |
| 20 | `coverage` | 1 test | ❌ **Theater** |
| 21 | `bench-build` | 1 benchmark | 🟡 Partial |

## Excluded from pipeline (`runInCI: false`) — 15 tasks

| Task | Reason | Verdict |
|------|--------|---------|
| `test-determinism` | "Current tree has pre-existing findings" | 🚨 **CRITICAL** — hides 1,088 findings |
| `benchmark-regression-policy` | xtask excluded | 🟡 Legitimate |
| `benchmark-proof` | 180m criterion run | 🟡 Legitimate |
| `pgo-instrument-build` | PGO profile gen | 🟡 Legitimate |
| `pgo-optimized-build` | PGO profile use | 🟡 Legitimate |
| `maxperf` | release build | 🟡 Legitimate |
| `maxperf-native` | native CPU | 🟡 Legitimate |
| `verify-fast` | Kani gauntlet (4 harnesses) | 🟡 Legitimate |
| `verify-standard` | Kani gauntlet (7 harnesses) | 🟡 Legitimate |
| `verify-deep` | Kani gauntlet + dedup | 🟡 Legitimate |
| `verify-proof` | full gauntlet | 🟡 Legitimate |
| `verify-all` | wraps verify-proof | 🟡 Legitimate |
| `contracts` | xtask not in workspace | 🟡 Legitimate |
| `quick` | fmt+lint+check | 🟡 Legitimate |
| `verify-verus-all` | 180m | 🟡 Legitimate |

## P0 Bug Bead Status

| Bead | Title | Status | Effective State |
|------|-------|--------|-----------------|
| `vb-1ev82` | P0: restore vb_runtime runtime module | **● blocked** | Code fixed, bead not closed. cargo check green. |
| `vb-8o7p5` | P0: Kani dep graph blockers | **● blocked** | Real bug. Kani harness timeouts. |
| `vb-o5zb` | P0: reconcile taint step-state resource | **● blocked** | All 5 children closed. Parent should close. |
| `vb-yesh4` | P0: fuzz manifest cfg | **● blocked** | Doesn't reproduce in main. Stale claim. |

**3 of 4 P0s are effectively closed but never marked closed; 1 is a stale claim.**

## Smoke-Only Lane Analysis

### A. `miri` — 0.14% test coverage
- Total `#[test]` in workspace: 11,725
- Tests run under Miri in CI: 17 (3 filters: vb_core 1/2193, vb_expr 15/775, vb_compile 1/852)
- Coverage ratio: **0.14%**
- `MIRIFLAGS` includes `-Zmiri-disable-isolation` — disables a UB-relevant check

### B. `fuzz-smoke` — 5.4% target × 1 second
- Total fuzz `[[bin]]` targets: 93
- Targets run in CI: 5
- Time per target: `-max_total_time=1`
- Total fuzz time: **5 seconds across the entire suite**

### C. `mutants-smoke` — 0.026% function × 1 mutation
- Total `fn` definitions in `vb_core`: 3,773
- Functions mutated in CI: 1 — `is_supported_code` at `crates/vb_core/src/diagnostic.rs:2057-2059`
- Mutation ratio: **1/3,773 = 0.026% of vb_core functions**

### D. `coverage` — 1 test run
- Test filter: `-- action::tests::validate_action_outcome_failed_always_succeeds` (1 test)
- Output: `target/llvm-cov/lcov.info` is generated, but its coverage reflects 1 test, not the workspace

### E. `nightly-feature-cargo-probe` — no-op phantom
```yaml
nightly-feature-cargo-probe:
  script: |
    set -euo pipefail
    # check already runs the exact all-targets/all-features cargo probe.
    true
```

The script body is literally `true`. **The real nightly feature gate lives in `nightly-feature-gate`.**

### F. `banned-token-gates` — phantom task
```yaml
banned-token-gates:
  deps:
    - 'panic-surface'
    - 'workspace-assertions'
    - 'ignored-fallible-results'
```
**No `command:`. No `script:`. No `outputs:`.**

## Hidden 1,088 Findings

`test-determinism` is `runInCI: false` and hides **1,088** test-determinism findings:
- 256 `UncontrolledClock` (mostly `Instant::now()` in `crates/vb_runtime/tests/`)
- 784 `SharedTempState` (`tempdir()`, `TempDir`, `/tmp/`)
- 31 `UncontrolledRandom` (`rand::`, `fastrand::`)
- 15 `GlobalMutableState` (`static mut`, `Once`, `Cell/RefCell`)
- 2 `SleepAsSync` (`thread::sleep`)

**Total: 1,088 findings — the largest integrity debt in the pipeline.**

## Top 3 Worst Findings

1. **`test-determinism` is excluded from CI with 1,088 known findings.** The task literally exists, finds real test-reliability problems across the entire workspace, and is disabled with a comment "Current tree has pre-existing findings; run explicitly until clean."

2. **All five "verification" lanes are smoke-only theater.** Miri 0.14%, Mutants 0.026%, Fuzz 5s total, Coverage 1 test. The pipeline summary shows all five as "PASS" with the same visual weight as the real gates.

3. **4 P0 beads are stale claims that mask completed work.** `vb-1ev82` notes say "holzman-rust restored vb_runtime::runtime ... Focused checks PASS" but the bead is `status: blocked`. `vb-yesh4` claims depend on vb-1ev82 being unfixed but it is. `vb-o5zb` is an umbrella whose children are all closed, but the parent is still blocked.

## Required Repair Actions

1. **CRITICAL**: Move `test-determinism` to `runInCI: true` and triage the 1,088 findings.
2. **CRITICAL**: Either expand or relabel each smoke lane to make scope explicit.
3. **CRITICAL**: Close the 4 stale P0 bugs.
4. **HIGH**: Remove `nightly-feature-cargo-probe` from the pipeline.
5. **HIGH**: Either give `banned-token-gates` a real `command:`, or remove it.
6. **HIGH**: Either re-admit `xtask` to the workspace or carve out `cargo xtask` as separate.
7. **HIGH**: Fix the 7 over-limit files.
8. **MEDIUM**: Wire `flux-check-package.sh` and a `loom` task into `.moon/tasks/*.yml`.
9. **MEDIUM**: Add `verify-kani-vb-validate` to the `.moon.yml` pipeline.
10. **MEDIUM**: For each of the 15 `runInCI: false` tasks, either file a bead with owner and removal criterion, or re-admit to the pipeline.

## Verdict: SHIP-BLOCKER

The combination of test-determinism debt hidden from CI, smoke-lane theater across 5 of 21 pipeline tasks, and stale P0 status makes any claim of "CI is green" unreliable as a release criterion.
