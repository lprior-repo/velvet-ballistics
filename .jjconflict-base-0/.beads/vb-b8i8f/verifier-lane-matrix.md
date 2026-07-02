# Verifier Lane Matrix: vb-b8i8f

## Lane Applicability Matrix

| Proof Seed | Verus | Kani | Flux-rs | proptest | Loom | Miri | cargo-fuzz |
|------------|-------|------|---------|----------|------|------|------------|
| vb-b8i8f-seed-001 (live-only) | required | required | required | required | not_applicable | not_applicable | not_applicable |
| vb-b8i8f-seed-002 (single-terminal) | required | required | required | required | not_applicable | not_applicable | not_applicable |
| vb-b8i8f-seed-003 (stale-authority) | required | required | required | required | not_applicable | not_applicable | not_applicable |
| vb-b8i8f-seed-004 (kind28-admission) | required | required | required | required | not_applicable | not_applicable | required |
| vb-b8i8f-seed-005 (replay-ordinal) | required | required | required | required | not_applicable | not_applicable | required |

## Applicability Legend

- **required**: Mandatory verifier lane per default Rust profile or conditional risk trigger.
- **not_applicable**: Lane does not apply; concrete evidence provided in lane decision.
- **blocked_tooling**: Tool is unavailable; blocks proof closure.

## Non-Applicability Evidence Summary

| Lane | Reason | Evidence Ref |
|------|--------|-------------|
| Loom (all seeds) | No concurrency, atomics, channels, locks, async shutdown, or task ownership. Shard processing is single-threaded via command queue; cancel/kill handlers run synchronously within Shard::tick. | `crates/vb_runtime/src/runtime.rs:198` (tick_all processes one command per shard per tick); no `tokio`, `crossbeam`, `std::sync::mpsc`, or `Arc<Mutex<>>` in handler scope. |
| Miri (all seeds) | All touched files carry `#![forbid(unsafe_code)]`; zero unsafe blocks, no FFI, no raw pointers, no MaybeUninit, no provenance-sensitive operations in codec validation, records, events, runtime, or journal mapping. | `crates/vb_runtime/src/runtime.rs:1`, `crates/vb_storage/src/records.rs:1`, `crates/vb_storage/src/events.rs:1`; codec validation.rs uses safe `matches!` and `match` on primitives only. |
| cargo-fuzz (seeds 001-003) | No parsers, codecs, binary/persisted payloads, or hostile input boundaries in the public API routing, shard lifecycle handlers, or stale authority checks. | Cancel/kill routing uses typed `RunId` and typed `ShardCommand` enums, not raw bytes. |
