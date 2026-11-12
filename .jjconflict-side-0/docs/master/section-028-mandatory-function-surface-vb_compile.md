---
section: 28
title: "Mandatory Function Surface: `vb_compile`"
parent: velvet-ballistics-MASTER.md
---

## 28. Mandatory Function Surface: `vb_compile`


**Source of truth:** `crates/vb_compile/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Entry point | `compile_workflow`, `YamlCompiler::compile`, `parse_ast`. |
| Slot compilation | Slot layout, accessor table, constant pool construction. |
| Lowering | Per-primitive lowering (set, do, choose, for_each, together, collect, reduce, repeat, wait, ask, finish). |
| Validation | Schema, reference, control flow, type-taint validation integrated into compile pipeline. |
| Expression | Expression compilation with reference resolution to `SlotIdx`. |
| Output | Digest computation, compiled artifact emission. |

---
