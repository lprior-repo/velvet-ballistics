// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for ipc_runtime_transitions Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the
// `ipc_runtime_transitions.rs` Verus spec. It includes the in-tree
// production mirror at
// `verification/verus/production_inner/ipc_runtime_transitions_production.rs`
// via `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_ipc_runtime_transitions.rs"]`; this file
//     uses
//     `#[path = "production_inner/ipc_runtime_transitions_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/ipc_runtime_transitions_production.rs` mirror
//     and the spec proofs that depend on it.
//
// The mirror at
// `production_inner/ipc_runtime_transitions_production.rs` is a
// hand-written structural copy of the production state-machine code
// referenced as REFINE-IPC-003..005 in
// `crates/vb_runtime/src/ipc_refinement.rs`,
// `crates/vb_runtime/src/shard/types.rs`,
// `crates/vb_runtime/src/shard/timer_wheel.rs`, and
// `crates/vb_core/src/policy.rs`. The substitutions relative to
// direct production `#[path]` inclusion are documented in the
// mirror's header (in summary: the production sources depend on
// `vb_core`, `crate::admission`, `crate::shard`, and serde derives
// that cannot be resolved in a single-file Verus unit under the "no
// installs / no production changes" constraints).
//
// ============================================================================
// BINDING LEDGER (mirrors production_inner/ipc_runtime_transitions_production.rs)
// ============================================================================
//   - `RuntimeEvent` (9-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:818-841
//   - `RuntimeEvent::is_terminal()`                   <- crates/vb_runtime/src/shard/types.rs:843-851
//   - `RuntimeEvent::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:853-861
//   - `RuntimeState` (5-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:794-808
//   - `RuntimeState::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:810-816
//   - `ShardStatus` (11-field struct)                 <- crates/vb_runtime/src/shard/types.rs:718-743
//   - `ShardHealth` (2-variant enum)                  <- crates/vb_runtime/src/shard/types.rs:745-753
//   - `MAX_COMMAND_QUEUE_CAPACITY = 65_536`           <- crates/vb_runtime/src/shard/types.rs:572-573
//   - `ShardCommandQueue` (capacity/depth struct)     <- crates/vb_runtime/src/shard/types.rs:550-639
//   - `PendingTimerKind` (2-variant enum)             <- crates/vb_runtime/src/shard/types.rs:31-34
//   - `TimerEntry` (4-field struct)                   <- crates/vb_runtime/src/shard/timer_wheel.rs:20-30
//   - `TimerWheel`                                    <- crates/vb_runtime/src/shard/timer_wheel.rs:41-46
//   - `TimerWheel::new`                               <- crates/vb_runtime/src/shard/timer_wheel.rs:51-56
//   - `TimerWheel::cancel`                            <- crates/vb_runtime/src/shard/timer_wheel.rs:93-104
//   - `TimerWheel::fire_expired`                      <- crates/vb_runtime/src/shard/timer_wheel.rs:109-128
//   - `TimerWheel::len`                               <- crates/vb_runtime/src/shard/timer_wheel.rs:144-146
//   - `TimerWheel::get_kind`                          <- crates/vb_runtime/src/shard/timer_wheel.rs:150-152
//   - `RuntimePolicy` (4-variant enum)                <- crates/vb_core/src/policy.rs:7+
//   - `RunAdmission` (6-field struct)                 <- crates/vb_runtime/src/admission/parts/chunk_002_records.rs:3-16
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_runtime_transitions.rs`) state the
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
// `production_inner/ipc_runtime_transitions_production.rs` (NOT the
// actual production source). The mirror is a hand-written structural
// copy of the state-machine code referenced as REFINE-IPC-003..005,
// with documented substitutions (extern-crate imports stripped,
// method bodies replaced by `#[verifier::external]` wrappers). Any
// drift in field NAME, discriminant shape, or method signature
// breaks the verification build.
#[path = "production_inner/ipc_runtime_transitions_production.rs"]
pub mod production_inner;

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
