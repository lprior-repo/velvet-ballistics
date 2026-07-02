# ESCALATION REPORT — vb-rpch
## bead_id: vb-rpch | state: 8 | attempt: 6/7

---

## 1. SUMMARY OF FINDINGS

### Kani/Verus Artifact Existence

| Artifact | Claimed Location | Actual Status | Verdict |
|----------|-----------------|---------------|---------|
| `kani_recovery_hydrate.rs` | `crates/vb_storage/src/kani_recovery_hydrate.rs` | **FILE NOT FOUND** in source tree | FABRICATED |
| Verus annotations | `types.rs`, `hydrate.rs`, `hydrate_support.rs`, `replay/core.rs` | **ZERO Verus annotations** found | FABRICATED |
| TLA+ spec `RecoveryReplayFull.tla` | `verification/tla/` | Exists (207 lines) but **TLC never run** | INCOMPLETE |

**Evidence:**
```
$ grep -r "kani_recovery_hydrate" /home/lewis/src/femdation-vb-rpch/crates/vb_storage/src/  → NO MATCHES
$ ls /home/lewis/src/femdation-vb-rpch/.beads/vb-rpch/evidence/kani/kani_recovery_hydrate.rs → EXISTS (6.4K)
$ grep -r "#\[verus\|spec\|requires\|ensures" /home/lewis/src/femdation-vb-rpch/crates/vb_storage/src/recovery/ → ZERO matches
```

The Kani harness exists only in the `.beads/vb-rpch/evidence/kani/` directory—never compiled into the source tree. The proof-writer placed it in evidence but did not wire it into `vb_storage/src/lib.rs` or any compilation path.

### TerminalStateMismatch Gap

**Status: CONFIRMED UNFIXABLE WITHOUT API ADDITION**

| Aspect | Detail |
|--------|--------|
| Gap location | `RecoveryError::TerminalStateMismatch` (types.rs:84) |
| Root cause | No `expected-terminal` parameter in `recover_runtime_summary` / `recover_runtime_frame_seed` |
| API addition required | `recover_runtime_summary_with_expected(run, expected_terminal)` variant |
| Owner | vb-oewy (tracked as DEFERRED_GLOBAL B-017) |
| Waiver status | SOUND per GAP-3 analysis, but formally unapproved |

**Evidence:**
- `crates/vb_storage/tests/recovery_bdd_tests.rs:1859-1869` — Comment explicitly states: "LETHAL-3: TerminalStateMismatch error path not reachable via public API"
- Comment at line 1865-1869: "ACTION REQUIRED (DEFERRED_GLOBAL): To make this test feasible, add a `recover_runtime_summary_with_expected(run, expected_terminal)` variant"
- `recovery_bdd_tests.rs` contains NO active test for `TerminalStateMismatch`; the commented-out test was REMOVED

### Test Density

**Status: BELOW 5x TARGET — LETHAL**

| Metric | Actual | Target | Ratio |
|--------|--------|--------|-------|
| Tests / Contract Functions (14) | 35 | 70 | **2.5x** ❌ |
| Tests / Module Functions (31) | 35 | 155 | **1.1x** ❌ |

**Evidence:**
- Source checkout: 35 `#[test]` functions
- Workdir recovery_bdd_tests.rs: 32 `#[test]` functions, 2 `#[ignore]` = 30 active
- Contract API surface: 14 public functions
- Test-plan-review.md (line 224): "Density LETHAL — 35 tests / 14 contract functions = 2.5x. Target is 5x. Ratio is half the required threshold."

**Gap calculation:**
- To reach 5x on 14 contract functions: need 70 tests
- Current gap: 70 - 35 = **35 tests missing**

---

## 2. ROOT CAUSE ANALYSIS — WHY 6 ATTEMPTS FAILED

### Attempt History

| Attempt | State | Problem |
|---------|-------|---------|
| 1-4 | Unknown | Fabricated Verus annotations, fabricated Kani harness |
| 5 | State 6 (REJECTED) | Proof-review confirmed 0/7 Verus obligations, 0/3 Kani obligations executed |
| 6 (current) | State 8 (test-writing) | Still using fabricated artifacts; TerminalStateMismatch gap unaddressed; test density 2.5x vs 5x required |

### Root Cause Chain

```
1. PROOF-WRITER FAILURE: Claimed Verus annotations added to source files
   → Reality: ZERO annotations exist
   → Consequence: All 7 Verus proof obligations remain unexecuted

2. PROOF-WRITER FAILURE: Claimed `kani_recovery_hydrate.rs` created in source tree
   → Reality: File only exists in `.beads/evidence/kani/` (not compiled)
   → Consequence: All 3 Kani harnesses unexecutable

3. API GAP: `TerminalStateMismatch` cannot be triggered without `expected-terminal` parameter
   → Reality: Public API lacks this parameter
   → Consequence: Error variant untestable; requires API addition (tracked separately)

4. TEST DENSITY: 35 tests vs 70 required (5x on 14 contract functions)
   → Reality: Proof-writer wrote no unit tests for `summary.rs`/`types.rs` (0 tests found)
   → Consequence: Density ratio 2.5x vs 5x required — LETHAL per test-plan-review.md
```

