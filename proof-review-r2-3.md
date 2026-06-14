# Proof Review R2-3: Adversarial Findings

**Reviewer:** PROOF REVIEWER #3 — ROUND 2
**Date:** 2026-06-14
**Target:** `/home/lewis/src/velvet-ballistics`

---

## Summary Verdict: **REJECTED**

### BLOCKER COUNT: 3
### HIGH SEVERITY: 2
### MEDIUM SEVERITY: 2

---

## Required Scan Results

### Kani Quality

| Metric | Count | Threshold | Verdict |
|--------|-------|-----------|---------|
| `#[kani::unwind(1-3)]` shallow bounds | **8** | 0 | **BLOCKER** |
| `panic!` in `#[cfg(kani)]` files | **0** | 0 | PASS |
| `.expect(` in `#[cfg(kani)]` files | **0** | 0 | PASS |
| `.unwrap(` in `#[cfg(kani)]` files | **0** | 0 | PASS |

### Verus Integrity

| Metric | Count | Threshold | Verdict |
|--------|-------|-----------|---------|
| `#[verifier::external_body]` | **57** | documented | — |
| `ensures true` (non-external_body) | **2** | ≤ 2 | PASS |

### Trusted-Boundary Check

| Metric | Count | Threshold | Verdict |
|--------|-------|-----------|---------|
| Unledgered `external_body` files | **0** | 0 | PASS |
| Ledger stale entries | **2** | 0 | **HIGH** |

### TLA+ Infrastructure

| Metric | Count | Threshold | Verdict |
|--------|-------|-----------|---------|
| Configs missing `CHECK_DEADLOCK TRUE` | **0/49** | 0 | PASS |

### Anti-Laundering

| Metric | Result | Verdict |
|--------|--------|---------|
| `scripts/anti-verification-laundering.sh` | PASS (no blocking) | PASS |

### Test Suite

| Metric | Result | Verdict |
|--------|--------|---------|
| `cargo test -p vb_core -p vb_compile` | **3587 passed, 6 ignored** | PASS |

---

## BLOCKER 1: Kani GOD RULE 1 — Hardcoded WorkflowParts Shapes

**Severity: BLOCKER**
**Files:**
- `verification/kani/vb_xi2f_error_variants.rs` — **6 hardcoded** `WorkflowParts { }` struct literals
- `verification/kani/collect_try_from_parts.rs` — **2 hardcoded** `WorkflowParts { }` struct literals
- `verification/kani/vb_xi2f_compile_source.rs` — **1 hardcoded** `WorkflowParts { }` struct literal (H2 harness, while H1 correctly uses `kani::any()`)
- `verification/kani/vb-fzgdn/PS-006-harness.rs` — **1 hardcoded** `WorkflowParts { }` in `make_wf_with_node` helper

**Total: 10 hardcoded structural shapes across 4 Kani harness files.**

**GOD RULE Violation:** "Kani verification harnesses MUST NOT hardcode structural inputs (like `WorkflowParts` or `RunFrame`) with fixed dummy data. You MUST implement and use `kani::Arbitrary` for core structures, or write safe, exhaustive generator harnesses using `kani::any()`."

**Evidence:**
```
$ rg -c 'WorkflowParts\s*\{' verification/kani/ --glob '*.rs'
verification/kani/vb_xi2f_error_variants.rs:6
verification/kani/vb_xi2f_compile_source.rs:1   # H2 harness only; H1 uses kani::any() correctly
verification/kani/collect_try_from_parts.rs:2
verification/kani/vb-fzgdn/PS-006-harness.rs:1
```

**Why this is a BLOCKER:**
Each hardcoded `WorkflowParts` tests exactly one input shape. Proving `try_from_parts` doesn't panic on one hardcoded data structure proves nothing about the general case. These harnesses would pass even if `try_from_parts` panicked on every other possible input. The `vb_xi2f_compile_source.rs` file is especially egregious — its header claims "GOD RULE 1: kani::any() generates bounded WorkflowParts — no hardcoded shapes" but then line 79 builds a literal.

**Fix:** Replace all `WorkflowParts { ... }` struct literals with `kani::any::<WorkflowParts>()` constrained by `kani::assume(...)` for the specific validation property under test.

---

## BLOCKER 2: Verus GOD RULE 2 — Zero Production Import Binding

