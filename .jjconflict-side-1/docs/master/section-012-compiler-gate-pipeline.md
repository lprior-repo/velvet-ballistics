---
section: 12
title: "Compiler Gate Pipeline"
parent: velvet-ballistics-MASTER.md
---

## 12. Compiler Gate Pipeline

The compiler gate pipeline is mandatory:

```text
Rust SDK macro parse
  -> SDK grammar validation
  -> schema derivation validation
  -> action manifest resolution
  -> name/scope validation
  -> expression compilation
  -> key-expression compilation
  -> control-flow validation
  -> type checking
  -> effect checking
  -> boundedness analysis
  -> resource budget checking
  -> idempotency verification
  -> capability verification
  -> secret availability declaration validation
  -> direct data-flow taint analysis
  -> durability proof construction
  -> observability/reportability check
  -> accepted artifact emission
```

A workflow must pass every gate. Policy may promote warnings to errors. The artifact records the exact policy digest used.

---

