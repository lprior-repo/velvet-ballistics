---
section: 17
title: "Capabilities and Grants"
parent: velvet-ballistics-MASTER.md
---

## 17. Capabilities and Grants

Required capabilities and granted capabilities are distinct types.

```rust
pub struct RequiredCapabilities {
    inner: Box<[Capability]>,
}

pub struct CapabilityGrants {
    inner: Box<[Capability]>,
}
```

The artifact declares requirements. The operator supplies grants. Runtime admission checks policy:

```text
strict-exact: required == grants
subset: required ⊆ grants, runtime exposes only required capabilities to the run
```

The default for `strict_ai` is `strict-exact`. A broader operator profile may use subset mode only if the runtime passes only declared capabilities into the run.

Forbidden SDK shape:

```rust
SubmitOptions { capability_grants: artifact.required_capabilities() }
```

Required SDK shape:

```rust
SubmitOptions {
    durability: Durability::Strict,
    capability_grants: operator_grants,
}
```

Admission failure returns `CapabilityDenied` with missing and undeclared grants separated.

---

