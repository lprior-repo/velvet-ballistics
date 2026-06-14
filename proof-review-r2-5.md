# Proof Review R2-5 — Full Audit Report

**Reviewer:** PROOF REVIEWER #5 (ROUND 2)
**Date:** 2026-06-14
**Workspace:** `/home/lewis/src/velvet-ballistics`
**Artifact:** `proof-review-r2-5.md`

---

## Provenance

- All commands executed live in workspace shell.
- Raw command evidence captured below.
- No self-approval: independent reviewer.

---

## Metric Matrix

### 1. Anti-Laundering Shield

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| EXIT code | 0 | 0 | ✅ |
| "No blocking" present | Yes | Yes | ✅ |
| Blocking laundering detected | No | No | ✅ |

**Command:** `bash scripts/anti-verification-laundering.sh 2>&1`
**Output:** `⚠️  No blocking verification laundering detected. Existing warning-class debt is delegated to registry gates.`

**NOTE:** Warning-class debt exists (Kani `assume(false)` paths, test silent early returns) but is delegated to registry gates. Non-blocking.

---

### 2. Kani Health — ALL Metrics

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| `panic!` in kani files | 0 | **0** | ✅ |
| `.expect(` in kani files | 0 | **0** | ✅ |
| `.unwrap(` in kani files | 0 | **0** | ✅ |
| `kani::cover!(true)` active in kani files | 0 | **0** | ✅ *[1]* |
| `assert!(true)` in kani files | 0 | **0** | ✅ |
| `#[kani::unwind(1-3)]` in kani files | 0 | **8** | ❌ **VIOLATION** |

**Footnotes:**
- *[1]* Two `kani::cover!(true)` instances exist in `crates/vb_ipc/src/kani_flag_validation.rs:876,882` but **both are commented out** (`//` prefix). Active count is 0.

**Shallow unwind (1-3) detail — 8 instances, all in `verification/kani/`:**

| File | Line | Unwind |
|------|------|--------|
| `verification/kani/choose_branch_validation.rs` | 24 | `#[kani::unwind(3)]` |
| `verification/kani/choose_branch_validation.rs` | 46 | `#[kani::unwind(3)]` |
| `verification/kani/vb_xi2f_error_variants.rs` | 23 | `#[kani::unwind(3)]` |
| `verification/kani/step_offset_overflow.rs` | 100 | `#[kani::unwind(3)]` |
| `verification/kani/step_offset_overflow.rs` | 120 | `#[kani::unwind(3)]` |
| `verification/kani/error_parity_harness.rs` | 25 | `#[kani::unwind(3)]` |
| `verification/kani/emit_single_body_set_empty.rs` | 53 | `#[kani::unwind(3)]` |
| `verification/kani/emit_single_body_set_all_calls.rs` | 63 | `#[kani::unwind(3)]` |

**Location:** All 8 are in the standalone `verification/kani/` crate (not in production `crates/*/`). None in production crates. Still counts as "kani files" per the rule.

---

### 3. Verus Integrity

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| `#[verifier::external_body]` | ≤ 2 (documented) | **78** instances, **27** files | ❌ **VIOLATION** |
| `ensure true` (non-external_body) | 0 | **0** ✅ [*2]* | ✅ |
| `assume(` in Verus | 0 | **0** | ✅ |
| `axiom` in Verus | 0 | **0** | ✅ |

**Footnotes:**
- *[2]* Two `ensures true` instances exist (`verification/verus/vb_ajc40_compiled_slug_decode.rs:25` and `verification/verus/vb_ajc40_compiled_query_decode.rs:25`) but both are immediately preceded by `#[verifier::external_body]` on the same function. Per rule, `external_body` doesn't count toward this metric. Effective active count: **0**.

**`external_body` detail:**

- **78 total instances** across **27 files** — 39× the limit of 2.
- **Ledger documentation:** 31 entries in `verification/trusted-base-ledger.jsonl` document external_body trust boundaries, covering **25 of 27 files**.
- **Undocumented files** (2, with 2 total instances):
  - `verification/verus/vb_ajc40_compiled_slug_decode.rs` — 1 instance
  - `verification/verus/vb_ajc40_compiled_query_decode.rs` — 1 instance
