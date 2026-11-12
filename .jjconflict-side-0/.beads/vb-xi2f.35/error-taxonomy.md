# Error Taxonomy: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Railway Error Model

Errors flow through the compilation pipeline as `Result<T, E>`. All errors are semantic (domain-meaningful), not stringly-typed.

## Error Hierarchy

```
CompilationError
├── ParseError (vb_yaml)
│   ├── YamlError::FieldShape { field, expected }
│   ├── YamlError::MissingField { field }
│   ├── YamlError::UnknownField { field }
│   └── YamlError::InvalidResourceContract { field, reason }  ← NEW
├── CompileError (vb_compile)
│   ├── CompileError::UnsupportedStepPrimitive { step, primitive }
│   ├── CompileError::ExpressionLoweringUnsupported { feature }
│   ├── CompileError::ContractDigestMismatch                   ← NEW
│   └── CompileError::ContractValidation { field, reason }     ← NEW
│
├── WorkflowError (vb_core)
│   ├── WorkflowError::ResourceContractExceeded { resource }
│   ├── WorkflowError::ResourceContractTooLarge { resource }
│   ├── WorkflowError::InvalidResourceContract { field, detail }  ← NEW
│   ├── WorkflowError::StepOutOfBounds { step }
│   ├── WorkflowError::SlotOutOfBounds { slot }
│   ├── WorkflowError::ExprOutOfBounds { expr }
│   ├── WorkflowError::ConstOutOfBounds { constant }
│   ├── WorkflowError::EntryOutOfBounds { entry }
│   └── WorkflowError::BudgetExceeded { dimension, requested, available }
│
├── RuntimeError (vb_runtime)
│   ├── RuntimeError::SecretResultNotAllowed
│   ├── RuntimeError::IpcPayloadSizeExceeded { size, max }
│   ├── RuntimeError::RunNotFound
│   └── RuntimeError::InvalidTimerFire
│
└── JournalError (vb_storage)
    └── JournalError::ArtifactMalformed
```

## Semantic Error Variants (Current and Required)

### Existing Errors — Contract-Relevant

| Variant | Crate | Location | Behavior |
|---------|-------|----------|----------|
| `WorkflowError::ResourceContractExceeded { resource }` | vb_core | `validation/resource.rs:83` | A resource count exceeds the contract limit |
| `WorkflowError::ResourceContractTooLarge { resource }` | vb_core | `validation/resource.rs:79` | A contract limit exceeds the system hard limit |
| `RuntimeError::SecretResultNotAllowed` | vb_runtime | `shard/lifecycle/chunk_002.rs:7` | Secret-tainted answer rejected by `allows_secret_results=false` |
| `RuntimeError::IpcPayloadSizeExceeded { size, max }` | vb_runtime | `shard/lifecycle/chunk_002.rs:10` | Answer payload exceeds `max_ipc_payload_bytes` |

### Required New Error Variants

| Variant | Crate | Trigger | Behavior Affecting? |
|---------|-------|---------|---------------------|
| `CompileError::ContractDigestMismatch` | vb_compile | Contract in compilation does not match expected digest | YES — blocks compilation |
| `WorkflowError::InvalidResourceContract { field, detail }` | vb_core | Contract field fails validation (e.g., zero max_steps) | YES — blocks validation |
| `YamlError::InvalidResourceContract { field, reason }` | vb_yaml | Malformed contract in YAML source | YES — blocks parsing |
| `WorkflowError::ContractFieldMissing { field }` | vb_core | Required contract field not specified when sourcing from non-DEFAULT | YES — blocks validation |

## Error Behavior by Digestion Phase

### Phase 1: Contract Resolution (NEW)

| Failure | Error | Recovery |
|---------|-------|----------|
| YAML contract field missing | `YamlError::MissingField { field: "resource_contract" }` | Use DEFAULT |
| YAML contract field invalid | `YamlError::InvalidResourceContract { field, reason }` | Report to author |
| API contract invalid | `WorkflowError::InvalidResourceContract { field, detail }` | Reject at boundary |
| No contract specified | Use `ResourceContract::DEFAULT` | Default is always valid |

