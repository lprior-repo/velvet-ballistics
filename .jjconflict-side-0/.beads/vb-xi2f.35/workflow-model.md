# Workflow Model: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Workflow: Compilation with Resource Contract

### State Machine: Compilation Pipeline

```
                    ┌─────────────┐
                    │   YAML Raw  │
                    │    Bytes    │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │   Parse     │
                    │  to AST     │
                    │(WorkflowSrc)│
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
    ┌─────────▼──────┐    │    ┌───────▼──────────┐
    │ Resource       │    │    │ Resource         │
    │ Contract from  │    │    │ Contract from    │
    │ DEFAULT const  │    │    │ YAML / API param │
    │ (current path) │    │    │ (future path)    │
    └─────────┬──────┘    │    └───────┬──────────┘
              │            │            │
              └────────────┼────────────┘
                           │
                    ┌──────▼──────┐
                    │  Canonical  │
                    │  Digest     │
                    │(source +    │
                    │ contract)   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  Build      │
                    │  Workflow   │
                    │  Parts      │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  Validate   │
                    │  Parts      │
                    │(incl.       │
                    │ contract)   │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
    ┌─────────▼──────┐    │    ┌───────▼──────────┐
    │ Validation     │    │    │ Validation       │
    │ FAILS          │    │    │ PASSES           │
    │ → Err          │    │    │ → CompiledWf     │
    └────────────────┘    │    └───────┬──────────┘
                           │            │
                    ┌──────▼──────┐    │
                    │  Runtime    │◄───┘
                    │  Admission  │
                    │ (checks     │
                    │  contract)  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │                         │
    ┌─────────▼──────┐         ┌───────▼──────────┐
    │ Execute with   │         │ Execute with     │
    │ Contract A     │         │ Contract B       │
    │ (expects A's   │         │ (expects B's     │
    │  behavior)     │         │  behavior)       │
    └────────────────┘         └──────────────────┘
```

### States

| State | Description | Entry Condition |
|-------|-------------|-----------------|
| `RawSource` | Raw YAML bytes | External input |
| `ParsedAST` | `WorkflowSource` | Successful YAML parsing |
| `ContractResolved` | `ResourceContract` is determined | Contract from YAML, API, or DEFAULT |
| `Digested` | `WorkflowDigest` computed from source + contract | Canonical digest function |
| `PartsBuilt` | `WorkflowParts { digest, contract }` constructed | Lowering complete |
| `Validated` | `CompiledWorkflow` after validation | All invariants hold |
| `Admitted` | Artifact persisted with policy digest | Admission gate passes |
| `Executing` | Runtime executing with contract limits | Run started |

### Transitions

| From | To | Trigger | Guard |
|------|----|---------|-------|
| `RawSource` | `ParsedAST` | `parse_workflow_from_yaml()` | Valid YAML syntax |
| `RawSource` | `Err(ParserError)` | Parse failure | N/A |
| `ParsedAST` | `ContractResolved` | Contract resolution | Contract from YAML or DEFAULT |
| `ParsedAST + ContractResolved` | `Digested` | `canonical_digest(source, contract)` | Both inputs valid |
| `Digested + ContractResolved + PartsBuilt` | `Validated` | `CompiledWorkflow::try_from_parts()` | `validate_parts()`, `validate_budget()`, `validate_resource_contract()` pass |
| `Validated` | `Err(WorkflowError)` | Validation failure | Contract exceeded or parts invalid |
| `Validated` | `Admitted` | `submit_artifact()` | Policy gates pass |
| `Admitted` | `Executing` | `Shard::start_run()` | Capacity available |
| `Executing` | `Executing` | Tick budget available | Contract limits not exceeded |
| `Executing` | `Err(RuntimeError::SecretResultNotAllowed)` | Secret taint on answer + `allows_secret_results=false` | Taint violation |
| `Executing` | `Err(RuntimeError::IpcPayloadSizeExceeded)` | Answer payload > `max_ipc_payload_bytes` | Limit violation |
| `Executing` | `Terminal(Finished)` | Workflow completes | All steps executed |

