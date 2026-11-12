---
section: 38
title: "Property Tests"
parent: velvet-ballistics-MASTER.md
---

## 38. Property Tests


Required proptest coverage areas:

| Property | Description |
|----------|-------------|
| Constant folding | Constant expressions fold to identical result as runtime evaluation |
| Bytecode/AST parity | Compiled bytecode produces same result as AST interpretation |
| Digest stability | Same input produces same compiled digest |
| Layout stability | Slot layout and accessor layout stable for same workflow |
| Replay determinism | Journal replay produces identical run state |
| Snapshot equivalence | Snapshot + tail replay equals full journal replay |
| Ordering invariants | `for_each` output order matches input order; `together` output order matches YAML order |
| Bound enforcement | Retry attempts never exceed limit; collect never exceeds page/item/time limits |
| State machine | No terminal state transitions back to running |
| Taint safety | Secret taint never enters finish result (at compile time) |

---
