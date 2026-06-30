// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for ipc_capacity_bounds Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `ipc_capacity_bounds.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/ipc_capacity_bounds_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_ipc_capacity_bounds.rs"]`; this file uses
//     `#[path = "production_inner/ipc_capacity_bounds_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/ipc_capacity_bounds_production.rs` mirror and
//     the spec proofs that depend on it.
//
// The mirror at
// `production_inner/ipc_capacity_bounds_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_ipc/src/bounded.rs`, `crates/vb_ipc/src/ingress.rs`, and
// the relevant subset of `crates/vb_ipc/src/error.rs`. The
// substitutions relative to direct production `#[path]` inclusion are
// documented in the mirror's header (in summary: the production
// sources depend on `bytes::Bytes` and `crossbeam_channel::bounded`
// extern crates that cannot be resolved in a single-file Verus unit
// under the "no installs / no production changes" constraints).
//
// ============================================================================
// BINDING LEDGER (mirrors production_inner/ipc_capacity_bounds_production.rs)
// ============================================================================
//   - `MirrorIpcError` (3-variant subset enum)        <- crates/vb_ipc/src/error.rs:10-75
//   - `MirrorQueueCapacity` (struct)                 <- crates/vb_ipc/src/bounded.rs:12
//   - `MirrorQueueCapacity::new`                     <- crates/vb_ipc/src/bounded.rs:16-18
//   - `MirrorMaxPayloadBytes` (struct)               <- crates/vb_ipc/src/bounded.rs:28
//   - `MirrorMaxPayloadBytes::new`                   <- crates/vb_ipc/src/bounded.rs:38-40
//   - `MirrorMaxPayloadBytes::DEFAULT` (=1_048_576)  <- crates/vb_ipc/src/bounded.rs:32-35
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
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_capacity_bounds.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/ipc_capacity_bounds_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of `crates/vb_ipc/src/bounded.rs` and
// `crates/vb_ipc/src/ingress.rs` with documented substitutions
// (bytes/crossbeam_channel extern-crate imports stripped, method
// bodies replaced by `#[verifier::external]` wrappers). Any drift in
// field NAME, discriminant shape, or method signature breaks the
// verification build.
#[path = "production_inner/ipc_capacity_bounds_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
