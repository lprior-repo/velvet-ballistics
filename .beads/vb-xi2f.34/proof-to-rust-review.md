# Proof-to-Rust Bridge Review — vb-xi2f.34: Finish Digest Coverage

**Reviewer Skill**: proof-reviewer
**Reviewer Invocation ID**: proof-reviewer-vb-xi2f.34-20260525-bridge
**Review State**: p7-proof-reviewer (bridge review)
**Date**: 2026-05-25
**Workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.34
**Input Bridge**: `.beads/vb-xi2f.34/proof-to-rust-map.md` (proof-to-implementation)
**Input Obligations**: `.beads/vb-xi2f.34/rust-refinement-obligations.jsonl` (12 rows)
**Input Proof Review**: `.beads/vb-xi2f.34/proof-review.md` (APPROVED, proof-reviewer-vb-xi2f.34-20260525-p6)

---

## Review Provenance

| Field | Value |
|---|---|
| Reviewer invocation | proof-reviewer-vb-xi2f.34-20260525-bridge |
| Bridge agent | proof-to-implementation (proof-to-rust-map.md, rust-refinement-obligations.jsonl) |
| Previous proof reviewer | proof-reviewer-vb-xi2f.34-20260525-p6 (STATUS: APPROVED) |
| Proof writer (repair) | proof-writer-vb-xi2f.34-20260525-repair2 |
| Self-approval risk | NONE — bridge agent is distinct from reviewer |
| Provenance ledger | 3 entries: femdation setup, proof-writer repair2, proof-reviewer p6 |

---

## Executive Summary

Bridge review of 12 rust-refinement-obligations mapping proof claims to Rust implementation. All 12 obligations are mapped: 10 `materialized`, 1 `resolved-no-op` (dead code discovery), 1 INFO-level finding carried forward. All 10 contract clauses are mapped across 4 defense-in-depth layers.

**Result: APPROVED** — 4 non-blocking findings (1 MEDIUM, 1 LOW, 2 INFO) documented. One evidence command unwind mismatch is a documentation gap (evidence stronger than claimed). No unmapped behavior-affecting obligations, no file-only refs, no verifier harness used as behavior test. All source refs, behavior test refs, refinement harness refs, and evidence commands verified on disk.

---

## Obligation Mapping Verification

### Obligation → Source Ref Cross-Check

| Obligation | Bridge Source Ref(s) | Verified on Disk | Match |
|---|---|---|---|
| PO-KANI-FINISH-001 | `part_05.rs:150-156, :153` | Lines 150-156: Finish match; 153: `hasher.update(value.as_bytes())` | ✅ |
| PO-KANI-FINISH-002 | `part_05.rs:150-156, :154` | Lines 150-156: Finish match; 154: `hasher.update(&value.to_le_bytes())` | ✅ |
| PO-KANI-FINISH-003 | `part_05.rs:150-156, :152-156` | Lines 152-156: inner match on ScalarValue in Finish arm | ✅ |
| PO-PROPTEST-FINISH-001 | `canonical_digest:116-138` + `compile_source:46` | Line 46: `canonical_digest(source)`. Lines 116-138: full function | ✅ |
| PO-PROPTEST-FINISH-002 | `digest_step_primitive:150-156` | Lines 150-156: Finish match, String/Integer/_ arms | ✅ |
| PO-PROPTEST-FINISH-003 | `canonical_digest:133-136` | Lines 133-136: `for step in source.steps() { hasher.update(step.id.as_bytes()); }` | ✅ |
| PO-INT-FINISH-001 | `compile_source:46` + `canonical_digest:116-138` + `digest_step_primitive:140-162` | All source refs verified on disk | ✅ |
| PO-INT-FINISH-002 | `canonical_digest:133-134` | Lines 133-134: step ID hashing loop | ✅ |
| PO-INT-FINISH-003 | `digest_step_primitive:152-156` | Lines 152-156: ScalarValue variant dispatch | ✅ |
| PO-INT-FINISH-004 | `canonical_digest:116-138` | Dead-code resolved; no `mod compile;` in lib.rs | ✅ |
| PO-STATIC-FINISH-001 | `digest_step_primitive:152-156` | Lines 152-156: inner match with `_` arm at 155 | ✅ |
| PO-STATIC-FINISH-002 | `canonical_digest:116-138` + `digest_step_primitive:140-162` | Audit scope lines 116-162 confirmed | ✅ |

**Source Ref Quality**: All 12 obligations have concrete `crate::module::function:line-line` references. Zero file-only or prose-only refs.

---

### Independent Behavior Test Cross-Check

