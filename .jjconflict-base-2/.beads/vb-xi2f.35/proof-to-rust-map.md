# Proof-to-Rust Map: ResourceContract Digest Coverage

## Bridge Metadata

| Field | Value |
|-------|-------|
| **bridge_skill** | `proof-to-implementation` |
| **bead_id** | `vb-xi2f.35` |
| **bead_title** | P1: digest covers resource contract semantics |
| **workspace** | `/home/lewis/src/vb-workspaces/vb-xi2f.35` |
| **proof_review_status** | CONDITIONALLY APPROVED (R5) |
| **bridge_date** | 2026-05-26T23:00:00Z (original); repaired 2026-05-26T03:00:00Z (PF-BR-001, PF-BR-002 fixes) |
| **input_artifact** | `.beads/vb-xi2f.35/proof-review.md` (R5) |
| **schema** | `proof-to-rust-map/v1` |

## Executive Summary

All 26 proof obligations (14 Kani + 4 Verus + 7 proptest + 1 fuzz) mapped to concrete Rust source refs. The canonical `ResourceContract` type resides in `crates/vb_core/src/workflow/mod.rs:191` with 17 fields. A stale 16-field duplicate persists in `crates/vb_core/src/compiled_workflow.rs:130`. Two `canonical_digest` implementations exist in `part_05.rs:116` and `compile/mod.rs:221`, both calling the shared `contract_encoding::encode_contract_bytes` at `crates/vb_core/src/contract_encoding.rs:27`.

**Critical gap**: `crates/vb_core/src/validation/resource.rs:12` imports the 16-field duplicate (`use crate::compiled_workflow::ResourceContract`) instead of the 17-field canonical type. This blocks verification of PO-K11 (17-field validation).

## Obligation-to-Rust Mapping

### PO-K01: Digest Determinism

| Aspect | Concrete Ref |
|--------|-------------|
| **Production function** | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116` — `pub(crate) fn canonical_digest(source, contract)` |
| **Duplicate path** | `crates/vb_compile/src/compile/mod.rs:221` — `pub(crate) fn canonical_digest(source, contract)` |
| **Shared encoding** | `crates/vb_core/src/contract_encoding.rs:27` — `pub fn encode_contract_bytes(contract) -> Vec<u8>` |
| **Re-export path** | `crates/vb_compile/src/mod_compile_lowering.rs:26` — `pub use part_05::*` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_digest_determinism.rs:72` — `prove_digest_determinism` |
| **Encoding harness** | `crates/vb_compile/src/kani_resource_contract_digest_determinism.rs` — `prove_contract_encoding_determinism` |
| **Proptest** | `crates/vb_compile/tests/proptest_digest_determinism.rs` |
| **Status** | Encoding sub-harness APPROVED ✅; blake3 sub-harness CONDITIONAL ⚠️ (BLAKE3_SYMBOLIC_COST) |
| **Test command** | `cargo test -p vb_compile --test proptest_digest_determinism -- --nocapture` |

### PO-K02: Single-Field Sensitivity