**Severity: BLOCKER**
**Files:** 145 Verus files under `verification/verus/`

**Evidence:**
```
Total Verus files: 145
Files with real production imports (use vb_core|vb_runtime|vb_compile|vb_storage|vb_ipc): 0
Files with commented-out import suggestions: 7
```

**GOD RULE Violation:** "Verus `proof fn` and `spec fn` models MUST mathematically bind to the actual Rust implementations (`exec fn`) inside the production codebase."

**Why this is a BLOCKER:**
Zero out of 145 Verus proof files import a production Rust crate. Every Verus proof defines standalone `spec` mirrors of production types with no structural isomorphism proof. The 7 files that appear to reference production imports (`vb-h09wf/PS-*.rs`) only do so in *comments*. The proofs cannot prove anything about the actual Rust implementation because they never reference it.

The ledger acknowledges this at entry 5 (severity CRITICAL) but no remediation has been applied. The compensating evidence (Kani harnesses and proptest) does not close the GOD RULE 2 gap because Verus proofs and Kani proofs verify different properties through different mechanisms.

---

## BLOCKER 3: 8 Shallow `#[kani::unwind(1-3)]` Bounds

**Severity: BLOCKER** (per rule: any non-zero shallow count)

**Files:**

| File | Line | Harness | unwind |
|------|------|---------|--------|
| `verification/kani/vb_xi2f_error_variants.rs` | 23 | `kani_try_from_parts_empty_nodes` | 3 |
| `verification/kani/step_offset_overflow.rs` | 100 | `kani_step_offset_boundary_max` | 3 |
| `verification/kani/step_offset_overflow.rs` | 120 | `kani_step_offset_boundary_valid` | 3 |
| `verification/kani/error_parity_harness.rs` | 25 | `kani_error_parity` | 3 |
| `verification/kani/emit_single_body_set_empty.rs` | 53 | `kani_empty_vec_first` | 3 |
| `verification/kani/emit_single_body_set_all_calls.rs` | 63 | `kani_emit_single_body_set_all_empty` | 3 |
| `verification/kani/choose_branch_validation.rs` | 24 | `choose_empty_no_otherwise_error` | 3 |
| `verification/kani/choose_branch_validation.rs` | 46 | `choose_empty_with_otherwise_valid` | 3 |

**Note:** All 8 are on loop-free harnesses. The `unwind(3)` is Kani's minimum value and not actually restrictive for these particular harnesses. However, per the BLOCKER definition ("any non-zero shallow/panic/expect/unwrap"), 8 > 0 constitutes a BLOCKER.

---

## HIGH 1: Ledger Staleness — Multiple Entries Outdated

**Severity: HIGH**

**Evidence:**

1. **`kani_assume_false` count mismatch:**
   - Ledger claims: `~170 occurrences`
   - Actual: **230 occurrences** (+35%)
   - Ledger entry 57 is stale by ~60 entries.

2. **`flux_rs::trusted` count mismatch:**
   - Ledger claims: `41 markers`
   - Actual: **44 markers** (+7%)
   - Ledger entry 55 is stale by 3 entries.

3. **`kani_assume(false)` pattern description:**
   - Ledger claims: `Err(_) => { kani::assume(false); loop {} }`
   - Actual: Many use `kani::assume(false); return;` instead. The `loop {}` pattern is present in some files (kani_idempotency_gates.rs, kani_expr_bound.rs) but the `return;` pattern dominates (kani_admission.rs, frame.rs, kani_action_queue.rs, etc.).

**Why HIGH:** A stale ledger undermines the trust boundary audit trail. New trust markers added since the ledger was last updated are not tracked. The pattern description error is less severe than the count mismatches.

---

## HIGH 2: 57 `#[verifier::external_body]` — Enormous Trusted Surface

**Severity: HIGH**

**Evidence:**
```
$ rg -c '#\[verifier::external_body\]' verification/verus/ --glob '*.rs'
57 occurrences across ~35 unique files
```

All 57 are ledgered with compensating Kani cross-references. However, 57 external_body markers across an ecosystem of 145 Verus files represents ~39% of all Verus proof files relying on trust boundaries. This is an extraordinarily high ratio.