- These 2 undocumented files have a compensating fuzz target reference inline but no `trust_marker` ledger entry.

**Files with external_body (instances per file):**

| File | Count |
|------|-------|
| `vb_ajc40_compiled_slug_decode.rs` | 1 |
| `vb_ajc40_compiled_query_decode.rs` | 1 |
| `vb-vzcuf-PS-009.rs` | 2 |
| `vb-vzcuf-PS-008.rs` | 2 |
| `vb-vzcuf-PS-004.rs` | 2 |
| `vb-vzcuf-PS-003.rs` | 2 |
| `vb-h09wf/PS-011-digest-triangle.rs` | 2 |
| `vb-h09wf/PS-010-policy-digest.rs` | 2 |
| `vb-h09wf/PS-007-verification-digest-match.rs` | 2 |
| `vb-h09wf/PS-006-artifact-digest-match.rs` | 2 |
| `vb-h09wf/PS-002-anti-contract.rs` | 2 |
| `vb-h09wf/PS-001-digest-binding.rs` | 2 |
| `vb-fzgdn/PS-010-proof.rs` | 3 |
| `vb_compile/width_parity_proof.rs` | 3 |
| `vb-fzgdn/PS-009-proof.rs` | 3 |
| `vb_compile/recursive_lowering_proof.rs` | 2 |
| `vb-fzgdn/PS-008-proof.rs` | 4 |
| `vb_compile/emit_order_proof.rs` | 2 |
| `vb-fzgdn/PS-007-proof.rs` | 4 |
| `vb_compile/body_step_width_proof.rs` | 3 |
| `vb-fzgdn/PS-003-proof.rs` | 4 |
| `vb-fzgdn/PS-006-proof.rs` | 10 |
| `vb_compile/body_dispatcher_proof.rs` | 3 |
| `vb-fzgdn/PS-002-proof.rs` | 4 |
| `vb-fzgdn/PS-005-proof.rs` | 4 |
| `vb-fzgdn/PS-001-proof.rs` | 4 |
| `vb-fzgdn/PS-004-proof.rs` | 3 |
| **TOTAL** | **78** |

---

### 4. TLA+ Completeness

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| `CHECK_DEADLOCK TRUE` present | All cfg files | **49/49** files | ✅ |
| Missing `CHECK_DEADLOCK` | 0 | **0** | ✅ |

---

### 5. Trusted-Base Ledger

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| Ledger entries | ≥ 60 | **67** | ✅ |
| External_body documented entries | — | 31 | ⚠️ (see Verus section) |

---

### 6. Test & Build Results

| Suite | Expected | Actual | Status |
|-------|----------|--------|--------|
| `cargo test -p vb_core` | All pass | **2631 passed** (52 suites, 1.37s) | ✅ |
| `cargo test -p vb_compile` | All pass | **956 passed, 6 ignored** (40 suites, 8.64s) | ✅ |
| `cargo check` | Compiles clean | **11 crates compiled, `Finished dev`** | ✅ |

---

## Findings

### FINDING-001 (BLOCKER) — Shallow Kani unwinds exceed limit

- **Obligation:** Kani proof harnesses: `#[kani::unwind(1-3)]` = 0
- **Severity:** BLOCKER
- **Location:** `verification/kani/` — 8 instances of `#[kani::unwind(3)]`
- **Evidence:** `/usr/bin/rg -n '#\[kani::unwind\([1-3]\)\]' verification/kani/ -g '*.rs'`
- **Required fix:** Remove `#[kani::unwind(3)]` annotations or replace with `#[kani::unwind(4+)]` if loops require deeper unwinding; or justify each `unwind(3)` with a documented rationale in the ledger.
- **Disposition:** `blocker`

### FINDING-002 (BLOCKER) — Excessive external_body markers in Verus

- **Obligation:** `#[verifier::external_body]` ≤ 2, both documented
- **Severity:** BLOCKER
- **Location:** `verification/verus/` — 78 instances across 27 files
- **Evidence:** `/usr/bin/rg -c '#\[verifier::external_body' verification/verus/ -g '*.rs'`
- **Required fix:** Either:
  a. Reduce external_body count to ≤ 2 by replacing with full Verus proofs, OR
  b. Obtain documented waiver for expanded trusted boundary with compensating Kani/Fuzz evidence ledgered for all 78 instances (currently only 31 have ledger entries; 47 undocumented).