| Aspect | Concrete Ref |
|--------|-------------|
| **Production function** | Same `canonical_digest` as PO-K01 → `part_05.rs:116` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_digest_field_sensitivity.rs` — `prove_single_field_changes_digest` |
| **Encoding harness** | `kani_resource_contract_digest_field_sensitivity.rs` — `prove_single_field_changes_encoding` (encoding-only sub-harness) |
| **Proptest** | `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs` — 5 tests |
| **Status** | CONDITIONAL ⚠️ (blake3); proptest APPROVED ✅ |
| **Test command** | `cargo test -p vb_compile --test proptest_contract_field_sensitivity -- --nocapture` |

### PO-K03: Cross-Field Collision Prevention

| Aspect | Concrete Ref |
|--------|-------------|
| **Encoding function** | `crates/vb_core/src/contract_encoding.rs:27` — tagged field encoding with unique field-name prefixes |
| **Kani harness (encoding)** | `crates/vb_compile/src/kani_resource_contract_cross_field_collision.rs` — `prove_no_cross_field_collision_u32` ✅, `prove_no_cross_field_collision_u64` ✅ |
| **Kani harness (blake3)** | `kani_resource_contract_cross_field_collision.rs` — `prove_no_cross_field_collision` ⚠️ |
| **Status** | 2 encoding harnesses APPROVED ✅; 1 blake3 harness CONDITIONAL ⚠️ |
| **Evidence command** | `cargo kani -p vb_compile --harness prove_no_cross_field_collision_u32 --unwind 3 --no-unwinding-checks` |

### PO-K04: Migration Digest

| Aspect | Concrete Ref |
|--------|-------------|
| **Production function** | `part_05.rs:116` — `canonical_digest` incorporates contract into v2 digest |
| **Encoding harness** | `crates/vb_compile/src/kani_resource_contract_migration_digest.rs` — `prove_contract_encoding_is_stable` ✅ |
| **Blake3 harness** | `kani_resource_contract_migration_digest.rs` — `prove_migration_digest_relationship` ⚠️ |
| **Status** | Encoding harness APPROVED ✅; blake3 CONDITIONAL ⚠️ |

### PO-K05: Single Canonical Type (17 Fields)

| Aspect | Concrete Ref |
|--------|-------------|
| **Canonical type** | `crates/vb_core/src/workflow/mod.rs:191-228` — 17-field `ResourceContract` w/ `max_transitions_per_tick` and `allows_secret_results` |
| **Duplicate type (STALE)** | `crates/vb_core/src/compiled_workflow.rs:130-163` — 16-field `ResourceContract` (MISSING: `max_transitions_per_tick`, `allows_secret_results`) |
| **Validation import (WRONG)** | `crates/vb_core/src/validation/resource.rs:12` — `use crate::compiled_workflow::ResourceContract` (16-field duplicate!) |
| **Kani harness** | `crates/vb_core/src/kani_resource_contract_type_canonical_fields.rs` — `prove_canonical_contract_has_17_fields` |
| **Status** | PENDING EXECUTION ⚠️ (CI cluster prerequisite) |
| **Source gap** | `validation/resource.rs` must switch import to `crate::workflow::ResourceContract` for 17-field validation |
| **Evidence command** | `cargo kani -p vb_core --harness prove_canonical_contract_has_17_fields --unwind 1` |

### PO-K06: Type Identity Across Code Paths

| Aspect | Concrete Ref |
|--------|-------------|
| **Validation import (WRONG)** | `crates/vb_core/src/validation/resource.rs:12` — imports 16-field type |
| **Budget import (CORRECT)** | `crates/vb_core/src/budget.rs:7` — `use crate::workflow::{..., ResourceContract, ...}` |
| **Compiled workflow (STALE)** | `crates/vb_core/src/compiled_workflow.rs:21,106,130,225` — uses 16-field type |
| **Lib re-exports (CORRECT)** | `crates/vb_core/src/lib.rs:111` — re-exports `workflow::ResourceContract` |
| **Kani harness** | `crates/vb_core/src/kani_resource_contract_type_identity_paths.rs` — `prove_type_identity_across_paths` |
| **Status** | PENDING EXECUTION ⚠️ (CI cluster prerequisite) |
| **Evidence command** | `cargo kani -p vb_core --harness prove_type_identity_across_paths --unwind 1` |

### PO-K07: Entry Point Contract Parameter

| Aspect | Concrete Ref |
|--------|-------------|
| **Entry point 1** | `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16` — `pub fn compile_source(source, contract: ResourceContract)` |
| **Entry point 2** | `crates/vb_compile/src/compile/mod.rs:25` — `pub fn compile_source(source, contract: ResourceContract)` |
| **Contract in WorkflowParts** | `part_01.rs:55` — `resource_contract: contract` |
| **Duplicate path** | `compile/mod.rs:106` — `resource_contract: contract` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_entry_point.rs` — `prove_contract_survives_compilation` |
| **Encoding harness** | `kani_resource_contract_entry_point.rs` — `prove_non_default_contract_encoding_differs` ✅ |
| **Proptest** | `crates/vb_compile/tests/proptest_entry_point_contract.rs` — 2 tests |
| **Status** | Encoding harness APPROVED ✅; blake3+compilation CONDITIONAL ⚠️; proptest APPROVED ✅ |
| **Test command** | `cargo test -p vb_compile --test proptest_entry_point_contract -- --nocapture` |

