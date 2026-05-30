# Proof Evidence: vb-8mdp.7 State 5

## Executive Summary
Proof writing executed per approved plan. TLA+ (PASS) and Verus (PASS — but VACUUM) evidence collected. Kani harnesses REPAIRED for GOD RULE compliance. All Rust-based verifications BLOCKED by production compilation failure in `vb_core`.

## Evidence Inventory

### TLC Model Checking
- **File**: `.beads/vb-8mdp.7/evidence/tlc-collect-body-model.log`
- **Command**: `java -XX:+UseParallelGC -Xmx2G -cp tla2tools.jar tlc2.TLC verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg`
- **Exit code**: 0
- **Output summary**: 
  - 20 states generated, 20 distinct, 0 left on queue
  - Depth: 5
  - 6 invariants checked: NodeCountInvariant, OffsetInvariant, NodeKindInvariant, NoOverflowInvariant, TypeOK, LoweringDeterminism
  - No errors found
- **Obligations covered**: PO-001, PO-008, PO-008b, PO-012, PO-017
- **Model bounds**: MaxStepIdx=65535, current_id in 0..3, 4-node emission sequence

### Verus Verification
- **File**: `.beads/vb-8mdp.7/evidence/verus-collect-lowering.log`
- **Command**: `verus verification/verus/collect_lowering.rs --crate-type=lib`
- **Exit code**: 0
- **Output summary**: 6 verified, 0 errors
- **Lemmas verified**:
  - `lemma_lower_canonical_collect_step_offsets`: id+1, id+2, id+3 all ≤ u16::MAX
  - `lemma_lower_canonical_collect_emits_4_nodes`: offset computation produces (id+1, id+2, id+3)
  - `lemma_max_valid_collect_id`: max valid id = u16::MAX - 3
  - `lemma_source_slot_recorded`: max_slot ≥ source
  - `lemma_budget_defaults`: limit/pages_size unwrap_or(1) ≥ 1
- **Obligation covered**: PO-011
- **VACUUM PROOF ASSESSMENT**: These are purely mathematical lemmas about integers. No `requires`/`ensures` link to `lower_canonical_collect` in production code. GOD RULE 2 violation — documented, not waived.

### Production Source Verification (manual inspection)
- **File**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:263-299`
- **Status**: FIXED — `StepPrimitive::Collect` now has dedicated match arm hashing variable, source, pages, items, body
- **Fields hashed**: 
  - `variable` → `hasher.update(variable.as_bytes())`
  - `source` → `hasher.update(source.as_bytes())`  
  - `pages` → `hasher.update(b"some")` + `value.to_le_bytes()` or `hasher.update(b"none")`
  - `items` → same pattern as pages with `b"some"`/`b"none"` discriminators
  - `body` → recursive `digest_sub_step()` for each child step

### Kani Harness Repairs (artifacts only — not executed)
- **collect_field_coverage.rs**: Rewritten with 5 GOD RULE-compliant harnesses + meta-harness
- **foreach_field_coverage.rs**: Rewritten — removed local function copies, now calls production `digest_step_primitive`
- **aggregate_field_coverage.rs**: Rewritten — removed local function copies, now calls production `digest_step_primitive`

### Unit Tests (exist but cannot compile)
- **File**: `crates/vb_compile/src/mod_compile_lowering/tests.rs`
- **Tests**: `digest_collect_variable_field`, `digest_collect_source_field`, `digest_collect_pages_field`, `digest_collect_items_field`, `digest_collect_body_recursive`, `digest_collect_pages_none_vs_some`, `digest_collect_items_none_vs_some`, `collect_digest_equality_property`, `digest_collect_repeated_calls_same_digest`
- **Blocked by**: `vb_core/src/diagnostic.rs:1561` — `const_cmp` feature not enabled

### Integration Test (exists but cannot compile)
- **File**: `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs`
- **Test**: `test_admission_rejects_when_ir_digest_mismatches_artifact`
- **Blocked by**: Same compilation failure chain

## Tooling Availability

| Tool | Installed | Version |
|------|-----------|---------|
| TLC | Yes | 2.19 (2024-08-08) |
| Verus | Yes | 0.2026.05.05.d03e906 |
| Java | Yes | 26.0.1 |
| Kani | **NO** | — |
| Cargo | Yes | 1.97.0-nightly (2026-04-27) |

## Blocker Evidence

### BLOCKER: Production Compilation Failure
```
error: `PartialEq` is not yet stable as a const trait
    --> crates/vb_core/src/diagnostic.rs:1561:12
     |
1561 |         if CODE_REGISTRY[i].symbolic == symbolic {
     |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: add `#![feature(const_cmp)]` to the crate attributes to enable
```
- Impact: All Rust tests, Kani harnesses, and proptest properties cannot compile
- Root cause: `const fn symbolic_to_numeric` uses `==` on `&str` which requires nightly `const_cmp`

### BLOCKER: Kani Not Installed
```
$ which kani
kani not found
```
- Impact: All 7 Kani harnesses (PO-002, PO-013, PO-015, PO-016, PO-020) cannot be executed

---
*Proof evidence. State 5. Bead vb-8mdp.7. 2026-05-29.*
