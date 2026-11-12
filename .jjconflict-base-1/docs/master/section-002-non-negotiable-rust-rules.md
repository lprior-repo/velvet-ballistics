---
section: 2
title: "Non-Negotiable Rust Rules"
parent: velvet-ballistics-MASTER.md
---

## 2. Non-Negotiable Rust Rules


First-party Rust code under this workspace must satisfy these rules on every change:

- `#![forbid(unsafe_code)]` in every first-party crate.
- No `unsafe` blocks, traits, functions, impls, or FFI in first-party code.
- No `.unwrap()`.
- No `.expect()`.
- No `panic!`.
- No `todo!`, `unimplemented!`, or `dbg!`.
- No unchecked indexing with `[]`.
- No unchecked slicing.
- No unchecked `as` casts.
- No unchecked arithmetic, offset math, capacity math, or length math.
- No ignored `Result` or ignored fallible return value.
- No unbounded queues, loops, retries, fanout, buffers, task spawning, timers, pagination, or expression stacks.
- No YAML interpretation during run execution.
- No JSON in the runtime core.
- No HTTP in the runtime core.
- No dynamic string lookup for references during execution.
- No `HashMap<String, Value>` runtime state.
- No task-per-step scheduler.
- No formatted text output inside hot execution loops.

Dependency rule: third-party crates may contain internal unsafe only if pinned and justified by the repository dependency policy. `cargo-geiger`, `cargo-vet`, `cargo-deny`, and related supply-chain tools remain advisory reports under the 2026-05-23 owner waiver; their warnings do not block the current Backend / IR Interpreter Complete milestone unless a specific bead opts back into blocking enforcement.

---