### PO-K08: allows_secret_results Digest Sensitivity

| Aspect | Concrete Ref |
|--------|-------------|
| **Encoding function** | `crates/vb_core/src/contract_encoding.rs:83-84` — `b"allows_secret_results"` + bool byte |
| **Canonical type** | `crates/vb_core/src/workflow/mod.rs:227` — `pub allows_secret_results: bool` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_digest_field_sensitivity.rs` — `prove_secret_results_changes_digest` |
| **Proptest** | `crates/vb_compile/tests/proptest_secret_results_digest_sensitivity.rs` |
| **Status** | CONDITIONAL ⚠️ (blake3); proptest APPROVED ✅ |
| **Evidence command** | `cargo test -p vb_compile --test proptest_secret_results_digest_sensitivity -- --nocapture` |

### PO-K09: Runtime SecretResultNotAllowed Enforcement

| Aspect | Concrete Ref |
|--------|-------------|
| **Runtime enforcement** | `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:6-7` — `if answer.taint == Taint::Secret && !contract.allows_secret_results { return Err(RuntimeError::SecretResultNotAllowed); }` |
| **Error variant** | `crates/vb_runtime/src/error/mod.rs:154` — `SecretResultNotAllowed` |
| **Contract source** | `chunk_002.rs:5` — `let contract = state.workflow.resource_contract()` |
| **Kani harness** | `crates/vb_runtime/src/kani_resource_contract_secret_enforcement.rs:29` — `prove_secret_result_not_allowed_enforcement` |
| **Status** | PENDING EXECUTION ⚠️ (CI cluster prerequisite) |
| **Evidence command** | `cargo kani -p vb_runtime --harness prove_secret_result_not_allowed_enforcement --unwind 3` |

### PO-K10: Dual Path Digest Equivalence

| Aspect | Concrete Ref |
|--------|-------------|
| **Path A** | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116` — `canonical_digest` |
| **Path B** | `crates/vb_compile/src/compile/mod.rs:221` — `canonical_digest` |
| **Shared encoding** | Both call `crates/vb_core/src/contract_encoding.rs:27` — `encode_contract_bytes` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_dual_path_equivalence.rs` — `prove_dual_path_digest_equivalence`, `prove_dual_path_digest_equivalence_non_default` |
| **Proptest** | `crates/vb_compile/tests/proptest_dual_path_equivalence.rs` — **DOES NOT TEST DUAL PATHS** (PF-BR-001). Proptest calls `compile_source` twice with identical args (determinism, not dual-path). True dual-path equivalence requires calling `part_05::canonical_digest` and `compile/mod::canonical_digest` independently. |
| **Status** | CONDITIONAL ⚠️ (blake3+compile_source; CI cluster); proptest → planned |
| **Source risk** | Both implementations are structurally identical but not deduplicated. Any drift breaks equivalence. **Proptest determinism test cannot detect drift — only Kani or manual sync can.** |

### PO-K11: Validation 17 Fields

| Aspect | Concrete Ref |
|--------|-------------|
| **Validation function** | `crates/vb_core/src/validation/resource.rs:17` — `validate_resource_contract(parts)` |
| **Current import (WRONG)** | `resource.rs:12` — `use crate::compiled_workflow::ResourceContract` (16-field) |
| **Must switch to** | `use crate::workflow::ResourceContract` (17-field) |
| **Missing validation** | `max_transitions_per_tick` and `allows_secret_results` not validated by current `validate_resource_contract` |
| **Kani harness** | `crates/vb_core/src/kani_resource_contract_validation_17_fields.rs` — `prove_validation_covers_all_17_fields` |
| **Status** | PENDING EXECUTION ⚠️ (CI cluster prerequisite; import fix required first) |
| **Source gap** | `resource.rs` needs: (a) `max_transitions_per_tick == 0` → error, (b) `max_transitions_per_tick > HARD_MAX` → `ResourceContractTooLarge`, (c) `allows_secret_results` valid bool → OK |

### PO-K12: Encoding Injectivity

| Aspect | Concrete Ref |
|--------|-------------|
| **Encoding function** | `crates/vb_core/src/contract_encoding.rs:27` — `encode_contract_bytes` |
| **Field tags** | Lines 32-84: 17 unique ASCII field-name prefixes |
| **Kani harness** | `crates/vb_core/src/kani_resource_contract_encoding_injectivity.rs` — `prove_encoding_no_collision` |
| **Status** | PENDING EXECUTION ⚠️ (CI cluster prerequisite) |
| **Evidence command** | `cargo kani -p vb_core --harness prove_encoding_no_collision --unwind 2` |

### PO-K13: with_default Equivalence

| Aspect | Concrete Ref |
|--------|-------------|
| **Required API** | `pub fn compile_source_with_default(source) -> Result<CompiledWorkflow, ...>` — **NOT YET IMPLEMENTED** (PF-BR-002). `grep -r 'compile_source_with_default' crates/vb_compile/src/` returns ZERO results. |
| **Should delegate to** | `compile_source(source, ResourceContract::DEFAULT)` |
| **Suggested location** | `crates/vb_compile/src/mod_compile_core.rs` |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_dual_path_equivalence.rs` — `prove_with_default_equivalence` |
| **Proptest** | `crates/vb_compile/tests/proptest_with_default_equivalence.rs` — **DOES NOT TEST with_default equivalence** (PF-BR-002). Proptest calls `compile_source(&source, DEFAULT)` twice (determinism, not with_default vs explicit-DEFAULT equivalence). |
| **Status** | CONDITIONAL ⚠️ (blake3; API missing; proptest → planned) |
| **Source gap** | `compile_source_with_default` API does not exist yet. Both Kani harness and proptest obligation are blocked until API is implemented. |

