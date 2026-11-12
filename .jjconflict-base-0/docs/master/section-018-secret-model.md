---
section: 18
title: "Secret Model"
parent: velvet-ballistics-MASTER.md
---

## 18. Secret Model

The compiler validates secret references. The artifact records secret requirements, not secret values. Runtime admission checks presence. Action workers receive opaque secret handles when authorized.

```rust
pub struct SecretRequirement {
    pub name: SymbolId,
    pub required_capability: Capability,
    pub allowed_actions: Box<[ActionId]>,
}

pub struct SecretHandle {
    pub secret: SecretRequirement,
    pub run: RunId,
    pub ticket: Option<ActionTicket>,
}
```

Rules:

```text
No raw secret bytes in artifacts.
No raw secret bytes in durable history.
No raw secret bytes in hot runtime frame.
No raw secret bytes in diagnostics.
No secret values in idempotency keys.
No secret values in public reports unless explicit unsafe operator flag exists.
Action workers resolve secret handles through a secret provider.
```

v1 tracks direct data-flow taint. v1 may reject explicit secret references in branch conditions under strict policy. v1 does not prove full control-flow taint safety.

---

