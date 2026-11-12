---
section: 11
title: "Accepted Artifact Contract"
parent: velvet-ballistics-MASTER.md
---

## 11. Accepted Artifact Contract

An accepted artifact is the only deployable unit.

```rust
pub struct AcceptedArtifactHeader {
    pub artifact_schema_version: u16,
    pub workflow_language_version: u16,
    pub artifact_digest: ArtifactDigest,
    pub source_digest: SourceDigest,
    pub ir_digest: IrDigest,
    pub action_abi_digest: ActionAbiDigest,
    pub policy_digest: PolicyDigest,
    pub resource_budget_digest: ResourceBudgetDigest,
    pub verification_certificate_digest: CertificateDigest,
}
```

Artifact payload contains:

```text
compiled numeric IR
constant table
accessor table
expression bytecode table
action table
source metadata side table
resource contract
whole-workflow budget
capability requirements
secret requirements
idempotency certificates
taint certificate
durability certificate
schema digests
verification certificate
```

Every artifact is encoded with the standard binary envelope and Postcard payload. Decode order is mandatory:

```text
read fixed header
validate magic
validate schema version
validate record kind
validate payload length before allocation
validate header CRC32C
read exact payload bytes
validate BLAKE3 payload digest
Postcard-decode typed payload
validate artifact cross-digests
validate IR structure
```

---