### PO-K14: Canonical vs Policy Digest Agreement

| Aspect | Concrete Ref |
|--------|-------------|
| **Canonical digest** | `part_05.rs:116` — `canonical_digest(source, contract)` → blake3 hash |
| **Policy digest** | `crates/vb_storage/src/admission.rs:204` — `pub fn compute_policy_digest(workflow)` → postcard + blake3 |
| **Kani harness** | `crates/vb_compile/src/kani_resource_contract_digest_determinism.rs` — `prove_canonical_policy_digest_agree_on_identity` |
| **Status** | CONDITIONAL ⚠️ (blake3) |
| **Evidence command** | `cargo kani -p vb_compile --harness prove_canonical_policy_digest_agree_on_identity --unwind 2 --no-unwinding-checks` |

### PO-V01 through PO-V04: Verus Proofs (WAIVED to vb-xi2f.36)

| Obligation | Verus File | Models | Status |
|-----------|-----------|--------|--------|
| PO-V01 | `verification/verus/vb_compile/digest_contract_binding.rs` | For-all contract inequality ⇒ digest inequality | WAIVED ⚠️ (vacuity fix prerequisite) |
| PO-V02 | `verification/verus/vb_compile/encoding_injectivity.rs` | Encoding is injective over all contract pairs | WAIVED ⚠️ (standalone model types) |
| PO-V03 | `verification/verus/vb_compile/secret_results_injectivity.rs` | allows_secret_results injective in hash | WAIVED ⚠️ |
| PO-V04 | `verification/verus/vb_runtime/contract_identity_tracking.rs` | Contract identity preserved through compile→runtime | WAIVED ⚠️ |

**Critical prerequisite (PF-VB-004v3):** `digest_contract_binding.rs:127-157` has a vacuous proof. Both helper functions return identical `Seq::empty()` making the precondition always false. Must be fixed before any vb-xi2f.36 Verus work.

### PO-P01 through PO-P07: Proptest Tests

**⚠️ BRIDGE REPAIR (PF-BR-001, PF-BR-002, PF-BR-003):** Three proptest obligations have claims that diverge from what the actual tests verify. See finding remediation table below.

