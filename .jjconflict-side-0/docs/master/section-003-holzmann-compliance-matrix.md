---
section: 3
title: "Holzmann Compliance Matrix"
parent: velvet-ballistics-MASTER.md
---

## 3. Holzmann Compliance Matrix


| Holzmann rule | `velvet-ballistics` build contract |
|---------------|-------------------------------------|
| Simple control flow | Runtime transitions are explicit `StepIdx -> StepIdx`; no hidden graph mutation after compile. |
| Bounded loops | `for_each`, `collect`, `reduce`, `repeat`, retries, scheduler ticks, trace rings, storage batches, IPC frames, and expression stacks require explicit limits. |
| No dynamic allocation after init where avoidable | Current turbo-style backend paths preallocate or reserve frames, slots, step states, stacks, queues, trace rings, journal buffers, and IPC buffers before run admission. |
| Short functions | Hot functions must be <= 25 logical lines. Complex cold validation phase functions must be decomposed or carry a bead-linked justification and must stay out of hot paths. CI and Moon tasks must include a source-length gate that fails hot functions over 25 logical lines. |
| Assertions/contracts | User errors return typed errors. Debug assertions may check compiler invariants that are unreachable for validated IR. |
| Small scopes | Each run belongs to exactly one shard. Shards own mutable runtime state. No global mutable run map. |
| Checked parameters/returns | Parse, validate, compile, eval, storage, IPC, action dispatch, and scheduler return typed `Result`. |
| Restricted macros | No macro-hidden business logic in current backend crates. Codegen work is removed from current scope and cannot be used as release evidence. |
| Restricted pointer complexity | No first-party pointer manipulation. Tables are addressed by checked numeric IDs. |
| Zero warnings | CI denies first-party warnings, clippy violations, forbidden constructs, and missing benchmark metadata. Advisory dependency/supply-chain/API report warnings do not block release under the owner waiver unless a specific bead opts in. |

---
