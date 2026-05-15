# Codebase Map: vb-0253.2

## Bead
- **id**: vb-0253.2
- **title**: ipc: Finish ingress modularization and dedupe
- **phase**: 2 (Explore and Scope)
- **updated_at**: 2026-05-15T00:00:00Z

## Scope Summary

Finish the modularization of the `vb_ipc` crate by converting `lib.rs` into a facade/re-export layer.
The split modules (`bounded.rs`, `ingress.rs`, `error.rs`) already hold the canonical definitions;
`lib.rs` currently contains verbatim duplicates of those definitions (lines 641–960).

## Canonical Module Layout

### `crates/vb_ipc/src/bounded.rs` (69 lines, `#![forbid(unsafe_code)]`)
Holds the single canonical definition of:
- `QueueCapacity` (pub struct, wraps `NonZeroUsize`, line 12)
- `MaxPayloadBytes` (pub struct, wraps `NonZeroUsize`, `DEFAULT = 1 MiB`, line 28)
- `BoundedPayload` (pub struct, wraps `Bytes`, line 49)
- `BoundedPayload::new(payload, max)` → `Result<BoundedPayload, IpcError>` (parse-don't-validate, line 53)

### `crates/vb_ipc/src/ingress.rs` (97 lines, `#![forbid(unsafe_code)]`)
Holds the single canonical definition of:
- `IngressFrame` (pub struct, fields: `run_id`, `workflow`, `payload: BoundedPayload`, line 14)
- `IngressFrame::new(run_id, workflow, payload, max_payload)` → `Result<Self, IpcError>` (line 22)
- `MemoryIngress` (pub struct, wraps `Sender<IngressFrame>` + `Receiver<IngressFrame>`, line 56)
- `MemoryIngress::bounded(capacity: QueueCapacity)` → `Self` (line 64)
- `MemoryIngress::try_submit(frame)` → `Result<(), IpcError>` (line 70, maps `TrySendError::Full → IpcError::Full`, `Disconnected → IpcError::Disconnected`)
- `MemoryIngress::try_recv()` → `Result<Option<IngressFrame>, IpcError>` (line 78, maps `TryRecvError::Disconnected → IpcError::Disconnected`)
- `MemoryIngress::len()` → `usize` (line 88)
- `MemoryIngress::is_empty()` → `bool` (line 94)

### `crates/vb_ipc/src/error.rs` (165 lines, `#![forbid(unsafe_code)]`)
Holds the single canonical definition of:
- `IpcError` (pub enum with 14 variants, line 9)
- `IpcError` impl block with `diagnostic_code()` and `runtime_code()` methods (lines 76–153)
- `u32_to_usize(value: u32)` → `Result<usize, IpcError>` (pub(crate), line 160)

### `crates/vb_ipc/src/codec.rs`
- `encode_payload(payload, max)` → `Result<BoundedPayload, IpcError>` (pub fn, line 11)
- `decode_payload(payload)` → `Result<IpcPayload, IpcError>` (pub fn, line 20)

## Duplicate Definitions in `lib.rs` (lines 641–960)

The following are **verbatim duplicates** that must be removed from `lib.rs` and replaced with re-exports:

| lib.rs lines | Type/Function | Canonical module |
|---|---|---|
| 641–649 | `encode_payload` | `codec.rs` |
| 650–652 | `decode_payload` | `codec.rs` |
| 654–668 | `QueueCapacity` struct + impl | `bounded.rs` |
| 670–690 | `MaxPayloadBytes` struct + impl | `bounded.rs` |
| 693–714 | `BoundedPayload` struct + impl | `bounded.rs` |
| 716–756 | `IngressFrame` struct + impl | `ingress.rs` |
| 758–798 | `MemoryIngress` struct + impl | `ingress.rs` |
| 800–867 | `IpcError` enum | `error.rs` |
| 869–946 | `IpcError` impl block | `error.rs` |
| 948–953 | `u32_to_usize` fn | `error.rs` (pub(crate)) |
| 955–960 | `map_try_send` fn | INGRESS.HELPER — this fn is not in ingress.rs; `ingress.rs` inlines the match directly |

**Note**: `map_try_send` (lib.rs lines 955–960) maps `TrySendError<IngressFrame>` to `IpcError`. In `ingress.rs` (line 71–74), the same mapping is done inline with a `match`. So `map_try_send` in lib.rs is unused after the dedupe and can be deleted.

## Private Module Declarations (lib.rs line 15-17)

Current:
```rust
pub mod client;
pub mod frame;
pub mod server;
```

`bounded`, `ingress`, and `error` modules are **NOT declared** in lib.rs at all (they are file-modules, not path-modules). This means they are currently inaccessible to external callers. **After the dedupe**, they must be declared as public so re-exports are possible.

## Public API Surface (what downstream crates import)

Downstream crates use `vb_ipc::` namespace directly. After the facade conversion, these must still resolve:
- `vb_ipc::MemoryIngress`
- `vb_ipc::IngressFrame`
- `vb_ipc::QueueCapacity`
- `vb_ipc::MaxPayloadBytes`
- `vb_ipc::BoundedPayload`
- `vb_ipc::IpcError`
- `vb_ipc::IpcCommand`
- `vb_ipc::IpcPayload`
- `vb_ipc::IpcFrameHeader`
- `vb_ipc::IpcFrame`
- `vb_ipc::encode_payload`
- `vb_ipc::decode_payload`
- `vb_ipc::encode_frame`
- `vb_ipc::decode_frame`
- `vb_ipc::decode_frame_header`
- `vb_ipc::MaxPayloadBytes::DEFAULT`
- `vb_ipc::IpcError::Full`, `::Disconnected`, `::PayloadTooLarge {...}`, etc.
- `vb_ipc::IpcError::diagnostic_code()`
- `vb_ipc::IpcError::runtime_code()`

## Downstream Crates Using vb_ipc Public API

1. `crates/velvet_ballastics/src/main.rs` — uses `vb_ipc::MaxPayloadBytes::DEFAULT`
2. `crates/velvet_ballastics/tests/cross_crate_adversarial.rs` — uses `vb_ipc::MemoryIngress`, `IngressFrame`, `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`
3. `crates/velvet_ballastics/tests/cli_integration.rs` — uses `vb_ipc::MaxPayloadBytes`
4. `crates/workspace_tests/benches/velvet_ballastics.rs` — uses `vb_ipc::MaxPayloadBytes::DEFAULT`, `MemoryIngress::bounded`, `QueueCapacity`, `IngressFrame`
5. `benches/velvet_ballastics.rs` — same as above

## Test Coverage

- `crates/vb_ipc/src/tests.rs` (internal `#[cfg(test)]` of lib.rs) — imports from `crate::` (lines 6–9), covering `MemoryIngress`, `IngressFrame`, `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`, `encode/decode_payload`
- `crates/vb_ipc/src/client/tests.rs`
- `crates/vb_ipc/src/server/impl_tests.rs`
- `crates/vb_ipc/src/frame/tests.rs`

The tests in `tests.rs` are the most relevant to the dedupe. They must be updated to import from split modules after lib.rs becomes a facade.

## Risk Tags

- **public_api**: The refactor must preserve all public API symbols. No symbol removal without deprecation.
- **migration**: Downstream crates (`velvet_ballastics`, `workspace_tests`) import vb_ipc symbols; they must not need changes.
- **no_unsafe**: No `unsafe` involved; all files are `#![forbid(unsafe_code)]`.
- **no_concurrency**: No new concurrency patterns; `MemoryIngress` already uses `crossbeam_channel`.
- **no_temporal**: No temporal behavior changes.
- **no_persistence**: No persistence behavior changes.

## Required Verifier Modes

- `verify-standard` (moon ci): sufficient — refactor is behavior-preserving
- No proof obligations required (no formal specs exist for this refactor)
- No TLA+/Verus/Kani/Loom/Miri required

## Implementation Plan (for State 10 reference)

1. In `lib.rs`, add `pub mod bounded; pub mod ingress; pub mod error;` module declarations
2. Add re-exports: `pub use bounded::{QueueCapacity, MaxPayloadBytes, BoundedPayload}; pub use ingress::{IngressFrame, MemoryIngress}; pub use error::IpcError;`
3. Also re-export `codec::{encode_payload, decode_payload}` from `codec.rs` (already pub)
4. Delete lines 641–960 from `lib.rs` (all duplicate definitions)
5. Delete `map_try_send` helper (line 955–960) — unused (ingress.rs inlines its logic)
6. Update `tests.rs` imports to use split module paths (from `crate::bounded::X`, `crate::ingress::X`, `crate::error::IpcError`, `crate::codec::X`)
7. Verify `cargo build -p vb_ipc` compiles without errors
8. Verify `cargo test -p vb_ipc` passes
9. Verify downstream crates compile (`cargo build -p velvet_ballastics`)

## Open Questions

- `map_try_send` in lib.rs: confirmed unused after dedupe (ingress.rs inlines the match). Delete.
- `u32_to_usize` in lib.rs: duplicate of `error.rs` `pub(crate)` fn. Delete from lib.rs; callers should use `error::u32_to_usize` or the re-exported version.
- Should `lib.rs` keep `mod tests;` at line 963? YES — internal test module of lib.rs.
- Should `lib.rs` retain `#[cfg(test)] mod tests { ... }`? There is none in lib.rs (it's `mod tests;` at line 963), the actual tests are in `tests.rs`.

## Excluded from Scope

- `crates/vb_ipc/src/frame.rs` — not duplicated (has `IpcFrame` which is different from `IngressFrame`)
- `crates/vb_ipc/src/frame_types.rs` — not duplicated (has `IpcFrameHeader`)
- `crates/vb_ipc/src/payloads.rs` — not duplicated (has `IpcPayload`, `SubmitRunPayload`)
- `crates/vb_ipc/src/commands.rs` — not duplicated (has `IpcCommand`)
- `crates/vb_ipc/src/codec.rs` — canonical source, not a duplicate
- Server, client, action_output, ids, metrics modules — out of scope