| Obligation | Test File | Tests | Test Verifies | Mapping Status |
|-----------|----------|-------|---------------|:---:|
| PO-P01 | `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs` | 5 | Field sensitivity (2 fields + allows_secret_results) | ⚠️ planned (coverage partial — see PF-BR-003) |
| PO-P02 | `crates/vb_compile/tests/proptest_entry_point_contract.rs` | 2 | Contract preserved through entry points | ✅ verified |
| PO-P03 | `crates/vb_compile/tests/proptest_secret_results_digest_sensitivity.rs` | 1 | allows_secret_results digest sensitivity | ✅ verified |
| PO-P04 | `crates/vb_compile/tests/proptest_dual_path_equivalence.rs` | 1 | **Determinism only** (same fn ×2). NOT dual-path equivalence | ⚠️ planned (PF-BR-001) |
| PO-P05 | `crates/vb_compile/tests/proptest_digest_determinism.rs` | 1 | Determinism at scale | ✅ verified |
| PO-P06 | `crates/vb_compile/tests/proptest_with_default_equivalence.rs` | 1 | **Determinism only** (same fn ×2). NOT with_default; API absent | ⚠️ planned (PF-BR-002) |
| PO-P07 | Covered by PO-P01 | — | (Partial — see PF-BR-003) | ⚠️ planned |

#### Bridge Finding Remediation (State 5 → State 7)

| Finding | Obligation | Issue | Bridge Action | Unresolved Gap |
|---------|-----------|-------|:---:|------|
| **PF-BR-001** | PO-P04 | Proptest tests single-path determinism, not dual compilation path equivalence | Changed RO-PO-P04 `mapping_status` from `verified` → `planned`; updated `proof_claim` to accurately describe determinism | Dual-path equivalence needs PO-K10 Kani execution (CI cluster) OR proptest extension to call both paths independently |
| **PF-BR-002** | PO-P06 | `compile_source_with_default` API does not exist; proptest tests determinism only | Changed RO-PO-P06 `mapping_status` from `verified` → `planned`; updated `proof_claim` to accurately describe determinism | Need either `compile_source_with_default()` implementation OR recategorize obligation as pending API |
| **PF-BR-003** | PO-P01 | Coverage weaker than obligation claims (2 fields vs 17) | Retained `mapping_status: verified` on RO-PO-P01 (proptest passes correctly) but documented coverage gap in bridge | Kani encoding harnesses (PO-K03u32, PO-K03u64) provide bounded exhaustive encoding-layer coverage |
| **PF-BR-005** | overlap | Three tests overlap on determinism (PO-P04, PO-P05, PO-P06) | Documented overlap; recommendation to consolidate into single determinism test | Repurpose two slots for actual dual-path and with-default equivalence when APIs/material exist |

**Unified evidence command:**
```bash
cargo test -p vb_compile \
  --test proptest_contract_field_sensitivity \
  --test proptest_entry_point_contract \
  --test proptest_secret_results_digest_sensitivity \
  --test proptest_dual_path_equivalence \
  --test proptest_digest_determinism \
  --test proptest_with_default_equivalence \
  -- --nocapture --test-threads=1
```
Result: `11 passed; 0 failed; 0 ignored` in ~0.13s (independently verified).

### PO-F01: Cargo-Fuzz (WAIVED ⚠️ to P2)

| Aspect | Concrete Ref |
|--------|-------------|
| **Status** | WAIVED per WC-001 (P2 priority) |
| **Target** | `fuzz/fuzz_targets/yaml_resource_contract.rs` (to be implemented in P2) |

## Source File Impact Matrix

