# Wave 5 — Agent 13: Ad-hoc IPC Frame Expert

**Date:** 2026-06-24
**Bug chunk:** `/tmp/wave5-chunk-13.txt` (4 IDs)
**Working dir:** `/home/lewis/src/velvet-ballistics` (verified `git rev-parse --show-toplevel`)
**Domain:** IPC magic validation, payload length validation, command set, server buffering

## Chunk bug summary

None of the four bug IDs (`vb-wyixc`, `vb-xezc0`, `vb-y3az6`, `vb-y675j`) are
IPC bugs. Per `bd show`:

| bug-id | domain | master / scope |
|--------|--------|----------------|
| vb-wyixc | `vb_core::ValueStore::insert_symbol` (CF-005) — string interning | core frame |
| vb-xezc0 | workspace rename `velvet_ballistics_workspace_tests` → `vb_workspace_tests` | build hygiene |
| vb-y3az6 | `vb_storage::FrameSeedAccumulator` split | storage |
| vb-y675j | `vb_runtime::retry_math` saturating cursor (RE-012) | runtime |

All four are CLOSED. Ad-hoc IPC inspection of `crates/vb_ipc/src/` was
performed regardless; the IPC state is identical for every bug ID in this
chunk, so per-bug verdicts reflect that uniform state.

## Master contract (canonical)

`velvet-ballistics-MASTER.md` §21 (lines 1037-1097):

- Magic `0x56424C54` ("VBLT"), little-endian.
- 11 IPC commands (wire IDs `1..=11`): `SubmitRun, SubmitRunInline, CancelRun, InspectRun, ListEvents, AnswerAsk, CompleteAction, FailAction, DrainTrace, Health, Shutdown`.
- Reserved range `12..=16` (`ListRuns, GetMetrics, GetWorkflowGraph, GetTaintReport, VerifyWorkflow`) MUST decode as `UnknownCommand`.
- §21.1: "Validate magic before allocation."
- §21.2: "Validate payload length against configured maximum before reading payload."

`velvet-ballistics-MASTER.md` §50 (lines 2565-2614):

- "Backpressure: Bounded command queue (`ArrayQueue`). Queue full → `IpcError::Full` (E3001)."
- Library allowlist (lines 212-228): `crossbeam-queue::ArrayQueue` is required for bounded MPMC shard queues; SPSC ring is via `rtrb::RingBuffer`.

## Verdict table

| bug-id | pri | magic-pre-allocation | payload-pre-read | command-set | spsc-arrayqueue | targeted-cmd | result | verdict | evidence |
|--------|-----|---------------------|------------------|-------------|-----------------|--------------|--------|---------|----------|
| vb-wyixc | P0 | PASS (server `validate_magic_early` caps read buffer at 4 bytes before grow; `IpcFrameHeader::decode` checks magic first) | PASS (`IpcFrameHeader::decode` checks `payload_len > max_payload` at frame_types.rs:106-112 before any payload read; `append_read_bytes` caps buffer at `IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT`) | PASS (exactly 11 variants in `commands.rs:12-37`, wire IDs 1-11, no `ListRuns`/`GetMetrics`/etc.) | FAIL (`MemoryIngress::bounded` at `ingress.rs:77` uses `crossbeam_channel::bounded`; `Cargo.toml:11` has `crossbeam-channel` not `crossbeam-queue`; `queue/mod.rs` is a comment-only stub acknowledging MAJOR-1 migration pending) | `cargo test -p vb_ipc --lib server_disconnects_invalid_magic_without_response` → 1 passed | mixed | PARTIAL | `frame_types.rs:67-77,106-112`; `server/impl_.rs:200-217`; `server/helpers.rs:14-15,33-45,65-70`; `frame.rs:69-85,126-133`; `commands.rs:12-37`; `ingress.rs:5,77`; `queue/mod.rs:1-11`; `tests/ipc_command_properties.rs` (5/5 pass) |
| vb-xezc0 | P0 | PASS (same as above) | PASS (same as above) | PASS (same as above; tests in `tests/ipc_command_properties.rs` cover 1-11 and 12-16 → UnknownCommand) | FAIL (same as above) | `cargo test -p vb_ipc --test ipc_command_properties` → 5 passed | mixed | PARTIAL | `commands.rs:41-56`; `tests/ipc_command_properties.rs:13-37`; `error.rs:38-39` (`UnknownCommand(u16)` variant) |
| vb-y3az6 | P0 | PASS (same as above) | PASS (same as above; `slow_client_oversized_frame_disconnects_without_unbounded_growth` PASSES) | PASS (same as above) | FAIL (same as above) | `cargo test -p vb_ipc --lib slow_client_oversized_frame_disconnects_without_unbounded_growth` → 1 passed | mixed | PARTIAL | `frame_types.rs:106-112`; `server/helpers.rs:65-70`; `server/impl_tests.rs:363-401` (test asserts `read_buffer.len() == IPC_HEADER_LEN + 4` not `payload_len_max`) |
| vb-y675j | P2 | PASS (same as above) | PASS (same as above) | PASS (same as above) | FAIL (same as above) | `cargo test -p vb_ipc --lib server_responds_with_error_for_unsupported_version` → 1 passed | mixed | PARTIAL | `frame_types.rs:67-120`; `server/impl_tests.rs:404-436`; `commands.rs:14-34` |

