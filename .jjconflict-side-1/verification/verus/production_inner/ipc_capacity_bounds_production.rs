// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for ipc_capacity_bounds Verus spec
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `ipc_capacity_bounds.rs` Verus spec. It is a hand-written structural mirror
// of the production capacity-bound surface in
// `crates/vb_ipc/src/bounded.rs` and `crates/vb_ipc/src/ingress.rs`,
// plus a subset mirror of `IpcError` from `crates/vb_ipc/src/error.rs`.
//
// The substitutions relative to direct `#[path]` inclusion of the
// production source are documented in the companion extern file
// (`verification/verus/extern_ipc_capacity_bounds.rs`) header. In
// summary, the production sources depend on `bytes::Bytes` and
// `crossbeam_channel::bounded` extern crates that cannot be resolved
// in a single-file Verus unit under the "no installs / no production
// changes" constraints. The mirror preserves the public field names
// and discriminant shape so spec reasoning matches production.
//
// DRIFT POLICY: This file MUST be regenerated from the production
// sources whenever production changes. The mirror is annotated at the
// top of every section with the originating production line range so
// regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_ipc_capacity_bounds.rs`) via `#[path]`.
// Each production method body is marked `#[verifier::external]` so the
// body is opaque to Verus while the signature participates in the
// `assume_specification` binding in the companion spec file
// `ipc_capacity_bounds.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `MirrorIpcError` (3-variant subset enum)        <- crates/vb_ipc/src/error.rs:10-75
//     - `Full`                                       <- crates/vb_ipc/src/error.rs:13
//     - `Disconnected`                               <- crates/vb_ipc/src/error.rs:16
//     - `PayloadTooLarge { actual, limit }`          <- crates/vb_ipc/src/error.rs:19-24
//   - `MirrorQueueCapacity` (struct)                 <- crates/vb_ipc/src/bounded.rs:12
//   - `MirrorQueueCapacity::new`                     <- crates/vb_ipc/src/bounded.rs:16-18
//   - `MirrorMaxPayloadBytes` (struct)               <- crates/vb_ipc/src/bounded.rs:28
//   - `MirrorMaxPayloadBytes::new`                   <- crates/vb_ipc/src/bounded.rs:38-40
//   - `MirrorBoundedPayload` (struct)                <- crates/vb_ipc/src/bounded.rs:49
//   - `MirrorBoundedPayload::new`                    <- crates/vb_ipc/src/bounded.rs:53-62
//   - `MirrorMemoryIngress` (struct)                 <- crates/vb_ipc/src/ingress.rs:68-71
//   - `MirrorMemoryIngress::bounded`                 <- crates/vb_ipc/src/ingress.rs:76-79
//   - `MirrorMemoryIngress::try_submit`              <- crates/vb_ipc/src/ingress.rs:90-92, 122-127
//   - `MirrorMemoryIngress::len`                     <- crates/vb_ipc/src/ingress.rs:105-107
//   - `MirrorMemoryIngress::is_empty`                <- crates/vb_ipc/src/ingress.rs:111-113
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_capacity_bounds.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// Mirror of production `IpcError` (subset)
// ============================================================================
//
// Mirror of `crates/vb_ipc/src/error.rs:10-75` `IpcError` enum,
// restricted to the 3 variants the capacity-bound spec references:
// `Full`, `Disconnected`, `PayloadTooLarge { actual, limit }`. Field
// names match production exactly. `Debug` derive is intentionally
// dropped (Verus does not support `core::fmt::Debug` derivation
// without `external_type_specification`); the spec does not exercise
// `Debug`.
#[derive(Clone, Copy)]
pub enum MirrorIpcError {
    /// Mirror of production `IpcError::Full` at error.rs:13.
    Full,
    /// Mirror of production `IpcError::Disconnected` at error.rs:16.
    Disconnected,
    /// Mirror of production `IpcError::PayloadTooLarge { actual, limit }`
    /// at error.rs:19-24.
    PayloadTooLarge {
        /// Mirror of production `actual: usize`.
        actual: usize,
        /// Mirror of production `limit: usize`.
        limit: usize,
    },
}