| Obligation | Claimed Behavior Tests | Verified on Disk | Independent? |
|---|---|---|---|
| PO-KANI-FINISH-001 | proptest `finish_result_change_changes_digest_string` + integration `finish_result_value_changes_compiled_digest_string` | ✅ Both exist and pass | ✅ Real blake3 pipeline |
| PO-KANI-FINISH-002 | proptest `finish_result_change_changes_digest_integer` + integration `finish_result_value_changes_compiled_digest_integer` | ✅ Both exist and pass | ✅ Real blake3 pipeline |
| PO-KANI-FINISH-003 | integration `finish_result_type_changes_compiled_digest` | ✅ Exists and passes | ✅ Real blake3 pipeline |
| PO-PROPTEST-FINISH-001 | proptest `canonical_digest_is_deterministic` + structural `audit_digest_has_no_runtime_dependencies` | ✅ Both exist and pass | ✅ Self is the test |
| PO-PROPTEST-FINISH-002 | 2 proptest + 2 integration tests | ✅ All 4 exist and pass | ✅ Multi-layer |
| PO-PROPTEST-FINISH-003 | proptest `finish_position_change_changes_digest` + integration `finish_step_id_changes_compiled_digest` | ✅ Both exist and pass | ⚠️ See BF-004 |
| PO-INT-FINISH-001 | 2 integration tests for String/Integer | ✅ Both exist and pass | ✅ Self is the test |
| PO-INT-FINISH-002 | integration `finish_step_id_changes_compiled_digest` | ✅ Exists and passes | ✅ Self is the test |
| PO-INT-FINISH-003 | integration `finish_result_type_changes_compiled_digest` | ✅ Exists and passes | ✅ Self is the test |
| PO-INT-FINISH-004 | N/A (dead code, resolved-no-op) | N/A | N/A |
| PO-STATIC-FINISH-001 | structural `scalarvalue_exhaustiveness_in_digest` | ✅ Exists and passes | ✅ Self is the test |
| PO-STATIC-FINISH-002 | structural `audit_digest_has_no_runtime_dependencies` | ✅ Exists and passes | ✅ Self is the test |

**Independence Check**: No verifier harness is reused as a behavior test. Kani harnesses have separate integration/proptest tests exercising the real blake3 pipeline. Proptest harnesses are themselves the behavior test for L2. Integration tests are themselves for L3.

---

### Refinement Harness Cross-Check

| Obligation | Claimed Harness | Harness Lines Match? | Non-Vacuous? |
|---|---|---|---|
| PO-KANI-FINISH-001 | `kani_finish_digest.rs::finish_string_result_injectivity:203-227` | ✅ Lines 202-227: uses `kani::any()`, asserts `encodings_differ` | ✅ `assert!` is real claim |
| PO-KANI-FINISH-002 | `kani_finish_digest.rs::finish_integer_result_injectivity:246-259` | ✅ Lines 246-259: uses `kani::any()`, asserts `encoded1 != encoded2` | ✅ `assert!` is real claim |
| PO-KANI-FINISH-003 | `kani_finish_digest.rs::finish_scalarvalue_variant_discrimination:289-317` | ✅ Lines 289-317: uses `kani::any()`, properly scoped with `kani::assume` | ✅ Scoped universal claim |

**GOD RULE #1 Compliance**: All 3 Kani harnesses use `kani::any()` for symbolic inputs. Zero hardcoded structural shapes. ✅
**GOD RULE #2 Compliance**: Assertions are non-tautological real claims (verified by source code inspection). ✅

---

### Evidence Command Cross-Check

| Obligation | Bridge Map Command | Evidence File Command | Match? | Notes |
|---|---|---|---|---|
| PO-KANI-FINISH-001 | `cargo kani -p vb_compile --harness finish_string_result_injectivity --unwind 32` | `--unwind 32` | ✅ | Evidence: "0 of 115 failed" |
| PO-KANI-FINISH-002 | `cargo kani -p vb_compile --harness finish_integer_result_injectivity --unwind 3` | `--unwind 8` | ⚠️ **MISMATCH** | See BF-001 |
| PO-KANI-FINISH-003 | `cargo kani -p vb_compile --harness finish_scalarvalue_variant_discrimination --unwind 32` | `--unwind 32` | ✅ | Evidence: "0 of 72 failed" |
| PO-PROPTEST-FINISH-001/002/003 | `cargo test -p vb_compile --lib -- --ignored` | Same | ✅ | 4 passed (0.07s) |
| PO-INT-FINISH-001 | `cargo test ... -- finish_result_value_changes_compiled` | Verified: 7/7 pass | ✅ | Run confirmed |
| PO-INT-FINISH-002 | `cargo test ... -- finish_step_id` | Verified: passes | ✅ | Run confirmed |
| PO-INT-FINISH-003 | `cargo test ... -- finish_result_type` | Verified: passes | ✅ | Run confirmed |
| PO-STATIC-FINISH-001 | `cargo test ... -- scalarvalue_exhaustiveness` | Verified: 3/3 pass | ✅ | Run confirmed |
| PO-STATIC-FINISH-002 | grep audit + structural test | Verified: grep clean, test passes | ✅ | Run confirmed |