## Targeted test runs (verbatim tail)

```
$ cargo test -p vb_ipc --lib server_disconnects_invalid_magic_without_response --no-fail-fast
running 1 test
test server::impl_tests::server_disconnects_invalid_magic_without_response ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 539 filtered out

$ cargo test -p vb_ipc --lib slow_client_oversized_frame_disconnects_without_unbounded_growth --no-fail-fast
running 1 test
test server::impl_tests::slow_client_oversized_frame_disconnects_without_unbounded_growth ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 539 filtered out

$ cargo test -p vb_ipc --test ipc_command_properties --no-fail-fast
running 5 tests
test test_as_u16_in_valid_range ... ok
test test_exactly_eleven_variants ... ok
test test_removed_commands_return_unknown_command ... ok
test test_eleven_commands_parse_ok ... ok
test test_roundtrip_all_commands ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p vb_ipc --lib server_responds_with_error_for_unsupported_version --no-fail-fast
running 1 test
test server::impl_tests::server_responds_with_error_for_unsupported_version ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 539 filtered out
```

## Per-property findings

### 1. Magic pre-allocation — PATCHED

- `crates/vb_ipc/src/server/helpers.rs:15` — `pub const AWAITING_MAGIC_MAX_BYTES: usize = 4;` (cap = magic width).
- `crates/vb_ipc/src/server/helpers.rs:33-45` — `validate_magic_early` slices the first 4 bytes only; no allocation; returns `InvalidMagic` or `MagicValidated` without growing any buffer.
- `crates/vb_ipc/src/server/impl_.rs:200-217` — the server invokes `validate_magic_early` only when `client.read_buffer.len() >= AWAITING_MAGIC_MAX_BYTES`, i.e. magic check happens before any further buffer growth.
- `crates/vb_ipc/src/server/impl_.rs:219-222` — frame decode is gated on `client.magic_state == MagicValidated`.
- `crates/vb_ipc/src/frame_types.rs:67-77` — `IpcFrameHeader::decode` reads magic first, returns `IpcError::InvalidMagic` before any other decode step.
- Kani harness `crates/vb_ipc/src/kani_ipc_decode_order.rs:21-74` (`kani_harness_ipc_decode_order`) and `:131-167` (`kani_harness_ipc_magic_before_version`) prove magic precedes version check for arbitrary 24-byte inputs.
- Kani harness `crates/vb_ipc/src/kani_ipc_header.rs:43` (`kani_ipc_header_rejects_bad_magic`) covers the bad-magic rejection.

### 2. Payload length pre-read — PATCHED

- `crates/vb_ipc/src/frame_types.rs:103-112` — `IpcFrameHeader::decode` converts `payload_len` to `usize` (line 106), then bounds-checks against `max_payload.get()` (line 107) and returns `PayloadTooLarge` BEFORE the caller's buffer is grown for payload bytes.
- `crates/vb_ipc/src/frame.rs:69-85` — `validate_frame_bounds` re-checks bounds before any payload read.
- `crates/vb_ipc/src/frame.rs:126-133` — `read_frame_payload_bounded` calls `validate_frame_bounds` first then `read_frame_payload`.
- `crates/vb_ipc/src/server/helpers.rs:48-73` — `append_read_bytes` rejects any push that would exceed `IPC_HEADER_LEN + MaxPayloadBytes::DEFAULT`; returns `ReadBufferTooLarge` (typed).
- `crates/vb_ipc/src/server/impl_.rs:243-246` — server returns `Ok(false)` ("need more bytes") if `client.read_buffer.len() < total_len`, never grows to declared payload.
- Test `server::impl_tests::slow_client_partial_frame_keeps_read_buffer_bounded` asserts the buffer stays at `IPC_HEADER_LEN` (not `IPC_HEADER_LEN + payload_len`) when only the header has arrived.
- Test `server::impl_tests::slow_client_oversized_frame_disconnects_without_unbounded_growth` asserts the server disconnects (not allocates) when the declared payload exceeds `MaxPayloadBytes::DEFAULT`.
- Kani harness `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` covers the oversize rejection path.

### 3. Command set — PATCHED (matches master §21 exactly)

- `crates/vb_ipc/src/commands.rs:12-37` — `IpcCommand` enum has exactly 11 named variants (wire IDs 1-11) plus the `UnknownCommand(u16)` reserved catch-all.
- `crates/vb_ipc/src/commands.rs:41-56` — `from_u16` returns `Ok(UnknownCommand(other))` for any wire ID `>= 12`, including the explicitly reserved `12..=16` range.
- No references to `ListRuns`, `GetMetrics`, `GetWorkflowGraph`, `GetTaintReport`, or `VerifyWorkflow` in `crates/vb_ipc/src/`.
- `tests/ipc_command_properties.rs` (5 tests) covers parse, roundtrip, range, and 12-16 rejection — all PASS.
- `tests/proptest_ipc_error_codes.rs` covers the `UnknownCommand` diagnostic `0x3006` (E3006).

