---
section: 10
title: "Step Primitive Contract"
parent: velvet-ballistics-MASTER.md
---

## 10. Step Primitive Contract


Every YAML step has exactly one primitive:

```text
set · do · choose · for_each · together · collect · reduce · repeat · wait · ask · finish
```

**Canonical names.** The normative primitive names are `set · do · choose · for_each · together · collect · reduce · repeat · wait · ask · finish`. The implementation accepts these aliases:

| Alias | Canonical | Notes |
|-------|-----------|-------|
| `save` | `set` | Legacy alias in parser and compiler |
| `run` | `do` | Alternative step invocation |
| `foreach` | `for_each` | Single-word spelling in YAML parser |

These aliases are compiler-accepted; canonical names are preferred in authored YAML.

Control and metadata fields are not primitives:

```text
id · name · if · with · try_again · on_error · then
```

High-level YAML primitives may lower into multiple IR nodes. Runtime executes IR only in the current milestone.

---
