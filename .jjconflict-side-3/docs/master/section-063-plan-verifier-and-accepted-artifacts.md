---
section: 63
title: "Plan Verifier and Accepted Artifacts"
parent: velvet-ballistics-MASTER.md
---

## 63. Plan Verifier and Accepted Artifacts


### Core Principle

AI may propose workflows. Velvet verifies them. Only accepted artifacts run.

The compiler does not merely check syntax. It acts as a safety gate: if Velvet cannot prove the plan is bounded, inspectable, retry-safe, and durable, the plan is rejected before execution. No accepted workflow has unknown bounds.

### Verification Gate Pipeline

```text
YAML/Rust workflow definition
  → strict YAML parser (gate 1: profile)
  → schema validator (gate 2: shape)
  → name/scope validator (gate 3: names)
  → reference validator (gate 4: references)
  → expression compiler (gate 5: expressions)
  → control-flow validator (gate 6: CFG)
  → boundedness analyzer (gate 7: bounded — section 64)
  → resource budget checker (gate 8: budgets)
  → action contract verifier (gate 9: contracts)
  → taint/secret checker (gate 10: taint)
  → idempotency verifier (gate 11: idempotency — section 65)
  → durability checker (gate 12: durability)
  → capability checker (gate 13: capabilities)
  → result/output validator (gate 14: results)
  → observability checker (gate 15: evidence)
  → accepted artifact
  → runtime admission (section 66)
```

A workflow must pass every gate to produce an accepted artifact. The runtime must not execute anything that is not an accepted artifact.

### Accepted Artifact Record

When a workflow passes all verification gates, the compiler persists a verifiable artifact:

```rust
pub struct AcceptedArtifact {
    pub artifact_version: &'static str,  // "velvet.artifact/v1"
    pub workflow_name: Box<str>,
    pub workflow_version: &'static str,  // "velvet-ballistics/v1"
    pub workflow_digest: WorkflowDigest, // BLAKE3 of YAML source
    pub ir_digest: WorkflowDigest,       // BLAKE3 of compiled IR
    pub action_contract_digest: WorkflowDigest, // BLAKE3 of action contracts
    pub verified_at: u64,                // Unix timestamp
    pub resource_budget: WholeWorkflowBudget, // section 64
    pub capabilities: Box<[Capability]>, // section 66
    pub warnings: Box<[VerificationWarning]>,
    pub verification: VerificationProof,
}

pub struct VerificationProof {
    pub bounded: bool,
    pub taint_safe: bool,
    pub retry_safe: bool,
    pub durable: bool,
    pub replayable: bool,
    pub idempotency_keyed: Vec<ActionId>,   // actions with well-formed idempotency keys
    pub idempotency_attested: Vec<ActionId>, // actions attested idempotent by contract (external claim)
}

pub struct VerificationWarning {
    pub code: u32,
    pub message: Box<str>,
    pub gate: u8, // which verification gate produced it
}
```

Runs bind to this artifact by digest, not to loose YAML or unverified `CompiledWorkflow`.

### Accepted Artifact Persistence

Accepted artifacts are stored in the `compiled_ir` keyspace keyed by `ir_digest`. The storage layer already stores compiled IR by digest; the artifact record wraps the IR with verification metadata.

### Strict Verification Mode

For AI-authored workflows, strict mode is available:

```text
velvet-ballistics verify flow.yaml --profile strict --emit yaml
```

Strict mode rejects not only errors but selected warnings:

- unused secrets
- unsafe shell actions
- large fanout (branches > policy threshold)
- missing examples
- retry on side-effecting action without idempotency proof
- possibly skipped references
- opaque object where schema could be declared

This is the workflow equivalent of compile-with-warnings-as-errors. AI agents should use `--strict` as the default.

### Verification Gate Status

| Gate | Status | Notes |
|------|--------|-------|
| 1. YAML profile | Implemented | vb_yaml strict profile, 19 error variants |
| 2. Shape/schema | Implemented | vb_validate + vb_compile schema validation |
| 3. Name/scope | Implemented | ID grammar, reserved words enforcement |
| 4. Reference | Implemented | Forward refs rejected, runtime refs rejected |
| 5. Expression | Implemented | 30 opcodes, bytecode compiler, bounded stacks |
| 6. Control flow | Implemented | Forward-only CFG, cycle rejection, reachability |
| 7. Boundedness | Implemented, evidence-gated | `WholeWorkflowBudget`/`BoundednessPolicy` exist and `vb_compile` calls shared validation; full release evidence still required. |
| 8. Resource budget | Implemented, evidence-gated | `ResourceContract`, whole-workflow computation, arena caps, `BudgetExceeded`, and hard step-budget ceilings exist; full gate evidence still required. |
| 9. Action contract | Partial | `ActionContract`, `SideEffect`, `RetrySafety`, idempotency checks, and action contract validation surfaces exist; external attestation/schema parity evidence remains required. |
| 10. Secret/taint | Implemented | Compile-time + runtime taint, leak rejection, 3-level lattice |
| 11. Idempotency | Implemented, evidence-gated | `Idempotency`, `SideEffect`, `RetrySafety`, `IdempotencyViolation`, and verifier/runtime admission plumbing exist; generated/replay parity evidence remains a release gate. |
| 12. Durability | Partial | Journal events, per-primitive durability matrix, and `SlotWritten` value/taint evidence exist; pending-action recovery and strict ack ordering remain gates. |
| 13. Capability | Implemented, evidence-gated | `Capability`/`CapabilitySet` types and runtime admission enforcement exist; schema/CLI/e2e parity evidence remains required. |
| 14. Output/result | Implemented | Result validation, finish semantics |
| 15. Observability | Partial | Trace ring + counters; evidence chain gaps |

---
