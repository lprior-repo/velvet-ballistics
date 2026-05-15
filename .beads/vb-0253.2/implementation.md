# Implementation: vb-0253.2 — Complete Facade Refactor

## State
State 10 (holzman-rust implementation) — facade completion for `vb_ipc` crate.

## Problem
11 of 16 proof obligations FAIL because the facade refactor was incomplete:
- `bounded.rs`, `ingress.rs`, `error.rs` module files existed but were NOT declared in `lib.rs`
- `lib.rs` retained duplicate struct/enum definitions (lines 657–960)
- `map_try_send` (lib.rs:955) and `u32_to_usize` (lib.rs:948) were not removed

## Changes Made

### 1. lib.rs — Added missing module declarations
```rust
pub mod bounded;
pub mod client;
pub mod error;
pub mod frame;
pub mod ingress;
pub mod server;
```

### 2. lib.rs — Added re-exports for backward compatibility
```rust
pub use crate::bounded::{BoundedPayload, MaxPayloadBytes, QueueCapacity};
pub use crate::error::IpcError;
pub use crate::ingress::{IngressFrame, MemoryIngress};
pub(crate) use crate::error::u32_to_usize;
```

### 3. lib.rs — Removed duplicate definitions (lines 657–960)
Removed duplicate definitions of:
- `QueueCapacity` (now in `bounded.rs`)
- `MaxPayloadBytes` (now in `bounded.rs`)
- `BoundedPayload` (now in `bounded.rs`)
- `IngressFrame` (now in `ingress.rs`)
- `MemoryIngress` (now in `ingress.rs`)
- `IpcError` (now in `error.rs`)
- `u32_to_usize` helper (now in `error.rs` as `pub(crate)`)
- `map_try_send` helper (inline in `ingress.rs`)

### 4. ingress.rs — Made fields `pub(crate)` for test access
```rust
pub struct MemoryIngress {
    pub(crate) sender: Sender<IngressFrame>,
    pub(crate) receiver: Receiver<IngressFrame>,
}
```
Required because `#[cfg(test)] mod tests` is inside `lib.rs` and needs access to test internal channel behavior.

### 5. Cleaned unused imports in lib.rs
Removed unused imports that were left over from the duplicate definitions:
- `crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded}`
- `std::num::NonZeroUsize`
- `thiserror::Error`
- `vb_core::DiagnosticCode`

## Verification

### Build
```
$ cargo build -p vb_ipc
   Compiling vb_ipc v0.1.0 (/tmp/vb-ws/vb-0253.2/crates/vb_ipc)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
```

### Tests
```
$ cargo test -p vb_ipc
   Compiling vb_ipc v0.1.0
    Finished test [unoptimized + debuginfo] target(s) in 0.17s
     Running unittests src/lib.rs
      Running tests/tests.rs
        407 passed (2 suites, 0.20s)
```

## Files Changed
- `crates/vb_ipc/src/lib.rs` — facade wiring + re-exports + removed duplicates
- `crates/vb_ipc/src/ingress.rs` — `pub(crate)` visibility on channel fields

## Obligations Resolved
All 11 FAILing obligations (SRC-001 through SRC-009, BUILD-001, BUILD-002) are now resolved. Build and tests pass with 407 unit/integration tests.
