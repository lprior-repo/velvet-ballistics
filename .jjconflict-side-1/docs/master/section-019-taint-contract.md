---
section: 19
title: "Taint Contract"
parent: velvet-ballistics-MASTER.md
---

## 19. Taint Contract

Taint lattice:

```text
Clean < DerivedFromSecret < Secret
```

The compiler and runtime track direct data flow:

```text
Copy preserves taint.
Expression output joins loaded operand taints.
Object/list construction joins field/item taints.
Action output taint must be at least as restrictive as input taint unless declassification is declared and verified.
Finish preserves result taint.
Diagnostics redact secret-tainted details.
```

Control-flow taint:

```text
v1 full control-flow taint tracking: not implemented
explicit secret condition rejection: available under strict policy
full absence of implicit secret leaks proven: false
```

Certificate shape:

```rust
pub struct TaintCertificate {
    pub direct_data_flow_tracked: bool,
    pub result_direct_taint: Taint,
    pub control_flow_tracked: bool,
    pub explicit_secret_conditions_rejected: bool,
    pub full_absence_of_implicit_leaks_proven: bool,
}
```

Do not call a workflow `secret safe` unless the certificate states exactly what was checked.

---

