---
section: 37
title: "Rust/JPL Mechanical Rules"
parent: velvet-ballistics-MASTER.md
---

## 37. Rust/JPL Mechanical Rules

First-party code follows the Rust adaptation of Holzmann/JPL-style discipline.

Workspace requirements:

```text
#![forbid(unsafe_code)] in first-party crates
no unsafe
no unwrap
no expect
no panic
no todo/unimplemented/dbg
no unchecked indexing/slicing
no unchecked casts
no unchecked arithmetic in production paths
no ignored Result
no unbounded loops/queues/retries/fanout/buffers
no dynamic allocation after run admission in hot runtime
no task per step
no async runtime in core runtime/storage/IPC
no runtime JSON/YAML/HTTP in core runtime/storage/IPC
```

JPL mapping:

| Rule family | Velvet enforcement |
|---|---|
| simple control flow | tiny numeric IR, explicit decisions, no hidden graph mutation |
| bounded loops | compiler-proven bounds and runtime budget checks |
| no dynamic allocation after init | admission reservation and arena caps |
| short hot functions | zone-aware source scan and complexity cap |
| checked returns | `Result` everywhere, `#[must_use]` receipts |
| restricted macros | only approved SDK macros in workflow modules |
| restricted pointer complexity | no first-party unsafe/raw pointers |
| zero warnings | fmt/clippy/docs/tests gates |

---