### Phase 2: Digest Computation

| Failure | Error | Recovery |
|---------|-------|----------|
| Contract not provided to `canonical_digest` | Compile error (type system should prevent) | Fix call site |
| Hash collision (theoretical) | None (32-byte blake3 collision is astronomically unlikely) | N/A |

### Phase 3: Validation

| Failure | Error | Recovery |
|---------|-------|----------|
| `max_steps < node_count` | `ResourceContractExceeded { resource: "max_steps" }` | Increase contract or reduce steps |
| `max_steps > MAX_STEPS_PER_WORKFLOW` | `ResourceContractTooLarge { resource: "max_steps" }` | Reduce contract |
| `contract_field > hard_limit` | `ResourceContractTooLarge { resource }` | Reduce contract |
| `max_transitions_per_tick == 0` | `BudgetExceeded { ... }` | Set positive value |
| `max_transitions_per_tick > HARD_MAX` | `ResourceContractTooLarge { resource: "max_transitions_per_tick" }` | Reduce contract |

### Phase 4: Runtime Enforcement

| Failure | Error | Recovery |
|---------|-------|----------|
| Secret answer + `allows_secret_results=false` | `SecretResultNotAllowed` | Change contract or sanitize answer |
| Payload > `max_ipc_payload_bytes` | `IpcPayloadSizeExceeded` | Reduce payload or increase limit |

## Error Recovery Policies

| Error Class | Recovery Strategy |
|-------------|-------------------|
| **Parse errors** | Return to caller. Invalid YAML cannot be recovered. |
| **Contract validation errors** | Return to caller with diagnostic. Contract is semantic policy — user must fix. |
| **Digest mismatch** | Return `Err`. Indicates contract-semantic violation — a bug, not a user error. |
| **ResourceContractExceeded** | Return to caller. Workflow exceeds declared limits. Author must adjust contract or workflow. |
| **Taint violations** | Return to runtime. Cannot recover — answer is rejected. |
| **Payload size exceeded** | Return to runtime. Cannot recover — answer is rejected. |

## Hard Limits (System Invariants)

These are the absolute maximums enforced by `validate_contract_limit()`:

| Limit | Hard Max | Source |
|-------|----------|--------|
| `max_steps` | `MAX_STEPS_PER_WORKFLOW` | `vb_core::limits` |
| `max_slots` | `MAX_SLOTS_PER_WORKFLOW` | `vb_core::limits` |
| `max_constants` | `MAX_CONSTANTS` | `vb_core::limits` |
| `max_accessors` | `MAX_ACCESSORS` | `vb_core::limits` |
| `max_expressions` | `MAX_EXPRESSIONS` | `vb_core::limits` |
| `max_expr_stack` | `MAX_EXPRESSION_STACK` | `vb_core::limits` (u8) |
| `max_transitions_per_tick` | `HARD_MAX_TRANSITIONS_PER_TICK` | `vb_core::budget` |
| `max_step_budget_per_tick` | `HARD_MAX_STEP_BUDGET_PER_TICK` | `vb_core::budget` |

Any contract field exceeding its hard limit MUST produce `WorkflowError::ResourceContractTooLarge`.

## Panic Hazards

| Location | Risk | Mitigation |
|----------|------|------------|
| `validate_resource_contract()` currently uses `usize::from(contract.max_steps)` — the 15-field type prevents accessing `max_transitions_per_tick` | Missing validation | Extend to 17-field type |
| `compute_policy_digest()` has `unwrap_or_else` chains for postcard serialization | Panic-free through fallbacks | Already handled |
| `handle_ask_answer()` accesses `contract.allows_secret_results` | Safe — field exists on canonical type | No panic risk |
