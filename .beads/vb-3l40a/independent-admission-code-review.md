# vb-3l40a Independent Admission Code Review

**Candidate SHA:** 713902f756ea825ff6500c838a087dbb98f25cf4
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-test-review-5090
**Reviewer:** Independent 5090 agent
**Date:** 2026-10-21

## Verification Workspace Roots

- Git root: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-test-review-5090
- JJ root: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-test-review-5090

## Scope

Production runtime/shard changes for admission contract validation:
- `crates/vb_runtime/src/shard/contract_validation.rs` (NEW, 107 lines)
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` (handle_submit_with_contracts, handle_submit_with_inputs_and_contracts, validate_submit_admission_with_contracts)
- `crates/vb_runtime/src/shard/mod.rs` (module declaration)
- `crates/vb_runtime/src/runtime.rs` (API passthrough)

## Production Code Findings

### contract_validation.rs — Three Validation Helper Functions

**check_duplicate_ids:** Global pass over all contracts using HashSet to detect duplicate action IDs. Correctly returns first duplicate encountered.

**check_empty_names:** Global pass verifying all contract names are non-empty. Correctly returns first empty name encountered.

**check_dense_index:** Global pass verifying index == ID for dense registry model. Uses `u16::try_from(index)` with proper error mapping. Uses `contracts.get(action_index)` for bounded access in binding check.

**validate_contract_ids:** Orchestrates three global passes in correct precedence order: duplicate > empty name > dense index.

**validate_contract_capabilities:** Validates granted capabilities cover all contract requirements. Returns first uncovered capability.

**validate_workflow_action_binding:** Iterates workflow nodes, verifies every Do action has a contract at its exact registry index with matching ID.

### chunk_001.rs — Validation-Before-Enqueue Pattern

**handle_submit_with_contracts:** Calls `validate_submit_admission_with_contracts` BEFORE `handle_submit_with_inputs_contracts_and_header_mode`. No journal mutation on validation failure.

**handle_submit_with_inputs_and_contracts:** Same validate-then-enqueue pattern. No journal mutation on validation failure.

**validate_submit_admission_with_contracts:** Delegates to contract_validation helpers in correct order:
1. build_admission (run/digest/capability set construction)
2. validate_contract_ids (duplicate/empty/dense checks)
3. validate_contract_capabilities
4. validate_workflow_action_binding

### runtime.rs — API Passthrough

Both `submit_direct_with_grants_and_contracts` and `submit_direct_with_inputs_grants_and_contracts` pass `&workflow` to shard, which validates before enqueue.

## Contract Compliance

| Requirement | Status |
|-------------|--------|
| Validate action contract IDs against actual workflow Do actions | PASS |
| Dense registry model (index == ID) | PASS |
| Fabricated/sparse/reordered IDs rejected | PASS |
| Duplicate action IDs rejected | PASS |
| Missing contracts for referenced Do actions rejected | PASS |
| Extra contracts for unused actions allowed | PASS |
| Both Runtime APIs validate before enqueue | PASS |
| Both shard command handlers validate before journal mutation | PASS |
| No-contract behavior preserved | PASS |
| Error precedence: duplicate > empty name > dense index > capability > binding | PASS |

## Engineering Rules Compliance

| Rule | Status |
|------|--------|
| No unsafe | PASS (#![forbid(unsafe_code)] in contract_validation.rs) |
| No unwrap/expect/panic (production) | PASS |
| No unchecked indexing/casts/arithmetic | PASS (u16::try_from, contracts.get) |
| Bounded iteration | PASS (enumerate over contracts, 0u16..node_count) |
| Proper error propagation | PASS (RuntimeResult throughout) |
| Source length < 300 lines | PASS (107 lines) |
| No new forbidden constructs | PASS |

## Production Binding

Validation uses production types: CompiledWorkflow, ActionId, ActionContract, CompiledNodeKind::Do. Iterates actual workflow nodes and inspects actual Do node action IDs. Bound to production behavior.

## Disposition

STATUS: APPROVED

No open findings. Production code changes are correct, bounded, and compliant with the admission contract addendum and engineering rules. Both public ingress paths validate before enqueue with no journal mutation on failure. Dense registry model correctly implemented with proper error precedence (duplicate > empty name > dense index > capability > binding).
