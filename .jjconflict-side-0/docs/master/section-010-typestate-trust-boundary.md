---
section: 10
title: "Typestate Trust Boundary"
parent: velvet-ballistics-MASTER.md
---

## 10. Typestate Trust Boundary

The SDK must use typestate to make invalid lifecycle transitions unrepresentable.

```rust
pub struct WorkflowDefinition {
    ast: WorkflowAst,
    source_digest: SourceDigest,
}

pub struct AcceptedWorkflow {
    ast: WorkflowAst,
    certificate: VerificationCertificate,
    policy_digest: PolicyDigest,
    action_abi_digest: ActionAbiDigest,
    resource_budget: WholeWorkflowBudget,
}

pub struct AcceptedArtifact {
    bytes: ArtifactBytes,
    artifact_digest: ArtifactDigest,
    ir_digest: IrDigest,
    policy_digest: PolicyDigest,
    action_abi_digest: ActionAbiDigest,
}
```

Allowed transition:

```text
WorkflowDefinition -> VerificationOutcome::Accepted(AcceptedWorkflow) -> AcceptedArtifact -> InstalledArtifact -> RunAccepted
```

Forbidden transitions:

```text
WorkflowDefinition -> Runtime
RejectedWorkflow -> AcceptedArtifact
VerificationReport -> AcceptedArtifact
Raw IR -> Runtime admission
YAML source -> Runtime admission
Rust closure -> Runtime admission
```

Production runtime must expose no `run_yaml`, `submit_workflow_definition`, `submit_unverified_ir`, or `run_rust_closure` function.

---

