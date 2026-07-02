# Static Scan Report: vb-core-ipc-sync-evidence

updated_at: 2026-05-15T20:16:25Z
proof-writer: v1.0.1

## Scope

- Obligations: PO-017, PO-018.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`.
- Source writes: none.

## PO-017 Fanout And Buffer Scan

Command:

```text
rtk grep -n "tokio::spawn|std::thread::spawn|spawn\(|unbounded|Vec::new|Vec<|channel::unbounded" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src
```

Result: exit=0; `465 matches in 95 files`.

Classification summary:

- `tokio::spawn`, `std::thread::spawn`, and `channel::unbounded`: no surfaced matches in the scoped command output.
- `Vec` matches dominate the result set. Many are bounded by workflow/core limits, typed response caps, fixed IPC payload limits, or tests, but proof-writer did not claim exhaustive per-match closure for all 465 matches.
- IPC read buffering is bounded by `IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT` in `crates/vb_ipc/src/server/helpers.rs:28-38`.
- IPC client count is bounded by `MAX_CLIENTS = 256` and `ArrayVec<(Token, bool, bool), MAX_CLIENTS>` in `crates/vb_ipc/src/server/impl_.rs:45,96`.
- IPC write buffering remains bounded by serialized response size for a single response, with partial writes draining `write_buffer` in `crates/vb_ipc/src/server/helpers.rs:120-145` and `crates/vb_ipc/src/server/impl_.rs:241-258`. Slow-client connection-level eviction/backpressure beyond this single-response bound needs downstream production-connected test coverage.

Status: PARTIAL_PASS_WITH_BLOCKERS.

Blocker:

- PO-017 requires every match classified. This proof-writer pass records category-level classification only; exhaustive 465-match audit should be completed by formal-verifier/reviewer or a dedicated static-scan state.

## PO-018 YAML/JSON/HTTP Scan

Command:

```text
rtk grep -n "serde_json|serde_yaml|yaml|json|http|hyper|reqwest|axum|tonic" Cargo.toml crates/vb_ipc crates/vb_runtime crates/vb_core crates/vb_storage
```

Result: exit=0; `46 matches in 19 files`.

Classification summary:

- `Cargo.toml` contains workspace dependencies and `vb_yaml` entries. This is not by itself hot runtime-core execution evidence.
- Surfaced source matches are comments, tests, monotonicity text false positives, IPC wire comments, and capability names such as `network.http` in tests.
- No surfaced match showed direct `serde_json`, `serde_yaml`, `hyper`, `reqwest`, `axum`, or `tonic` execution in the scoped hot IPC/runtime/core/storage path.

Status: PARTIAL_PASS_WITH_BLOCKERS.

Blocker:

- PO-018 still needs reviewer-grade exhaustive classification of all 46 matches before final closure.
