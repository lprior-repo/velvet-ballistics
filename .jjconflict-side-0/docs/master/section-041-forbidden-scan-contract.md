---
section: 41
title: "Forbidden-Scan Contract"
parent: velvet-ballistics-MASTER.md
---

## 41. Forbidden-Scan Contract

A `syn`-based scanner is mandatory.

Required commands:

```text
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
cargo xtask sdk-scan --changed
cargo xtask action-scan --changed
cargo xtask invariants
```

Scanner detects:

```text
unsafe
unwrap/expect/panic/todo/unimplemented/dbg
unchecked indexing/slicing/as casts
ignored Result
HashMap<String, _> in runtime
serde_json in runtime
HTTP crates in runtime
format! in hot path
Vec::push in hot path without budget/capacity pattern
std::thread::spawn in runtime
async fn in runtime/storage/IPC
std::env/std::fs/std::net in workflow DSL modules
arbitrary macro calls inside workflow definitions
YAML parser crates in active workflow authoring path
```

---