### 4. SPSC ArrayQueue — NOT-PATCHED (architectural drift)

This is the one IPC property that the task brief asserts as required
("ArrayQueue for IPC SPSC … crossbeam_channel FORBIDDEN") but is NOT
present in the codebase.

- `crates/vb_ipc/Cargo.toml:11` — `crossbeam-channel.workspace = true` is the queue dependency. There is NO `crossbeam-queue` dependency and NO `rtrb` dependency.
- `crates/vb_ipc/src/ingress.rs:5` — `use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};`
- `crates/vb_ipc/src/ingress.rs:77` — `let (sender, receiver) = crossbeam_channel::bounded(capacity.get());` — production `MemoryIngress::bounded` is built on `crossbeam_channel`, not `crossbeam_queue::ArrayQueue`.
- `crates/vb_ipc/src/ingress.rs:117` — `disconnect_sender` test hook also re-allocates a `crossbeam_channel::bounded(1)`.
- `crates/vb_ipc/src/tests.rs:291,296,297,1522` — additional test-side `crossbeam_channel` usage.
- `crates/vb_ipc/src/queue/mod.rs:1-11` — module body is a documentation comment ONLY; the comment explicitly states "once `ArrayQueue<T, RingFlagged>` replaces it, the same assertions must hold" — confirming the migration is acknowledged but not performed.
- `crates/vb_ipc/src/queue/tests/array_queue_tests.rs:1-913` — 913 lines of BDD/property tests, but NOT compiled: `queue/mod.rs` has no `#[cfg(test)] mod tests;` (or any `mod tests;` include), so `cargo test --lib -p vb_ipc` reports `running 0 tests` for the `memory_ingress_*` tests. The tests are dead code, not evidence.
- Master §50 also says the SERVER should have a "Bounded command queue (`ArrayQueue`)" for backpressure; the current server (`server/impl_.rs`) dispatches per-connection via `dispatch_command_with_resolver` and has NO central command queue, so §50 backpressure contract is also unmet at the server level.

Note: the task brief paraphrases master §50 as "ArrayQueue for IPC SPSC",
but master actually calls §50's command queue an MPMC shard queue
("Bounded MPMC shard queues" in the library table at line 212; SPSC
ring is `rtrb::RingBuffer` at line 213). The drift is real regardless of
which library the master ultimately intends.

## Magic-after-allocation cases (none)

No cases found where the server allocates payload-shaped memory before
validating magic. The `AWAITING_MAGIC_MAX_BYTES = 4` cap is strictly
less than the minimum frame size (`IPC_HEADER_LEN = 24`), so any
buffer past 4 bytes must have passed the magic check first.

## Command-set drift (none)

No drift detected. The `IpcCommand` enum is exactly the 11 commands
master §21 mandates; reserved IDs `12..=16` decode as
`UnknownCommand(u16)` and are rejected by dispatch (the dispatch
table at `server/dispatch.rs` is keyed on the 11 named variants only).

## Top NOT-PATCHED with reason

1. **`MemoryIngress` still backed by `crossbeam_channel` instead of `crossbeam-queue::ArrayQueue`** — `crates/vb_ipc/src/ingress.rs:5,77,117` import and instantiate `crossbeam_channel::bounded`. Master §50 plus the AGENTS.md "ArrayQueue for IPC SPSC … crossbeam_channel FORBIDDEN" requirement mandate the migration. The migration is acknowledged in `crates/vb_ipc/src/queue/mod.rs:1-11` ("MAJOR-1") and 913 lines of BDD tests are staged in `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`, but the production struct still uses `crossbeam_channel`. The staged test file is not even compiled by `cargo test --lib` (no `mod tests;` in `queue/mod.rs`).

2. **No central server command queue (master §50 backpressure not wired at server level)** — `crates/vb_ipc/src/server/impl_.rs` accepts up to `MAX_CLIENTS = 256` and dispatches per-connection via `dispatch_command_with_resolver` directly. Master §50 says "Backpressure: Bounded command queue (`ArrayQueue`)" for the server's command admission path. The current server applies per-connection buffer caps (`append_read_bytes` → `ReadBufferTooLarge`) but does not throttle submission rate into the runtime via an `ArrayQueue`.

3. **`queue/mod.rs` is a comment-only module** — `crates/vb_ipc/src/queue/mod.rs:1-11` is 11 lines of `//!` documentation. There is no `pub mod queue;` body, no `mod tests;`, and no production struct. This means the staged `array_queue_tests.rs` (913 lines) is unreachable from any test binary. The migration deliverable is half-done: tests are written but neither wired into the build nor backed by the intended `ArrayQueue` implementation.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-13-adhoc-ipc-frame.md`
