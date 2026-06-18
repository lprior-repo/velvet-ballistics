# Architecture Steal Plan

Generated 2026-05-24. This is a Master-filtered backlog source for `vb-8mdp`.

## Authority

`velvet-ballistics-MASTER.md` is the only architecture contract.

## Black Hat Fence

Do not copy external orchestrator code, module names, type names, async architecture, HTTP/gRPC/JSON paths, distributed control-plane assumptions, storage layout, or wire formats.

Identify behavioral risks that must be re-expressed through velvet-ballistics types, numeric IR, binary IPC, Fjall/Postcard storage, bounded queues, and single-server shard-owned execution.

## Accepted Steals

| Area | Accepted Invariant | Rejected Coupling |
|---|---|---|
| IPC | Fragmented headers/bodies, oversize rejection, reserved flag rejection, decode-order tests | HTTP/gRPC transport, JSON payload interpretation, distributed service routing |
| Storage | Budget-before-decode, fixed record envelopes, lexicographic key ordering, side-index consistency, corruption fixtures | RocksDB layout, implicit serde migrations, string hot keys |
| Runtime | Delayed-action timer admission, stale wake-up fencing, cancellation/kill outcome lattice, idempotency hydration, bounded resource reservation | Task-per-step async execution, HLC clocks, distributed partition ownership |
| Scheduler | FIFO/backpressure, DRR-style fairness, completion prefix watermarks | Tokio runtime architecture, unsafe atomics in core |
| CLI/Doctor | Cold-path bounded inspection, typed decode diagnostics, safe filters, replay/status views | Runtime JSON, hot-path formatting, shell-completion/config systems not in Master |

## Implementation Rule

Every bead derived from this document must state concrete VB files and public APIs to test, exact typed outcomes/errors, and exact command evidence. A test that only asserts `Behavior verified` is not evidence.