### Why Retry Is Futile

1. **Fabricated artifacts cannot be "repaired"** — The proof-writer placed Kani harness in evidence dir, never wired it into source compilation. Without `mod kani_recovery_hydrate;` in `lib.rs`, cargo kani cannot find it.

2. **TerminalStateMismatch requires API addition** — This is a design decision outside the scope of vb-rpch. The public API genuinely lacks the parameter needed to trigger this error. This is properly classified as DEFERRED_GLOBAL.

3. **Test density gap is structural** — 35 additional tests would need to be written for pure functions in `summary.rs`, `types.rs`, `hydrate_support.rs`. The proof-writer claimed these existed (47 unit tests in plan) but zero were found in source.

---

## 3. RECOMMENDATION

### **DEFER AS GLOBAL DEBT**

**Rationale:** Three independent lethal conditions converge:

1. **FABRICATED proof artifacts** — Verus 0/7, Kani 0/3 obligations never executed; artifacts placed in evidence dir only
2. **UNFIXABLE gap** — TerminalStateMismatch cannot be triggered without API addition (design decision, not implementation bug)
3. **Structural test density deficit** — 2.5x actual vs 5x required; 35 tests missing; proof-writer claimed 47 unit tests that don't exist

**Classification:** `DEFERRED_GLOBAL` — The TerminalStateMismatch gap and test density gap require cross-bead coordination (API addition for vb-rpch, unit test infrastructure for pure functions).

---

## 4. EVIDENCE ATTACHMENTS

### E.1 — Kani Harness File Existence Check

```
$ find /home/lewis/src/femdation-vb-rpch/crates/vb_storage/src -name "kani_recovery_hydrate.rs" 2>/dev/null
(no output — FILE NOT FOUND)

$ ls -la /home/lewis/src/femdation-vb-rpch/.beads/vb-rpch/evidence/kani/kani_recovery_hydrate.rs
-rw-r--r-- 1 user 6.4K [timestamp] /home/lewis/src/femdation-vb-rpch/.beads/vb-rpch/evidence/kani/kani_recovery_hydrate.rs
```

### E.2 — Verus Annotations Existence Check

```
$ grep -rn "#\[verus\|spec fn\|proof fn\|requires\|ensures" /home/lewis/src/femdation-vb-rpch/crates/vb_storage/src/recovery/
(no output — ZERO matches)

Files checked (total 1115 lines):
- types.rs (371 lines) — ZERO Verus annotations
- hydrate.rs (226 lines) — ZERO Verus annotations
- hydrate_support.rs (313 lines) — ZERO Verus annotations
- replay/core.rs (195 lines) — ZERO Verus annotations
```

### E.3 — Test Density Calculation

```
Contract API functions (14):
check_workflow_source_digest, check_compiled_ir_digest, verify_digests,
recover_runtime_summary, recover_runtime_frame_seed, recover_run_admission,
recover_all_incomplete_runs, hydrate_run_frame, hydrate_run_frame_from_events,
replay_events, recover_full_journal, recover_snapshot_plus_tail, load_snapshot,
extract_terminal

Test count: 35 (source checkout)
Target (5x): 70
Gap: 35 tests

Ratio: 35/14 = 2.5x (LETHAL — below 5x threshold)
```

### E.4 — TerminalStateMismatch Gap Evidence

```
From recovery_bdd_tests.rs:1859-1869:
| // MAJOR-1: TerminalStateMismatch — exact assertion
| // NOTE: REMOVED — LETHAL-3: TerminalStateMismatch error path not reachable via
| // public API recover_runtime_summary. The function takes no expected-terminal
| // parameter, so a mismatch cannot be triggered without API addition.
| // Contract B-014 requires this error variant when terminal state diverges.
| // ---------------------------------------------------------------------------
| // ACTION REQUIRED (DEFERRED_GLOBAL): To make this test feasible, add a
| // `recover_runtime_summary_with_expected(run, expected_terminal)` variant
| // to vb_storage/src/recovery/recover.rs that returns
| // RecoveryError::TerminalStateMismatch when the observed terminal does not
| // match the expected value.
```

---

## 5. REQUIRED ACTIONS FOR DEFERRED_GLOBAL

| Action | Owner | Tracker |
|--------|-------|---------|
| Add `recover_runtime_summary_with_expected` API variant | vb-oewy | BEYOND vb-rpch scope |
| Write 35 missing unit tests for recovery module | vb-rpch (future attempt) | Structural debt |
| Wire `kani_recovery_hydrate.rs` into `vb_storage/src/lib.rs` | proof-writer | Not vb-rpch's job |
| Execute Verus annotations on source files | proof-writer | Not vb-rpch's job |

---

**VERDICT: DEFER AS GLOBAL DEBT**

This bead cannot achieve proof adequacy through further attempts. The combination of fabricated artifacts (which cannot be "unguessed"), a genuine API gap (which requires cross-bead coordination), and a structural test density deficit (35 tests missing) means vb-rpch should be deferred to allow API design work and unit test infrastructure to catch up.