### Temporal Hazards: Digest-Consistency

**Hazard D1: Silent Contract Substitution**

```
Scenario:
1. Compile workflow with contract A → digest D
2. Compile workflow with contract B → digest D (SAME! — invariant violation)
3. Admit artifact with digest D, contract A
4. Later, admit artifact with digest D, contract B (digest collision!)
5. Admission layer cannot distinguish the two contracts
```

**Impact**: An attacker or operator could substitute a different contract without detection. For example, change `allows_secret_results` from `false` to `true`, or relax `max_steps` from 10000 to u16::MAX — the digest would stay the same, and the admission verification would pass.

**Current Status**: BUG EXISTS. `canonical_digest()` does not hash any ResourceContract fields.

**Hazard D2: Duplicate Type Divergence**

```
Scenario:
1. Compile using vb_core::workflow::ResourceContract (17 fields)
2. Validate using vb_core::compiled_workflow::ResourceContract (15 fields)
3. max_transitions_per_tick and allows_secret_results are silently dropped
```

**Impact**: Runtime code references `allows_secret_results` (see `chunk_002.rs:6`), but validation code uses the 15-field type that does not have this field. The validation layer cannot check these two critical contract dimensions.

### Temporal Hazards: Compilation Path Divergence

**Hazard D3: Dual Path Drift**

Both `mod_compile_lowering/part_05.rs` and `compile/mod.rs` have independent `canonical_digest()` and `lower_steps_to_ir()` implementations. If only one is fixed, the other remains broken.

### Idempotence

| Operation | Idempotent? | Condition |
|-----------|-------------|-----------|
| `canonical_digest(source, contract)` | YES | Deterministic hash of immutable inputs |
| `parse_workflow_from_yaml(bytes)` | YES | Deterministic parser |
| `compile_source(source, contract)` | YES | Deterministic compilation given same inputs |
| `submit_artifact(workflow)` | NO | Has side effects (journal append) |
| `DEFAULT` const usage | YES | Same const always returns same value |

### Retry Paths

| Failure | Retry? | Reason |
|---------|--------|--------|
| Parse failure | Yes (fix source, retry) | Correctable input |
| Validation failure | Yes (fix constraints, retry) | Correctable input |
| Digest mismatch | No (bug) | Indicates contract-semantic gap |
| Runtime contract exceeded | Maybe | Depends on runtime policy |

### Cancellation Paths

- Compilation can be aborted at any point before `submit_artifact()` with no side effects.
- After admission, cancellation follows the shard lifecycle cancel flow.

### Guards

| Guard | Location | Condition |
|-------|----------|-----------|
| `validate_resource_contract` | `vb_core::validation::resource.rs` | Contract fields ≤ hard limits; actual counts ≤ contract limits |
| `validate_budget` | `vb_core::validation` | Budget arithmetic (for max_transitions_per_tick) |
| `allow_secret_results` guard | `vb_runtime::shard::lifecycle::chunk_002.rs` | `answer.taint == Secret && !contract.allows_secret_results → Err` |
| `validate_parts` | `vb_core::validation` | Index bounds, slot counts, entry validity |
| `compute_policy_digest` | `vb_storage::admission.rs` | Serializes contract and hashes (separate from canonical digest) |

### Terminal Outcomes

| Outcome | When |
|---------|------|
| `Ok(CompiledWorkflow)` | All validations pass, workflow ready for admission |
| `Err(CompileErrors)` | Compilation or validation failure |
| `Ok(AcceptedArtifact)` | Admission complete, artifact persisted |
| `Err(RuntimeError::SecretResultNotAllowed)` | Secret answer when `allows_secret_results=false` |
| `Err(RuntimeError::ResourceContractExceeded)` | Resource limit exceeded at runtime |
