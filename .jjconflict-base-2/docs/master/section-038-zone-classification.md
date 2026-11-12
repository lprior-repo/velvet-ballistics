---
section: 38
title: "Zone Classification"
parent: velvet-ballistics-MASTER.md
---

## 38. Zone Classification

Every source file declares one zone:

```rust
// velvet-zone: hot-runtime
// velvet-zone: storage-decode
// velvet-zone: cold-compiler
// velvet-zone: sdk-authoring
// velvet-zone: action-executor
// velvet-zone: cli-operator
// velvet-zone: test
```

Hot runtime forbidden:

```text
allocation after admission
formatting
HashMap/BTreeMap/String lookup
async/await
network/filesystem/env/process access
serde_json
std::thread::spawn
unbounded Vec growth
unchecked indexing/slicing/arithmetic/casts
```

SDK authoring macro code may allocate and format diagnostics but must never enter runtime execution.

Action executor code may perform side effects only behind bounded worker/executor contracts and action tickets.

---