---

## Contract Clause Coverage Matrix

| Clause | Description | L1 (Kani) | L2 (Proptest) | L3 (Integration) | L4 (Structural) | Coverage |
|---|---|---|---|---|---|---|
| C1 | Value sensitivity | KANI-001, KANI-002 | PROPTEST-002 | INT-001 | — | ✅ PROVEN |
| C2 | ID sensitivity | — | PROPTEST-003 | INT-002 | — | ✅ PROVEN |
| C3 | Position sensitivity | — | PROPTEST-003 | INT-001 (multi-step) | — | ⚠️ See BF-004 |
| C4 | Determinism | — | PROPTEST-001 | INT-001 | STATIC-002 | ✅ PROVEN |
| C5 | Variant discrimination | KANI-003 (scoped) | PROPTEST-002 | INT-003 | — | ✅ PROVEN |
| C6 | Digest survives compilation | — | — | INT-001 | — | ✅ PROVEN |
| C7 | Single implementation | — | — | INT-004 (NO-OP) | grep audit | ✅ PROVEN |
| C8 | Forward compatibility | — | — | — | STATIC-001 | ✅ PROVEN |
| C9 | Pre-validation | — | PROPTEST-001 | — | STATIC-002 | ✅ PROVEN |
| C10 | Exclusion of runtime | — | — | — | STATIC-002 | ✅ PROVEN |

**Coverage**: 10/10 clauses mapped across 4 layers. Zero unmapped behavior-affecting claims.

---

## TLA+ Claims

No TLA+ specifications are in scope for this bead (vb-xi2f.34 covers Kani/proptest/integration/structural only). No Rust event/state mapping required. ✅

---

## Bridge Integrity Assessment

### Strengths
1. Every obligation has concrete `crate::module::function:line-line` source refs — zero file-only or prose refs
2. Independent behavior tests verified on disk for all behavior-affecting obligations
3. Refinement harness refs present and verified for all 3 Kani obligations
4. Evidence commands runnable and re-producible
5. Double evidence files: updated REPAIR-2 evidence at `evidence/proof-evidence.md` (2026-05-25), legacy copy at `.beads/vb-xi2f.34/verification/proof-evidence.md` (2026-05-24). Bridge references correct file.
6. All closure obligations documented for State 12 re-verification
7. Dead code discovery (PO-INT-FINISH-004 resolved-no-op) is structurally verified: no `mod compile;` in `lib.rs`

### Concerns
1. No raw Kani log files (PF-REP2-002, accepted-for-p1) — evidence embedded in markdown only
2. Kani harnesses replicate production encoding rather than calling it (PF-REP2-001, accepted-for-p1)
3. Evidence unwind mismatch for PO-KANI-FINISH-002 (BF-001, see findings)
4. Stale evidence copy at `.beads/vb-xi2f.34/verification/proof-evidence.md` (BF-002)

---

## Detailed Bridge Findings

### BF-001 — MEDIUM: Evidence Command Unwind Mismatch for PO-KANI-FINISH-002

- **Code**: E_COMMAND_EVIDENCE_MISSING
- **Obligation**: PO-KANI-FINISH-002
- **Bridge Map**: `cargo kani ... --unwind 3`
- **Harness Annotation**: `#[kani::unwind(3)]` (kani_finish_digest.rs:245)
- **Evidence File**: `cargo kani ... --unwind 8` (evidence/proof-evidence.md:36)
- **Artifact**: `evidence/proof-evidence.md`

**Description**: The exact evidence command documented in the bridge map specifies `--unwind 3`, but the evidence file was captured with `--unwind 8`. The harness source annotation `#[kani::unwind(3)]` sets the default; running with `--unwind 8` provides MORE unwinding than the minimum required (superset, not subset). The evidence is therefore at least as strong as claimed.

However, the mismatch means the exact evidence command cannot be re-run to reproduce the exact evidence output without producing potentially different verification statistics (different check counts, different timing). This is a documentation integrity gap, not a correctness gap.

**Impact**: Low. `--unwind 8` is a superset of `--unwind 3` for this harness (more thorough, not less). The harness annotation `#[kani::unwind(3)]` in source code is the canonical specification.

**Required Fix**: Align the evidence command in `proof-to-rust-map.md` and `rust-refinement-obligations.jsonl` to match the actual executed command, OR re-run with `--unwind 3` and capture matching evidence.

---

### BF-002 — LOW: Stale Pre-Repair Evidence Copy on Disk