**Affected files include:**
- `vb-fzgdn/PS-*` — 18 markers across 10 proof files (entire TimerWheel proof suite)
- `vb_compile/*` — 6 markers in 5 proof files (compile pipeline proofs)
- `vb-h09wf/PS-*` — 7 markers in 6 digest-binding proof files
- `vb-vzcuf/PS-*` — 4 markers in 4 constraint-verification files
- `vb_ajc40_*` — 2 markers in slug/query decode proofs

The ledger correctly documents compensating Kani harnesses for each, but the sheer volume means the Verus proof suite has very little internal verification — it's essentially a specification layer validated by independent Kani tests.

---

## MEDIUM 1: Two `ensures true` Blocked Obligations

**Severity: MEDIUM**

**Files:**
- `verification/verus/vb_ajc40_compiled_slug_decode.rs:25`
- `verification/verus/vb_ajc40_compiled_query_decode.rs:25`

Both are on `#[verifier::external_body]` proof functions that are documented as blocked ("requires postcard/Serde wire-format model in Verus"). The ledger correctly documents them as `ensures_true_blocker` trust markers (entries 58–59).

**Status:** Properly ledgered, no proof closure action since last review. These are IOUs backed by fuzz targets, not Verus proof.

---

## MEDIUM 2: Orphaned Loom Model

**Severity: MEDIUM**

**File:** `crates/vb_storage/src/queue/loom_vb_mrwe_7.rs`

This Loom concurrency model (bead vb-mrwe.7) exists but is **not wired into any module**. The ledger acknowledges this at entry 56 (severity WARNING, status DOCUMENTED).

**Evidence:**
```
$ rg 'mod loom_vb_mrwe_7' crates/vb_storage/src/
# No results — not imported anywhere
```

This is an orphaned verification artifact — it will never be compiled or executed.

---

## Pass Verdicts

| Check | Result |
|-------|--------|
| Tests pass (`vb_core` + `vb_compile`) | ✓ 3587 passed |
| TLA+ `CHECK_DEADLOCK TRUE` | ✓ 49/49 configs compliant |
| Anti-verification laundering | ✓ No blocking detected |
| `panic!` in Kani harnesses | ✓ 0 occurrences |
| `.expect(` in Kani harnesses | ✓ 0 occurrences |
| `.unwrap(` in Kani harnesses | ✓ 0 occurrences |
| Unledgered `external_body` | ✓ 0 files |
| `ensures true` count ≤ 2 | ✓ exactly 2 |

---

## Raw Evidence References

```
Kani unwind(1-3):      8   (rg -c 'kani::unwind\([1-3]\)' ...)
Kani panic!:           0   (rg panic! | grep kani | grep -v bak | grep -v //!)
Kani expect:           0   (rg expect | grep kani | grep -v bak | grep -v //!)
Kani unwrap:           0   (rg unwrap | grep kani | grep -v bak | grep -v //!)
Verus external_body:  57   (rg 'verifier::external_body' verification/verus/)
Verus ensures true:    2   (rg 'ensures true' | grep -v external_body)
Flux trusted:         44   (rg 'flux_rs::trusted' crates/)
Kani assume(false):  230   (rg 'kani::assume\(false\)' crates/ verification/)
Verus production imports: 0/145
Hardcoded WorkflowParts: 10 (4 files)
TLA+ deadlock:        49/49
```

---

## Repair Guidance

### Immediate BLOCKER Resolution

1. **GOD RULE 1:** Refactor all 10 hardcoded `WorkflowParts` struct literals to use `kani::any::<WorkflowParts>()` with targeted `kani::assume` constraints. At minimum, the 6 harnesses in `vb_xi2f_error_variants.rs` and the 2 in `collect_try_from_parts.rs` must be converted.

2. **GOD RULE 2:** This requires a multi-bead effort. Minimum viable path:
   - Add `extern_spec` blocks binding Verus spec types to production types
   - Or restructure selected proof files as `exec fn` annotations on production code
   - Close the ledger entry 5 critical gap

### Next Session

- Update trusted-base-ledger.jsonl counts for `kani_assume_false` (170→230) and `flux_rs::trusted` (41→44)
- Wire or remove the orphaned loom model `loom_vb_mrwe_7.rs`
- Consider adding `unwind(4)` or removing explicit unwind bounds on loop-free harnesses (8 occurrences, LOW effort)

---

**STATUS: REJECTED**