- **Disposition:** `blocker`

### FINDING-003 (MINOR) — Comented-out `kani::cover!(true)` residue

- **Obligation:** `kani::cover!(true)` = 0
- **Severity:** MINOR (non-blocking — code is commented out)
- **Location:** `crates/vb_ipc/src/kani_flag_validation.rs:876,882`
- **Evidence:** Two lines of `// kani::cover!(true, "...")` are commented out
- **Required fix:** Remove commented-out code or uncomment with proper cover predicates.
- **Disposition:** `owner_approvable_debt`

### FINDING-004 (MINOR) — Two Verus files missing ledger entries

- **Obligation:** All trust markers must have `trusted-base-ledger/v1` rows
- **Severity:** MINOR
- **Location:** `verification/verus/vb_ajc40_compiled_slug_decode.rs`, `verification/verus/vb_ajc40_compiled_query_decode.rs`
- **Evidence:** `/usr/bin/rg 'external_body' verification/trusted-base-ledger.jsonl | /usr/bin/rg -o '"file":"[^"]*"' | sort -u` — 25 files covered, 2 missing
- **Required fix:** Add ledger entries for these 2 files with compensating evidence (fuzz targets referenced inline).
- **Disposition:** `owner_approvable_debt`

---

## Summary

| # | Check | Expected | Actual | Status |
|---|-------|----------|--------|--------|
| 1 | Anti-laundering shield | EXIT 0, "No blocking" | EXIT 0, "No blocking" | ✅ PASS |
| 2 | `panic!` in kani files | 0 | 0 | ✅ PASS |
| 3 | `.expect(` in kani files | 0 | 0 | ✅ PASS |
| 4 | `.unwrap(` in kani files | 0 | 0 | ✅ PASS |
| 5 | `kani::cover!(true)` active | 0 | 0 | ✅ PASS |
| 6 | `assert!(true)` in kani files | 0 | 0 | ✅ PASS |
| 7 | `#[kani::unwind(1-3)]` in kani files | 0 | **8** | ❌ **FAIL** |
| 8 | `#[verifier::external_body]` in Verus | ≤ 2 documented | **78** (31 documented) | ❌ **FAIL** |
| 9 | `ensures true` (non-external_body) | 0 | 0 | ✅ PASS |
| 10 | `assume(` in Verus | 0 | 0 | ✅ PASS |
| 11 | `axiom` in Verus | 0 | 0 | ✅ PASS |
| 12 | TLA+ missing CHECK_DEADLOCK | 0 | 0 | ✅ PASS |
| 13 | Test: `vb_core` | All pass | 2631 passed | ✅ PASS |
| 14 | Test: `vb_compile` | All pass | 956 passed, 6 ignored | ✅ PASS |
| 15 | `cargo check` | Compiles | Compiles clean | ✅ PASS |
| 16 | Ledger entries | ≥ 60 | 67 | ✅ PASS |

**PASS:** 14 / 16  
**FAIL:** 2 / 16 (both BLOCKER)

---

## Verdict

**STATUS: REJECTED**

Two BLOCKER findings prevent approval:

1. **FINDING-001**: 8 instances of `#[kani::unwind(3)]` in `verification/kani/` violate the zero-tolerance shallow-unwind rule. While all are `unwind(3)` (not 1 or 2) and in the standalone verification crate, the rule does not exempt any kani files.

2. **FINDING-002**: 78 instances of `#[verifier::external_body]` across 27 Verus files exceed the maximum of 2. Only 31 of 78 instances have ledger documentation. The trusted-boundary surface has expanded well beyond the approved limit.

**Resolution path:**
- For FINDING-001: Either remove explicit `#[kani::unwind(3)]` annotations (rely on default unwind) or document each with an approved waiver in the ledger.
- For FINDING-002: Either reduce external_body count to ≤ 2 with full Verus proof replacements, or obtain explicit owner-approved documentation for the expanded trusted boundary covering all 78 instances with compensating evidence in the ledger.
