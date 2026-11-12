---
section: 16
title: "Idempotency Key Expression Language"
parent: velvet-ballistics-MASTER.md
---

## 16. Idempotency Key Expression Language

Idempotency keys are AST values, not arbitrary Rust strings.

Allowed:

```rust
key!("github.issue_create", input.repo, input.ticket_id)
key!(workflow_digest(), step_id(), loop_index())
key!(artifact_digest(), action_id(), trigger_unique_key())
```

Forbidden:

```rust
format!("{}-{}", input.ticket_id, now())
rand::random()
std::env::var("USER")
secrets.github_token
attempt.number
current_time()
```

Internal representation:

```rust
pub enum KeyPart {
    Literal(SymbolId),
    WorkflowDigest,
    ArtifactDigest,
    RunId,
    StepIdx,
    ActionId,
    LoopIndex,
    TriggerUniqueKey,
    InputField(AccessorIdx),
    StableOutputField { step: StepIdx, accessor: AccessorIdx },
}

pub struct IdempotencyKeyExpr {
    pub parts: Box<[KeyPart]>,
    pub digest: Digest,
}
```

The compiler canonicalizes key expressions and records their digest in the artifact. Runtime carries a fixed-size key digest, not raw key material.

---