| File | Proposed Change | Risk | Proof Impact |
|------|----------------|------|-------------|
| `crates/vb_core/src/workflow/mod.rs:191-228` | Canonical 17-field type (authoritative) | LOW | All proof targets |
| `crates/vb_core/src/compiled_workflow.rs:130-163` | **DELETE duplicate 16-field type**; re-export canonical | HIGH | PO-K05, K06 |
| `crates/vb_core/src/validation/resource.rs:12` | **Fix import**: `compiled_workflow` → `workflow` | HIGH | PO-K11 |
| `crates/vb_core/src/validation/resource.rs:17-21` | Add `max_transitions_per_tick` + `allows_secret_results` validation | MEDIUM | PO-K11 |
| `crates/vb_core/src/contract_encoding.rs:27-87` | Shared encoding (no change needed) | NONE | PO-K01-K04, K08, K12, K14 |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116` | Canonial digest (no change needed) | NONE | PO-K01, K07, K10, K14 |
| `crates/vb_compile/src/compile/mod.rs:221` | Canonical digest duplicate (consider deduplication) | MEDIUM | PO-K10 |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16` | Entry point w/ contract (no change needed) | NONE | PO-K07 |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:5-7` | Runtime enforcement (no change needed) | NONE | PO-K09 |

## GOD RULE Alignment Check

| GOD RULE | Assessment | Evidence |
|----------|-----------|----------|
| **1: No Hardcoded Kani Shapes** | **PASS** | All 15 Kani harnesses use `kani::any()` + `kani::assume()` bounds. No hardcoded structs. |
| **2: No Vacuum Verus Proofs** | **WAIVED** ⚠️ | Verus deferred to vb-xi2f.36. `digest_contract_binding.rs` vacuity documented. |
| **3: No Unbounded TLA+ Math** | **N/A** | No TLA+ applied for this bead. |
| **4: No Loop Oscillations** | **COMPLIANT** | Production functions fixed per plan; harnesses test actual code. Bridge repair does not alter mathematical contracts — it corrects claim fidelity. |
| **5: No Blind Verification Mutations** | **COMPLIANT** | Scope limited to ResourceContract digest call-graph. Bridge repair corrects mapping claims without altering verification scope. |

## Bridge Repair Record

**Repair invocation:** `proof-to-implementation` agent, bead `vb-xi2f.35`, 2026-05-26

**Findings repaired:**

| Finding | Severity | Repair Action | Status |
|---------|:---:|------|:---:|
| PF-BR-001 | CRITICAL | RO-PO-P04 `mapping_status`: `verified` → `planned`; `proof_claim` corrected to "compile_source determinism"; dual-path equivalence mapped to Kani PO-K10. PO-P04 proptest table entry updated with honest description. | ✅ REPAIRED |
| PF-BR-002 | CRITICAL | RO-PO-P06 `mapping_status`: `verified` → `planned`; `proof_claim` corrected to "DEFAULT contract determinism"; missing API documented. PO-P06 proptest table entry updated. PO-K13 status updated. | ✅ REPAIRED |
| PF-BR-003 | HIGH | Bridge now accurately documents 2-field+allows_secret_results proptest coverage. Kani encoding harnesses provide bounded exhaustive coverage. | ✅ DOCUMENTED |
| PF-BR-004 | MEDIUM | PO-K05/K06 harness import gap documented (unresolved — requires validation.rs import fix). | ✅ DOCUMENTED |
| PF-BR-005 | LOW | Test overlap documented with consolidation recommendation. | ✅ DOCUMENTED |

## Closure Obligations (for State 12)

The following `mapping_status: planned` items must be resolved to `materialized` or `verified` before State 12 handoff:

1. **CI cluster execution** (13 conditional Kani harnesses): PO-K01-K04, K07, K08, K10, K14 (blake3) + PO-K05, K06, K11, K12 (other-crate) → must run on CI with 30+ min budgets
2. **Validation import fix**: `resource.rs:12` must switch to canonical type → unblocks PO-K11 verification
3. **Duplicate type resolution**: `compiled_workflow.rs:130` must reference canonical type → unblocks PO-K05, K06 verification
4. **compile_source_with_default API**: Missing convenience function → needed for PO-K13 behavior test AND PO-P06 proptest with_default equivalence (PF-BR-002)
5. **Dual-path deduplication**: `compile/mod.rs:221` should be deduplicated or explicitly proven equivalent via CI execution → PO-P04 proptest does NOT verify dual paths; only Kani PO-K10 does (PF-BR-001)
6. **PO-P04 proptest dual-path gap**: Extend proptest to call both `part_05::canonical_digest` and `compile/mod::canonical_digest` independently, OR accept that dual-path equivalence coverage comes from Kani PO-K10 only (PF-BR-001)
7. **PO-P06 proptest with_default gap**: Either implement `compile_source_with_default()` and extend proptest, or recategorize PO-P06 as pending API implementation (PF-BR-002)
8. **Verus vacuity fix (vb-xi2f.36 prerequisite)**: `digest_contract_binding.rs:147` must be fixed before any vb-xi2f.36 Verus work
9. **PO-F01 fuzz target**: Implement in P2 bead per WC-001 waiver

## Unresolved Mapping Gaps

| Gap | Severity | Description | Bridge Finding |
|-----|---------|-------------|:---:|
| **GAP-DUP-TYPE** | **HIGH** | `compiled_workflow::ResourceContract` (16 fields) is a stale duplicate of `workflow::ResourceContract` (17 fields). Blocks type-identity proofs. | PF-BR-004 |
| **GAP-VALIDATE-IMPORT** | **HIGH** | `validation/resource.rs` imports the 16-field duplicate. Must switch to canonical 17-field type. | PF-BR-004 |
| **GAP-DUAL-DIGEST** | **HIGH** ⬆️ | Both `canonical_digest` implementations are structurally identical but independently maintained. Proptest does NOT verify dual-path equivalence (PF-BR-001) — only tests single-path determinism. Drift risk is unmitigated by current proptest suite. Kani PO-K10 provides dual-path coverage (CI cluster pending). | PF-BR-001 |
| **GAP-WITH-DEFAULT** | **HIGH** ⬆️ | `compile_source_with_default` convenience function does not exist. Proptest does NOT test with_default equivalence (PF-BR-002) — only tests DEFAULT contract determinism. Both Kani PO-K13 and proptest PO-P06 blocked by missing API. | PF-BR-002 |
| **GAP-VERUS-VACUITY** | **DEFERRED** | `digest_contract_binding.rs` vacuity — tracked to vb-xi2f.36 | PF-VB-004v3 |
| **GAP-BRIDGE-CLAIM-FIDELITY** | **REPAIRED** ✅ | PO-P04, PO-P06, PO-P01 bridge claims corrected per bridge review findings PF-BR-001/002/003. `rust-refinement-obligations.jsonl` mapping_status fields updated. Proptest coverage documented honestly. | N/A |

## Reviewer Handoff Inputs

For `proof-reviewer` State 7 gate re-review (bridge repair):

1. **This file**: `proof-to-rust-map.md` (repaired — PF-BR-001, PF-BR-002 claims corrected)
2. **Refinement obligations**: `rust-refinement-obligations.jsonl` (RO-PO-P04, RO-PO-P06 rows repaired)
3. **Proof review**: `.beads/vb-xi2f.35/proof-review.md` (R5, CONDITIONALLY APPROVED)
4. **Proof obligations**: `.beads/vb-xi2f.35/proof-obligations.planned.jsonl`
5. **Proof findings**: `proof-findings.jsonl` (PF-BR-001 through PF-BR-005; BR-001/BR-002 addressed)
6. **Bridge review**: `.beads/vb-xi2f.35/proof-to-rust-review.md` (original rejection — contains the findings this repair addresses)
7. **Trusted-base ledger**: `.beads/vb-xi2f.35/trusted-base-ledger.jsonl`
8. **Verification ledger**: `verification-ledger.jsonl`

**Repair summary:** RO-PO-P04 and RO-PO-P06 `mapping_status` changed from `verified` to `planned`. Bridge claims now accurately describe what each proptest actually verifies (determinism, not dual-path/with-default). Two CRITICAL bridge findings are addressed but closure depends on implementation decisions (PO-K10 CI execution for dual-path, `compile_source_with_default` API implementation for with-default).

**Note**: `proof-to-rust-review.md` is written by `proof-reviewer`, not this agent. This file is the bridge mapping artifact for reviewer consumption.
