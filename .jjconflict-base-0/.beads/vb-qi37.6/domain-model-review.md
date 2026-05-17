# Domain Model Review

## Decision

Strict least privilege is the domain rule: grants are exact `(name, action_id)` pairs. Hierarchical parent grants are rejected. Cardinality exactness is part of admission, so extra grants fail closed just like missing grants.

## Findings from current code

- `vb_core::capability::Capability` is a serializable `(name, action)` value. `CapabilitySet` owns a bounded vector of grants.
- `CapabilitySet::grants` ignores empty grant names, then requires `grant.name() == required.name()` and `grant.action == required.action`.
- Gate 12 validation checks capability name grammar, action mismatch, and duplicate requirements inside `ActionContract.required_capabilities`.
- `AcceptedArtifact.required_capabilities` exists in storage, but `submit_artifact` currently persists `Box::new([])` for Relaxed, Journaled, and Strict paths.
- `vb_runtime::admission::REQUIRED_GATE_COUNT` is `15`; storage `ADMISSION_GATE_COUNT` is `2`.
- `admit_artifact_run` validates gate count/proof flags, then requires `caps.len() == artifact.required_capabilities.len()` and exact grant coverage.
- `Runtime::submit_direct`, `submit_compiled`, and `submit_compiled_with_inputs` pass `CapabilitySet::empty()`.
- `Shard::drive_state` forwards `&[]` action contracts to `drive_deterministic_full`; Do nodes therefore fail closed through `execute_do_without_contract`.
- `ActionDescriptionView.required_capabilities` exists and must remain a projection of the same source of truth.

## Required type-model shape

1. `ActionContract.required_capabilities` is the authoring source.
2. Gate 12 validates grammar, action equality, and duplicates.
3. Accepted artifact persistence copies the validated requirements exactly.
4. Runtime submit binds caller grants before admission.
5. Admission compares exact cardinality and exact pair coverage.
6. Run state carries granted capabilities.
7. Shard drive carries validated contracts and grants into Do execution.
8. UI renders the same validated required-capability slice.

## Illegal states to eliminate

- Accepted artifact with non-empty Do requirements in contracts but empty persisted `required_capabilities`.
- Strict/Journaled artifact accepted by storage with gate count `2` then expected to pass runtime gate count `15`.
- Public Strict/Journaled submit of a capability-protected workflow with no grant API.
- Shard-owned Do execution with empty contract slice for a workflow containing Do nodes.
- UI view showing capabilities from a different source than runtime enforcement.

## Review conclusion

The core exact-match capability type is usable. The failing domain model is end-to-end propagation: required capabilities and action contracts are not consistently carried through storage, runtime API, shard state, engine drive, and UI. The contract must force fail-closed behavior until these links are repaired.
