---
section: 15
title: "Idempotency Verification Gate"
parent: velvet-ballistics-MASTER.md
---

## 15. Idempotency Verification Gate

Every retry of side-effecting work requires compiler-validated idempotency attestation.

The compiler can prove:

```text
the workflow contains an idempotency key expression
the key expression is deterministic
the key expression is bounded
the key expression does not reference secrets
the key expression does not reference time/random/env/process state
the key expression does not depend on attempt number unless policy explicitly allows it
the key expression matches the action's required scope
the action manifest declares retry behavior
the action ABI digest matches the certificate
```

The compiler cannot prove external provider honesty. External idempotency is therefore recorded as attestation, not mathematical proof.

```rust
pub enum IdempotencyVerdict {
    ProvenPure,
    KeyedAndAttested,
    NotRetried,
    Rejected,
}

pub struct IdempotencyCertificate {
    pub step: StepIdx,
    pub action: ActionId,
    pub side_effect: SideEffect,
    pub retry_safety: RetrySafety,
    pub key_expr_digest: Option<Digest>,
    pub key_scope: IdempotencyScope,
    pub verdict: IdempotencyVerdict,
}
```

Idempotency diagnostics must distinguish:

```text
missing key
secret in key
random/time/env in key
attempt number in key
wrong key scope
non-retry-safe action in retry region
unknown action retry safety
provider-native key not mapped
key uses unbounded value
```

---