// ============================================================================
// Mirror of production `QueueCapacity`
// ============================================================================
//
// Mirror of production `QueueCapacity(NonZeroUsize)` at
// `crates/vb_ipc/src/bounded.rs:12`. The mirror preserves the
// public field shape (the inner capacity value) so spec reasoning
// matches production semantics, but tracks the capacity as `usize`
// (a flat integer) plus an explicit `value` field.
#[derive(Clone, Copy)]
pub struct MirrorQueueCapacity {
    /// Mirror of production `pub struct QueueCapacity(NonZeroUsize)`.
    pub value: usize,
}

impl MirrorQueueCapacity {
    /// Mirror of production `QueueCapacity::new(value: NonZeroUsize) -> Self`
    /// at `crates/vb_ipc/src/bounded.rs:16-18`.
    #[verifier::external]
    pub const fn new(value: usize) -> Self {
        Self { value }
    }
}

// ============================================================================
// Mirror of production `MaxPayloadBytes`
// ============================================================================
//
// Mirror of production `MaxPayloadBytes(NonZeroUsize)` at
// `crates/vb_ipc/src/bounded.rs:28`.
#[derive(Clone, Copy)]
pub struct MirrorMaxPayloadBytes {
    /// Mirror of production `pub struct MaxPayloadBytes(NonZeroUsize)`.
    pub value: usize,
}

impl MirrorMaxPayloadBytes {
    /// Mirror of production `MaxPayloadBytes::new(value: NonZeroUsize) -> Self`
    /// at `crates/vb_ipc/src/bounded.rs:38-40`.
    #[verifier::external]
    pub const fn new(value: usize) -> Self {
        Self { value }
    }
}

// ============================================================================
// Mirror of production `BoundedPayload`
// ============================================================================
//
// Mirror of production `BoundedPayload(Bytes)` at
/// `crates/vb_ipc/src/bounded.rs:49`. The mirror tracks the byte
/// length directly because the capacity-bound spec only reasons
/// about the size contract — the actual byte contents are not in
/// scope for the bound proof.
#[derive(Clone, Copy)]
pub struct MirrorBoundedPayload {
    /// Mirror of production `BoundedPayload(Bytes)` projected to
    /// `Bytes::len() -> usize`.
    pub bytes_len: usize,
}

impl MirrorBoundedPayload {
    /// Mirror of production `BoundedPayload::new(payload: Bytes, max:
    /// MaxPayloadBytes) -> Result<Self, IpcError>` at
    /// `crates/vb_ipc/src/bounded.rs:53-62`.
    #[verifier::external]
    pub fn new(payload_len: usize, max: MirrorMaxPayloadBytes) -> Result<Self, MirrorIpcError> {
        if payload_len > max.value {
            Err(MirrorIpcError::PayloadTooLarge { actual: payload_len, limit: max.value })
        } else {
            Ok(Self { bytes_len: payload_len })
        }
    }
}

// ============================================================================
// Mirror of production `MemoryIngress`
// ============================================================================
//
// Mirror of production `MemoryIngress` at
// `crates/vb_ipc/src/ingress.rs:68-71`. The mirror reduces the
// `crossbeam_channel` SPSC pair to a single `(capacity, len)` pair.
#[derive(Clone, Copy)]
pub struct MirrorMemoryIngress {
    /// Spec projection of production `crossbeam_channel::bounded(capacity)`
    /// capacity (production at ingress.rs:77).
    pub capacity: usize,
    /// Spec projection of production `MemoryIngress::len() -> usize`.
    pub len: usize,
}

impl MirrorMemoryIngress {
    /// Mirror of production `MemoryIngress::bounded(capacity: QueueCapacity) -> Self`
    /// at `crates/vb_ipc/src/ingress.rs:76-79`.
    #[verifier::external]
    pub fn bounded(capacity: MirrorQueueCapacity) -> Self {
        Self { capacity: capacity.value, len: 0 }
    }

    /// Mirror of production `MemoryIngress::try_submit` at
    /// `crates/vb_ipc/src/ingress.rs:90-92, 122-127`.
    #[verifier::external]
    pub fn try_submit(&mut self) -> Result<(), MirrorIpcError> {
        if self.len < self.capacity {
            self.len = self.len + 1;
            Ok(())
        } else {
            Err(MirrorIpcError::Full)
        }
    }

    /// Mirror of production `MemoryIngress::len` at
    /// `crates/vb_ipc/src/ingress.rs:105-107`.
    #[verifier::external]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Mirror of production `MemoryIngress::is_empty` at
    /// `crates/vb_ipc/src/ingress.rs:111-113`.
    #[verifier::external]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
