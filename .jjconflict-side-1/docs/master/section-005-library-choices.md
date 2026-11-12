---
section: 5
title: "Library Choices"
parent: velvet-ballistics-MASTER.md
---

## 5. Library Choices


| Library | Purpose | Constraint |
|---------|---------|------------|
| `saphyr-parser` | Strict YAML event parsing | Cold path only. |
| `postcard` | Compact binary records | Required for journal, snapshots, IPC payloads, compiled artifacts. |
| `fjall` | Embedded LSM persistence | Required storage engine. |
| `thiserror` | Typed errors | Public errors must stay typed and stable. |
| `byteorder` | Little-endian binary boundary helpers | Allowed for IPC/header/record field encode/decode only. Fjall keys remain explicit big-endian byte layouts for lexicographic ordering. |
| `bytes` | Payload and blob sharing | Handles only in hot runtime state. |
| `arrayvec` | Fixed-capacity expression stacks and bounded scratch buffers | Hot path allowed when capacity is explicit. |
| `logos` | Expression lexer state machine | Compile-time/cold-path lexer only. Must preserve exact spans, diagnostics, token limits, and fuzz coverage. No runtime execution path dependency. |
| `indexmap` | Deterministic object field side indexes | Cold `ValueStore` object lookup side table only. `SlotValue` remains handle-only; insertion order and duplicate-key behavior must remain stable. |
| `crossbeam-queue::ArrayQueue` | Bounded MPMC shard queues | No unbounded channel replacement. |
| `rtrb` | SPSC ring buffers and trace/action completion paths | Capacity required at construction. |
| `mio` | Low-level IPC event loop | No HTTP server, no JSON routing. |
| `criterion` | Statistical benchmarks | Required for local performance claims. |
| `iai-callgrind` | Instruction/cache benchmarks | Required for CI performance gates. |
| `proptest` | Property tests | Required for invariants. |
| `cargo-fuzz` | Fuzzing | Required for parsers/decoders. |
| `trybuild` | Compile-fail tests | Required only for active public macro/schema contracts in the current milestone; generated Rust contracts are removed. |
| `cargo-nextest` | Test execution | Required CI test runner. |
| `cargo-audit` | Vulnerability scan | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-deny` | Policy scan | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-vet` | Supply-chain review | Advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-geiger` | Unsafe dependency scan | Advisory report; first-party unsafe remains forbidden by lint. |
| `blake3` | Digest computation for envelopes and artifacts | Required for compiled digests, journal digests, blob digests. |
| `crc32c` | CRC32C header checksum for binary envelopes | Required for envelope header integrity. |

`crossbeam-queue::ArrayQueue` is required for bounded MPMC queues because capacity is fixed at construction and admission can fail without allocating. `rtrb` is required for SPSC trace/action rings where single-producer/single-consumer ownership gives predictable bounded behavior.

`serde` is allowed only for deriving binary/data schema serialization used by Postcard or cold diagnostics. `serde_json` is excluded from v1 runtime core.

`ordered-float` is not approved as the v1 `FiniteF64` implementation. `ordered_float::NotNan<f64>` rejects NaN but permits positive and negative infinity, while this language requires finite-only scalar values. Any future replacement must prove release-mode rejection of NaN and infinities, unchanged serialized representation, no panic/unwrap path, and no larger transitive footprint than the custom newtype.

---
