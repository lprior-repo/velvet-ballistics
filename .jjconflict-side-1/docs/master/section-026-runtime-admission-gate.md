---
section: 26
title: "Runtime Admission Gate"
parent: velvet-ballistics-MASTER.md
---

## 26. Runtime Admission Gate

A run is not admitted until `RunAccepted` is recorded according to durability policy.

Admission flow:

```text
load accepted artifact by digest
verify artifact digest and schema version
verify action ABI digest matches loaded action registry
verify policy digest is allowed
validate input schema and size
check required capabilities against operator grants
check required secrets are available
reserve/preallocate runtime resources
record RunAccepted
return SubmitReceipt
```

Submission API:

```rust
pub struct SubmitOptions {
    pub durability: Durability,
    pub capability_grants: CapabilityGrants,
    pub idempotency_key: SubmitIdempotencyKey,
}

pub struct SubmitReceipt {
    pub run_id: RunId,
    pub artifact_digest: ArtifactDigest,
    pub seq: SeqNo,
    pub durability: DurabilityEvidence,
}
```

Production admission rejects:

```text
raw workflow definitions
raw IR
unverified artifacts
artifact/action ABI mismatch
artifact/policy mismatch
missing secrets
missing capabilities
undeclared grants under strict-exact mode
unbounded input
resource reservation failure
```

---

