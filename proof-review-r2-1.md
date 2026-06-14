# Proof Review R2-1 — Adversarial Gate

**Reviewer:** PROOF REVIEWER #1 (Round 2)  
**Scope:** Full independent scan of `/home/lewis/src/velvet-ballistics`  
**Date:** 2026-06-14  
**Provenance:** Direct scans — no subagent proxies, no self-approval.

---

## Scan Command Results

Each scan was executed with `/usr/bin/rg` (ripgrep 15.1.0) directly to bypass RTK token rewriting. All counts are raw match counts against actual file content.

| # | Scan | Count | Threshold | Verdict |
|---|------|-------|-----------|---------|
| 1 | `#[kani::unwind([1-3])]` | **8** | > 0 = BLOCKER | **BLOCKER** |
| 2 | `panic!` in kani-context files | 0 | > 0 = BLOCKER | PASS |
| 3 | `.expect(` in kani-context files | 0 | > 0 = BLOCKER | PASS |
| 4 | `.unwrap(` in kani-context files | 0 | > 0 = BLOCKER | PASS |
| 5 | `kani::cover!(true` in code | 2 (comments only) | > 0 = BLOCKER | INFO (see below) |
| 6 | `assert!(true)` in kani-context | 0 | > 0 = BLOCKER | PASS |
| 7 | `ensures true` in Verus | **3** | > 2 = BLOCKER | **BLOCKER** |
| 8 | `#[verifier::external_body]` active | 33 | No explicit threshold | WARNING |
| 9 | TLA+ CHECK_DEADLOCK TRUE | 50/50 pass | Missing = BLOCKER | PASS |
| 10 | Anti-verification-laundering shield | PASS | Fail = BLOCKER | PASS |
| 11 | `cargo test -p vb_core -p vb_compile` | 22/22 pass | Fail = BLOCKER | PASS |

---

## BLOCKER FINDINGS

### FINDING-001 (Blocker): 8 shallow `#[kani::unwind(3)]` remain

Previous round claimed 74 shallow unwind(3) → FIXED. These 8 were not fixed.

**Occurrences:**

| File | Line | Unwind |
|------|------|--------|
| `verification/kani/vb_xi2f_error_variants.rs` | 23 | `#[kani::unwind(3)]` |
| `verification/kani/step_offset_overflow.rs` | 100 | `#[kani::unwind(3)]` |
| `verification/kani/step_offset_overflow.rs` | 120 | `#[kani::unwind(3)]` |
| `verification/kani/error_parity_harness.rs` | 25 | `#[kani::unwind(3)]` |
| `verification/kani/emit_single_body_set_empty.rs` | 53 | `#[kani::unwind(3)]` |
| `verification/kani/emit_single_body_set_all_calls.rs` | 63 | `#[kani::unwind(3)]` |
| `verification/kani/choose_branch_validation.rs` | 24 | `#[kani::unwind(3)]` |
| `verification/kani/choose_branch_validation.rs` | 46 | `#[kani::unwind(3)]` |

> **Total: 8 matches across 5 files.**

**Evidence refs:** Raw `/usr/bin/rg` output captured during scan.  
**Disposition:** `blocker`

**Required fix:** Each harness must either (a) raise unwind to a provably adequate bound, or (b) document with a justification comment why 3 iterations suffice. The previous round's fix did not cover these.

---

### FINDING-002 (Blocker): 3 `ensures true` in Verus (limit is 2)

The scan found 3 active `ensures true` clauses. The threshold is > 2 → BLOCKER.

**Occurrences:**

1. `verification/verus/vb_ajc40_compiled_slug_decode.rs:25` — `ensures true`  
   - Also `#[verifier::external_body]` at line 23  
   - Documented blocker placeholder: "requires postcard/Serde wire-format model in Verus"  
   - Cross-references libFuzzer at `fuzz/fuzz_targets/vb_ajc40_compiled_slug_decode.rs`  

2. `verification/verus/vb_ajc40_compiled_query_decode.rs:25` — `ensures true`  
   - Also `#[verifier::external_body]` at line 23  
   - Documented blocker placeholder: same pattern as #1  

3. `crates/vb_runtime/src/verification/verus/runtime_facade_typed_errors.rs:151` — `ensures true`  
   - Function: `theorem_runtime_error_exhaustive()`  
   - **NOT** protected by `#[verifier::external_body]` — this is an active vacuous ensures  
   - Also contains `assert(true) by (compute)` at line 155  
   - The function claims to prove structural exhaustiveness of RuntimeError variants but the `ensures true` and `assert(true)` make it vacuous  

**Evidence refs:** Raw file reads at specified paths.  
**Disposition:** `blocker`

**Required fix:**  
- Placeholder files #1 and #2 are already documented as blockers — must be capped at exactly 2 or resolved. The third occurrence (#3) either needs a meaningful `ensures` clause or must be documented/waived as a known gap.

---

## NON-BLOCKING FINDINGS

### FINDING-003 (Info): 2 `kani::cover!(true, ...)` — in comments only

Both occurrences are inside commented-out code blocks (lines 876 and 882 of `crates/vb_ipc/src/kani_flag_validation.rs`). They are placeholders for future integration testing, not compiled into any harness.

```
//     kani::cover!(true, "decode_reject: ReservedBitsSet path ...");
//     kani::cover!(true, "decode_reject: InvalidFlags path ...");
```

These are NOT compiled code. Not a blocker. Mark as `owner_approved_no_action`.

---

### FINDING-004 (Warning): 33 active `#[verifier::external_body]` annotations

The entire Verus trusted boundary comprises 33 functions whose behavior is trusted without Verus verification. This is a large trusted computing base for a formal verification effort. While not an explicit per-scan threshold, the volume of external_body markers means a significant portion of Verus proof obligations are unchecked by the verifier.

**Cross-reference requirement:** Each external_body must have a corresponding Kani harness or fuzz target documented in the comment. Multiple files already do this (e.g., `vb-fzgdn/PS-009-proof.rs` references Kani). An audit should verify every external_body has a compensating non-Verus verification artifact.

---

## SUMMARY

- **2 BLOCKERS**: 8 shallow unwind(3) remaining + 3 ensures true exceeding the 2-entity limit  
- **0 FAILED TESTS**: 22/22 pass  
- **0 MISSING TLA+ DEADLOCK CHECKS**: All 50 configs have `CHECK_DEADLOCK TRUE`  
- **0 SHIELD VIOLATIONS**: Anti-verification-laundering passes  

### Verdict: REJECTED

**STATUS: REJECTED**

---

## Repair Guidance (Informational)

### For FINDING-001 (unwind(3)):

```bash
# Files to fix:
# verification/kani/vb_xi2f_error_variants.rs
# verification/kani/step_offset_overflow.rs
# verification/kani/error_parity_harness.rs
# verification/kani/emit_single_body_set_empty.rs
# verification/kani/emit_single_body_set_all_calls.rs
# verification/kani/choose_branch_validation.rs
```

Each `#[kani::unwind(3)]` must be evaluated: does the harness loop ≤3 times, or does it need a higher bound? If the loop count is genuinely ≤3, add a `// kani-check: bound 3 is adequate because...` comment and it may be accepted. Otherwise raise the bound.

### For FINDING-002 (ensures true):

The `runtime_facade_typed_errors.rs` file at `crates/vb_runtime/src/verification/verus/runtime_facade_typed_errors.rs:151` must either:
1. Replace `ensures true` with a meaningful ensures clause about variant exhaustiveness
2. Or be converted to a documented external_body blocker placeholder with explicit waiver