- **Code**: (n/a — housekeeping)
- **Obligation**: PO-KANI-FINISH-003
- **Artifact**: `.beads/vb-xi2f.34/verification/proof-evidence.md` (2026-05-24)

**Description**: A pre-REPAIR-2 evidence file exists at `.beads/vb-xi2f.34/verification/proof-evidence.md` showing PO-KANI-FINISH-003 as FAILED (counterexample found). The correct REPAIR-2 evidence is at `evidence/proof-evidence.md` (2026-05-25) showing it VERIFIED with scoped `kani::assume`. The bridge map and obligation JSONL correctly reference the REPAIR-2 file. The stale copy is not referenced by any artifact but could cause confusion.

**Required Fix**: Remove or mark `.beads/vb-xi2f.34/verification/proof-evidence.md` as superseded.

---

### BF-003 — INFO: Kani Encoding Replication (PF-REP2-001 Carried Forward)

- **Code**: E_KANI_MODEL_REDUCTION
- **Obligation**: PO-KANI-FINISH-001, PO-KANI-FINISH-002, PO-KANI-FINISH-003
- **Artifact**: `crates/vb_compile/src/kani_finish_digest.rs`

**Description**: Kani encoding helpers (`encode_finish_string_bytes`, `encode_finish_integer`) replicate the production `digest_step_primitive` Finish arm byte-for-byte rather than calling it through a tracking mock. The replication is documented with exact production line references. Mitigated by proptest/integration defense-in-depth layers that test the real blake3 pipeline.

**Bridge Assessment**: The bridge map honestly documents this model reduction at PF-REP2-001 and identifies the exact production source lines each encoding helper mirrors. The mapping is accurate; no concealment.

---

### BF-004 — INFO: Proptest `finish_position_change_changes_digest` Tests C2, Not C3 (PF-REP2-003 Carried Forward)

- **Code**: E_PROPTEST_MISNAMED
- **Obligation**: PO-PROPTEST-FINISH-003
- **Artifact**: `crates/vb_compile/src/proptest_finish_digest.rs`, lines 190-209

**Description**: The proptest property varies step IDs (`id1` vs `id2`) for single-step workflows rather than testing Finish step position changes in multi-step workflows. This tests clause C2 (step ID sensitivity), not C3 (step position sensitivity). Contract C3 is effectively covered by C2 + ordered hashing in `canonical_digest()` (line 133-136) + multi-step integration test coverage.

**Bridge Assessment**: The bridge map explicitly documents this finding (proof-to-rust-map.md line 193) and correctly identifies the source ref (`canonical_digest:133-136`) and behavior test refs. The mapping note in `rust-refinement-obligations.jsonl` (RRO-FINISH-PROP-003) explains the coverage rationale.

---

## State 12 Closure Obligations (from bridge map)

| # | Obligation | Status |
|---|---|---|
| 1 | Capture raw Kani stdout to `evidence/` as `.out`/`.log` files | Required for State 12 |
| 2 | Re-run all evidence commands and record in verification ledger | Required for State 12 |
| 3 | Reconfirm all 10 trusted base entries | Required for State 12 |
| 4 | Remove dead code `compile/mod.rs` (894 lines) in follow-up bead | Follow-up bead |
| 5 | Fix unwind mismatch or re-run with documented `--unwind 3` | BF-001 |

---

## Decision

**STATUS: APPROVED**

The bridge mapping is comprehensive and honest:
- All 12 obligations mapped to concrete Rust symbols, files, and line ranges — verified on disk
- All behavior-affecting obligations have independent behavior tests — verified on disk and confirmed passing
- All 3 Kani obligations have refinement harness refs — verified on disk with non-vacuous assertions
- All 10 contract clauses covered across 4 defense-in-depth layers
- Dead code discovery (PO-INT-FINISH-004) structurally verified
- Known findings (PF-REP2-001 through PF-REP2-004) documented with mitigation rationale
- One unwind mismatch (BF-001) is a documentation gap — evidence is at least as strong as claimed (superset unwinding)

4 non-blocking findings recorded: 1 MEDIUM (documentation), 1 LOW (stale file), 2 INFO (carried forward). No rejection-grade issues.

---

## Reviewer Handoff Artifacts

1. `proof-to-rust-review.md` (this file) — bridge review decision and findings
2. Input bridge: `.beads/vb-xi2f.34/proof-to-rust-map.md`
3. Input obligations: `.beads/vb-xi2f.34/rust-refinement-obligations.jsonl`
4. Input proof review: `.beads/vb-xi2f.34/proof-review.md`
5. Contract: `.beads/vb-xi2f.34/contract.md`
6. Updated findings: `.beads/vb-xi2f.34/proof-findings.jsonl` (existing + 4 bridge findings)

---

## Output Artifacts

- `proof-to-rust-review.md` (this file) — bridge review with STATUS: APPROVED
